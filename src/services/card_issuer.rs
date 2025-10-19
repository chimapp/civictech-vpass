use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    card::{CreateCardData, MembershipCard},
    issuer::CardIssuer,
    member::{CreateMemberData, Member},
};
use crate::services::{comment_verifier, qr_generator};

#[derive(thiserror::Error, Debug)]
pub enum CardIssuanceError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Comment verification failed: {0}")]
    CommentVerification(#[from] comment_verifier::CommentVerificationError),

    #[error("QR generation failed: {0}")]
    QrGeneration(#[from] qr_generator::QrGenerationError),

    #[error("Issuer not found")]
    IssuerNotFound,

    #[error("Duplicate card exists for this member")]
    DuplicateCard,

    #[error("Invalid comment ID format")]
    InvalidCommentId,
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
    pub qr_svg: String,
}

/// Issues a new membership card
///
/// This function orchestrates the entire card issuance flow:
/// 1. Validates the issuer exists
/// 2. Extracts comment ID from link
/// 3. Verifies the comment on YouTube
/// 4. Creates or updates member record
/// 5. Generates QR payload and signature
/// 6. Stores the card in the database
/// 7. Returns the card with QR code
#[tracing::instrument(skip(pool, signing_key, request), fields(issuer_id = %request.issuer_id))]
pub async fn issue_card(
    pool: &PgPool,
    signing_key: &[u8],
    request: IssueCardRequest,
) -> Result<IssueCardResult, CardIssuanceError> {
    tracing::info!("Starting card issuance process");

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

    // 2. Extract comment ID from link
    let comment_id = comment_verifier::extract_comment_id(&request.comment_link_or_id)
        .ok_or(CardIssuanceError::InvalidCommentId)?;

    tracing::debug!(comment_id = %comment_id, "Extracted comment ID");

    // 3. Verify the comment
    let verification_result = comment_verifier::verify_comment(
        &comment_id,
        &issuer.verification_video_id,
        &request.member_youtube_user_id,
        request.session_started_at,
        &request.access_token,
    )
    .await?;

    tracing::info!(
        comment_id = %verification_result.comment_id,
        author = %verification_result.author_display_name,
        published_at = %verification_result.published_at,
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

    // 5. Check for duplicate primary cards
    let existing_card = MembershipCard::find_primary_for_member(pool, issuer.id, member.id).await?;

    if existing_card.is_some() {
        return Err(CardIssuanceError::DuplicateCard);
    }

    // 6. Generate QR payload
    let now = Utc::now();
    let card_id = Uuid::new_v4();

    let qr_payload = qr_generator::MembershipCardPayload::new(
        card_id,
        qr_generator::IssuerInfo {
            id: issuer.id.to_string(),
            name: issuer.channel_name.clone(),
            channel_id: issuer.youtube_channel_id.clone(),
            handle: issuer.channel_handle.clone(),
        },
        qr_generator::MemberInfo {
            display_name: verification_result.author_display_name.clone(),
        },
        qr_generator::MembershipInfo {
            level: issuer.default_membership_label.clone(),
            confirmed_at: verification_result.published_at,
            issued_at: now,
        },
        qr_generator::VerificationInfo {
            video_id: verification_result.video_id.clone(),
            comment_id: verification_result.comment_id.clone(),
        },
    );

    // 7. Sign the payload
    let qr_signature = qr_payload.sign(signing_key);

    tracing::debug!("QR payload generated and signed");

    // 8. Create snapshot for auditing
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

    // 9. Store the card
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
            qr_payload: qr_payload.to_jsonb(),
            qr_signature: qr_signature.clone(),
        },
    )
    .await?;

    tracing::info!(card_id = %card.id, "Card created successfully");

    // 10. Generate QR code SVG
    let qr_svg = qr_generator::generate_qr_svg(&qr_payload, &qr_signature)?;

    Ok(IssueCardResult {
        card,
        member,
        qr_svg,
    })
}
