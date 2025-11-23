use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    card::{CreateCardData, MembershipCard},
    issuer::CardIssuer,
    member::{CreateMemberData, Member},
};
use crate::services::membership_checker;

#[derive(thiserror::Error, Debug)]
pub enum CardIssuanceError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Membership verification failed: {0}")]
    MembershipVerificationFailed(String),

    #[error("Membership check error: {0}")]
    MembershipCheck(#[from] membership_checker::MembershipCheckError),

    #[error("Wallet QR generation failed: {0}")]
    WalletQrGeneration(#[from] crate::services::wallet_qr::WalletQrError),

    #[error("Taiwan Digital Wallet service unavailable. Please try again later.")]
    WalletServiceUnavailable,

    #[error("Issuer not found")]
    IssuerNotFound,

    #[error("Active card already exists. {0}")]
    DuplicateCard(String),

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
/// Flow:
/// 1. Validates issuer exists and wallet API health
/// 2. Verifies membership by checking access to members-only video
/// 3. Creates or updates member record
/// 4. Stores the card in the database
/// 5. Generates Taiwan Digital Wallet QR code
/// 6. Returns the card with QR code
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

    // 2. Verify membership by checking access to the members-only video
    let membership_video_id = issuer
        .members_only_video_id
        .as_deref()
        .unwrap_or(&issuer.verification_video_id);

    let youtube_start = Instant::now();
    let has_access =
        membership_checker::check_video_access(&request.access_token, membership_video_id).await?;
    let youtube_duration = youtube_start.elapsed();

    if !has_access {
        tracing::warn!(
            video_id = %membership_video_id,
            "Membership check failed: user cannot access members-only video"
        );
        return Err(CardIssuanceError::MembershipVerificationFailed(
            "Unable to confirm active membership for this channel".to_string(),
        ));
    }

    let verified_at = Utc::now();

    // 4. Create or update member record
    let member = Member::find_or_create(
        pool,
        CreateMemberData {
            youtube_user_id: request.member_youtube_user_id.clone(),
            default_display_name: request.member_display_name.clone(),
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
        "verification": {
            "method": "video_access",
            "video_id": membership_video_id,
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
    let sanitized_name = request
        .member_display_name
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
            membership_confirmed_at: verified_at,
            verification_comment_id: format!("membership-access:{}", membership_video_id),
            verification_video_id: membership_video_id.to_string(),
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
