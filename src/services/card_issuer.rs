use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    card::{CreateCardData, MembershipCard},
    issuer::CardIssuer,
    member::{CreateMemberData, Member},
};
use crate::services::comment_verifier;

#[derive(thiserror::Error, Debug)]
pub enum CardIssuanceError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Comment verification failed: {0}")]
    CommentVerification(#[from] comment_verifier::CommentVerificationError),

    #[error("Wallet QR generation failed: {0}")]
    WalletQrGeneration(#[from] crate::services::wallet_qr::WalletQrError),

    #[error("Taiwan Digital Wallet service unavailable. Please try again later.")]
    WalletServiceUnavailable,

    #[error("Issuer not found")]
    IssuerNotFound,

    #[error("Active card already exists. {0}")]
    DuplicateCard(String),

    #[error("Invalid comment ID format")]
    InvalidCommentId,

    #[error("Issuer not configured for Taiwan Digital Wallet (missing vc_uid)")]
    MissingVcUid,

    #[error("Issuer API not configured")]
    IssuerApiNotConfigured,
}

/// Request to issue a new membership card
pub struct IssueCardRequest {
    pub issuer_id: Uuid,
    pub member_youtube_user_id: String,
    pub member_display_name: String,
    pub member_avatar_url: Option<String>,
    pub comment_link_or_id: String,
    pub session_started_at: DateTime<Utc>,
    pub access_token: String,
}

/// Result of card issuance
pub struct IssueCardResult {
    pub card: MembershipCard,
    pub member: Member,
}

/// Issues a new membership card
///
/// This function orchestrates the entire card issuance flow:
/// 1. Validates the issuer exists
/// 2. Extracts comment ID from link
/// 3. Verifies the comment on YouTube
/// 4. Creates or updates member record
/// 5. Stores the card in the database
/// 6. Generates Taiwan Digital Wallet QR code
/// 7. Returns the card with QR code
#[tracing::instrument(skip(pool, issuer_api_config, request), fields(issuer_id = %request.issuer_id))]
pub async fn issue_card(
    pool: &PgPool,
    issuer_api_config: Option<(&str, &str)>, // (api_base_url, access_token)
    request: IssueCardRequest,
) -> Result<IssueCardResult, CardIssuanceError> {
    use std::time::Instant;
    let start_time = Instant::now();

    tracing::info!("Starting card issuance process");

    // 0. Early wallet API health check (FR-008a: fail fast if wallet unavailable)
    let (api_base_url, access_token) =
        issuer_api_config.ok_or(CardIssuanceError::IssuerApiNotConfigured)?;

    crate::services::wallet_qr::check_wallet_health(api_base_url, access_token)
        .await
        .map_err(|_| CardIssuanceError::WalletServiceUnavailable)?;

    tracing::debug!("Wallet API health check passed");

    // 1. Load and validate issuer
    let issuer = CardIssuer::find_by_id(pool, request.issuer_id)
        .await?
        .ok_or(CardIssuanceError::IssuerNotFound)?;

    if !issuer.is_active {
        return Err(CardIssuanceError::IssuerNotFound);
    }

    tracing::debug!(
        channel_name = %issuer.channel_name,
        verification_video = %issuer.verification_video_id,
        "Loaded issuer"
    );

    // 2. Extract comment ID and video ID from link
    let (comment_id, url_video_id) =
        comment_verifier::extract_comment_and_video_id(&request.comment_link_or_id)
            .ok_or(CardIssuanceError::InvalidCommentId)?;

    // If the URL contains a video ID, verify it matches the issuer's verification video
    if let Some(ref vid) = url_video_id {
        if vid != &issuer.verification_video_id {
            tracing::warn!(
                url_video_id = %vid,
                expected_video_id = %issuer.verification_video_id,
                "Video ID from URL doesn't match issuer's verification video"
            );
            return Err(CardIssuanceError::InvalidCommentId);
        }
    }

    tracing::debug!(
        comment_id = %comment_id,
        video_id = ?url_video_id,
        "Extracted comment and video ID from URL"
    );

    // 3. Verify the comment (no age restriction per FR-003)
    let youtube_start = Instant::now();
    let verification_result = comment_verifier::verify_comment(
        &comment_id,
        &issuer.verification_video_id,
        &request.member_youtube_user_id,
        &request.access_token,
    )
    .await?;
    let youtube_duration = youtube_start.elapsed();

    tracing::info!(
        comment_id = %verification_result.comment_id,
        author = %verification_result.author_display_name,
        published_at = %verification_result.published_at,
        youtube_api_duration_ms = youtube_duration.as_millis(),
        "Comment verified successfully"
    );

    // 4. Create or update member record
    let member = Member::find_or_create(
        pool,
        CreateMemberData {
            youtube_user_id: request.member_youtube_user_id.clone(),
            default_display_name: verification_result.author_display_name.clone(),
            avatar_url: request.member_avatar_url,
            locale: None,
        },
    )
    .await?;

    tracing::debug!(member_id = %member.id, "Member record created/updated");

    // 5. Check for duplicate active unexpired cards (FR-006 + FR-006a)
    let existing_cards = MembershipCard::find_active_unexpired_cards(pool, issuer.id, member.id).await?;

    if let Some(existing_card) = existing_cards.first() {
        let expires_info = existing_card
            .expires_at
            .map(|e| format!("Expires: {}", e.format("%Y-%m-%d")))
            .unwrap_or_else(|| "No expiration".to_string());

        return Err(CardIssuanceError::DuplicateCard(expires_info));
    }

    // 6. Create snapshot for auditing
    let now = Utc::now();
    let snapshot = serde_json::json!({
        "comment": {
            "id": verification_result.comment_id,
            "text": verification_result.text,
            "published_at": verification_result.published_at,
            "author_channel_id": verification_result.author_channel_id,
            "author_display_name": verification_result.author_display_name,
        },
        "verification": {
            "video_id": verification_result.video_id,
            "verified_at": now,
            "session_started_at": request.session_started_at,
        },
    });

    // 7. Validate issuer has vc_uid configured (api_base_url and access_token already extracted)
    let vc_uid = issuer
        .vc_uid
        .as_ref()
        .ok_or(CardIssuanceError::MissingVcUid)?;

    // 9. Generate Taiwan Digital Wallet QR code
    tracing::debug!("Generating Taiwan Digital Wallet QR code");
    let wallet_start = Instant::now();

    // Sanitize display name: Taiwan Digital Wallet only allows Chinese, English, numbers, and underscore
    let sanitized_name = verification_result
        .author_display_name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || (*c >= '\u{4e00}' && *c <= '\u{9fff}'))
        .collect::<String>();

    let display_name = if sanitized_name.is_empty() {
        "Member".to_string()
    } else {
        sanitized_name
    };

    let fields = vec![crate::services::wallet_qr::WalletQrField {
        ename: "name".to_string(),
        content: display_name,
    }];

    let wallet_qr_response =
        crate::services::wallet_qr::generate_wallet_qr(api_base_url, access_token, vc_uid, fields)
            .await?;
    let wallet_duration = wallet_start.elapsed();

    tracing::info!(
        wallet_api_duration_ms = wallet_duration.as_millis(),
        "Wallet QR code generated successfully"
    );

    // 10. Store the card
    let card = MembershipCard::create(
        pool,
        CreateCardData {
            issuer_id: issuer.id,
            member_id: member.id,
            membership_level_label: issuer.default_membership_label.clone(),
            membership_confirmed_at: verification_result.published_at,
            verification_comment_id: verification_result.comment_id,
            verification_video_id: verification_result.video_id,
            snapshot_json: snapshot,
        },
    )
    .await?;

    tracing::info!(
        card_id = %card.id,
        expires_at = %card.expires_at.map(|e| e.to_rfc3339()).unwrap_or_else(|| "never".to_string()),
        "Card created successfully"
    );

    // 11. Store wallet QR data on the card
    MembershipCard::set_wallet_qr(
        pool,
        card.id,
        wallet_qr_response.transaction_id.clone(),
        wallet_qr_response.qr_code,
        Some(wallet_qr_response.deep_link),
    )
    .await?;

    tracing::info!(
        card_id = %card.id,
        transaction_id = %wallet_qr_response.transaction_id,
        "Wallet QR data stored on card"
    );

    // Reload card to get wallet fields
    let card = MembershipCard::find_by_id(pool, card.id)
        .await?
        .expect("Card should exist after creation");

    // NFR-001: Log performance metrics (5-second target)
    let total_duration = start_time.elapsed();
    let duration_secs = total_duration.as_secs_f64();

    if duration_secs > 5.0 {
        tracing::warn!(
            duration_secs = duration_secs,
            youtube_api_ms = youtube_duration.as_millis(),
            wallet_api_ms = wallet_duration.as_millis(),
            card_id = %card.id,
            "Card issuance exceeded 5-second target (NFR-001)"
        );
    } else {
        tracing::info!(
            duration_secs = duration_secs,
            youtube_api_ms = youtube_duration.as_millis(),
            wallet_api_ms = wallet_duration.as_millis(),
            card_id = %card.id,
            "Card issuance completed within target"
        );
    }

    Ok(IssueCardResult { card, member })
}
