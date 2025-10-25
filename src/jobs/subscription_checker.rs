use sqlx::PgPool;

use crate::models::{
    card::{CardStatus, MembershipCard},
    issuer::CardIssuer,
    oauth_session::OAuthSession,
};
use crate::services::{membership_checker, oauth::youtube};

const EXPIRATION_EXTENSION_DAYS: i64 = 30;
const FAILURE_THRESHOLD: i32 = 3;

#[derive(Debug)]
pub struct VerificationStats {
    pub total_checked: usize,
    pub still_members: usize,
    pub expired_memberships: usize,
    pub token_refresh_failures: usize,
    pub api_errors: usize,
}

/// Background job that verifies active membership cards
///
/// For each active card that hasn't been verified in 24 hours:
/// 1. Get member's OAuth session and refresh token if needed
/// 2. Check video access using members-only video ID
/// 3. If still a member: extend card expiration by 30 days
/// 4. If not a member: increment failure count, expire after 3 failures
pub async fn verify_membership_cards(pool: &PgPool, batch_size: i64) -> Result<VerificationStats, Box<dyn std::error::Error>> {
    let mut stats = VerificationStats {
        total_checked: 0,
        still_members: 0,
        expired_memberships: 0,
        token_refresh_failures: 0,
        api_errors: 0,
    };

    // Get cards that need verification
    let cards = MembershipCard::find_cards_needing_verification(pool, batch_size).await?;
    stats.total_checked = cards.len();

    tracing::info!(
        total_cards = stats.total_checked,
        "Starting membership verification job"
    );

    for card in cards {
        match verify_single_card(pool, &card).await {
            Ok(VerificationResult::StillMember) => {
                stats.still_members += 1;
            }
            Ok(VerificationResult::MembershipExpired) => {
                stats.expired_memberships += 1;
            }
            Err(VerificationError::TokenRefreshFailed) => {
                stats.token_refresh_failures += 1;
            }
            Err(VerificationError::ApiError(e)) => {
                tracing::error!(
                    card_id = %card.id,
                    error = %e,
                    "API error during verification"
                );
                stats.api_errors += 1;
            }
            Err(VerificationError::DatabaseError(e)) => {
                tracing::error!(
                    card_id = %card.id,
                    error = %e,
                    "Database error during verification"
                );
                stats.api_errors += 1;
            }
        }
    }

    tracing::info!(
        ?stats,
        "Membership verification job completed"
    );

    Ok(stats)
}

enum VerificationResult {
    StillMember,
    MembershipExpired,
}

enum VerificationError {
    TokenRefreshFailed,
    ApiError(String),
    DatabaseError(sqlx::Error),
}

async fn verify_single_card(
    pool: &PgPool,
    card: &MembershipCard,
) -> Result<VerificationResult, VerificationError> {
    // 1. Load issuer configuration
    let issuer = CardIssuer::find_by_id(pool, card.issuer_id)
        .await
        .map_err(VerificationError::DatabaseError)?
        .ok_or_else(|| VerificationError::ApiError("Issuer not found".to_string()))?;

    // 2. Load member's OAuth session
    let oauth_session = OAuthSession::find_by_member_id(pool, card.member_id)
        .await
        .map_err(VerificationError::DatabaseError)?
        .ok_or_else(|| VerificationError::ApiError("OAuth session not found".to_string()))?;

    // 3. Refresh token if expired
    let access_token = if oauth_session.is_expired() {
        tracing::info!(
            card_id = %card.id,
            member_id = %card.member_id,
            "Access token expired, refreshing"
        );

        let refresh_token = oauth_session
            .refresh_token
            .as_ref()
            .and_then(|t| String::from_utf8(t.clone()).ok())
            .ok_or(VerificationError::TokenRefreshFailed)?;

        // Get config from environment (in a real implementation, pass this in)
        let youtube_client_id = std::env::var("YOUTUBE_CLIENT_ID")
            .map_err(|_| VerificationError::TokenRefreshFailed)?;
        let youtube_client_secret = std::env::var("YOUTUBE_CLIENT_SECRET")
            .map_err(|_| VerificationError::TokenRefreshFailed)?;
        let base_url = std::env::var("BASE_URL")
            .map_err(|_| VerificationError::TokenRefreshFailed)?;

        let token_data = youtube::refresh_access_token(
            &refresh_token,
            &youtube_client_id,
            &secrecy::Secret::new(youtube_client_secret),
            &format!("{}/auth/youtube/callback", base_url),
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Token refresh failed");
            VerificationError::TokenRefreshFailed
        })?;

        // Update session with new tokens
        OAuthSession::update_tokens(
            pool,
            oauth_session.id,
            token_data.access_token.as_bytes().to_vec(),
            token_data.refresh_token.map(|t| t.as_bytes().to_vec()),
            token_data.expires_at,
        )
        .await
        .map_err(VerificationError::DatabaseError)?;

        token_data.access_token
    } else {
        String::from_utf8(oauth_session.access_token.clone())
            .map_err(|_| VerificationError::ApiError("Invalid token encoding".to_string()))?
    };

    // 4. Check membership access
    let video_id = match issuer.verification_method.as_str() {
        "video" => issuer
            .members_only_video_id
            .as_ref()
            .unwrap_or(&issuer.verification_video_id),
        _ => &issuer.verification_video_id,
    };

    let is_still_member = match issuer.verification_method.as_str() {
        "video" => membership_checker::check_video_access(&access_token, video_id)
            .await
            .map_err(|e| VerificationError::ApiError(e.to_string()))?,
        "comment" => membership_checker::check_comment_access(&access_token, video_id)
            .await
            .map_err(|e| VerificationError::ApiError(e.to_string()))?,
        _ => return Err(VerificationError::ApiError("Invalid verification method".to_string())),
    };

    // 5. Update card based on result
    if is_still_member {
        // Extend expiration and reset failures
        MembershipCard::extend_expiration(pool, card.id, EXPIRATION_EXTENSION_DAYS)
            .await
            .map_err(VerificationError::DatabaseError)?;

        tracing::info!(
            card_id = %card.id,
            member_id = %card.member_id,
            "Membership verified, card extended"
        );

        Ok(VerificationResult::StillMember)
    } else {
        // Increment failure count
        let failures = MembershipCard::increment_verification_failure(pool, card.id)
            .await
            .map_err(VerificationError::DatabaseError)?;

        tracing::warn!(
            card_id = %card.id,
            member_id = %card.member_id,
            failures = failures,
            "Membership verification failed"
        );

        // Mark as expired if threshold reached
        if failures >= FAILURE_THRESHOLD {
            MembershipCard::set_status(pool, card.id, CardStatus::Expired)
                .await
                .map_err(VerificationError::DatabaseError)?;

            tracing::info!(
                card_id = %card.id,
                member_id = %card.member_id,
                "Card marked as expired after {} failures",
                failures
            );

            Ok(VerificationResult::MembershipExpired)
        } else {
            Ok(VerificationResult::MembershipExpired)
        }
    }
}
