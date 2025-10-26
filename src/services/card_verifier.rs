use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{
    card::{CardStatus, MembershipCard},
    issuer::CardIssuer,
};

#[derive(thiserror::Error, Debug)]
pub enum VerificationError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Invalid UUID format: {0}")]
    InvalidUuid(#[from] uuid::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrPayload {
    pub card_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationResult {
    Success {
        card: MembershipCard,
        issuer: CardIssuer,
    },
    CardNotFound {
        card_id: Uuid,
    },
    CardExpired {
        card: MembershipCard,
        issuer: CardIssuer,
    },
    CardRevoked {
        card: MembershipCard,
        issuer: CardIssuer,
    },
    CardSuspended {
        card: MembershipCard,
        issuer: CardIssuer,
    },
    InvalidPayload {
        error: String,
    },
}

impl VerificationResult {
    /// Returns the result type as a string for logging
    pub fn result_type(&self) -> &'static str {
        match self {
            VerificationResult::Success { .. } => "success",
            VerificationResult::CardNotFound { .. } => "card_not_found",
            VerificationResult::CardExpired { .. } => "card_expired",
            VerificationResult::CardRevoked { .. } => "card_revoked",
            VerificationResult::CardSuspended { .. } => "card_suspended",
            VerificationResult::InvalidPayload { .. } => "invalid_payload",
        }
    }

    /// Returns the card_id if available
    pub fn card_id(&self) -> Option<Uuid> {
        match self {
            VerificationResult::Success { card, .. } => Some(card.id),
            VerificationResult::CardNotFound { card_id } => Some(*card_id),
            VerificationResult::CardExpired { card, .. } => Some(card.id),
            VerificationResult::CardRevoked { card, .. } => Some(card.id),
            VerificationResult::CardSuspended { card, .. } => Some(card.id),
            VerificationResult::InvalidPayload { .. } => None,
        }
    }
}

/// Verifies a QR code payload
///
/// This function:
/// 1. Parses the QR payload (JSON with card_id)
/// 2. Looks up the card in the database
/// 3. Checks the card status (active, expired, revoked, suspended)
/// 4. Returns verification result
#[tracing::instrument(skip(pool))]
pub async fn verify_qr_payload(
    pool: &PgPool,
    qr_payload: &str,
) -> Result<VerificationResult, VerificationError> {
    tracing::debug!(payload_len = qr_payload.len(), "Parsing QR payload");

    // 1. Parse the payload
    let payload: QrPayload = match serde_json::from_str(qr_payload) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to parse QR payload");
            return Ok(VerificationResult::InvalidPayload {
                error: format!("Invalid JSON: {}", e),
            });
        }
    };

    tracing::info!(card_id = %payload.card_id, "Parsed QR payload");

    // 2. Look up the card
    let card = match MembershipCard::find_by_id(pool, payload.card_id).await? {
        Some(c) => c,
        None => {
            tracing::warn!(card_id = %payload.card_id, "Card not found");
            return Ok(VerificationResult::CardNotFound {
                card_id: payload.card_id,
            });
        }
    };

    tracing::debug!(
        card_id = %card.id,
        status = ?card.status,
        issuer_id = %card.issuer_id,
        "Found card"
    );

    // 3. Load the issuer
    let issuer = CardIssuer::find_by_id(pool, card.issuer_id)
        .await?
        .ok_or_else(|| {
            tracing::error!(issuer_id = %card.issuer_id, "Issuer not found for card");
            sqlx::Error::RowNotFound
        })?;

    // 4. Check card status
    let result = match card.status {
        CardStatus::Active => {
            // Check if expired
            if let Some(expires_at) = card.expires_at {
                if expires_at < chrono::Utc::now() {
                    tracing::info!(card_id = %card.id, expires_at = %expires_at, "Card expired");
                    VerificationResult::CardExpired { card, issuer }
                } else {
                    tracing::info!(card_id = %card.id, "Card verified successfully");
                    VerificationResult::Success { card, issuer }
                }
            } else {
                tracing::info!(card_id = %card.id, "Card verified successfully (no expiration)");
                VerificationResult::Success { card, issuer }
            }
        }
        CardStatus::Revoked => {
            tracing::info!(card_id = %card.id, "Card revoked");
            VerificationResult::CardRevoked { card, issuer }
        }
        CardStatus::Expired => {
            tracing::info!(card_id = %card.id, "Card expired");
            VerificationResult::CardExpired { card, issuer }
        }
        CardStatus::Suspended => {
            tracing::info!(card_id = %card.id, "Card suspended");
            VerificationResult::CardSuspended { card, issuer }
        }
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_payload_parsing() {
        let card_id = Uuid::new_v4();
        let payload = format!(r#"{{"card_id":"{}"}}"#, card_id);
        let parsed: QrPayload = serde_json::from_str(&payload).unwrap();
        assert_eq!(parsed.card_id, card_id);
    }

    #[test]
    fn test_invalid_payload() {
        let payload = r#"{"invalid":"data"}"#;
        let result: Result<QrPayload, _> = serde_json::from_str(payload);
        assert!(result.is_err());
    }
}
