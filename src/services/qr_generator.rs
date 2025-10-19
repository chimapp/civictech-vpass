use chrono::{DateTime, Utc};
use qrcode::render::svg;
use qrcode::QrCode;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::services::signature;

#[derive(thiserror::Error, Debug)]
pub enum QrGenerationError {
    #[error("QR code generation failed: {0}")]
    QrCodeError(#[from] qrcode::types::QrError),

    #[error("JSON serialization failed: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Signature error: {0}")]
    SignatureError(#[from] signature::SignatureError),
}

/// Payload structure for 數位皮夾 (Digital Wallet) compatible QR codes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipCardPayload {
    /// Card ID
    pub card_id: String,

    /// Issuer information
    pub issuer: IssuerInfo,

    /// Member information
    pub member: MemberInfo,

    /// Membership details
    pub membership: MembershipInfo,

    /// Verification information
    pub verification: VerificationInfo,

    /// HMAC signature of the payload
    #[serde(skip_serializing)]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuerInfo {
    pub id: String,
    pub name: String,
    pub channel_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberInfo {
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MembershipInfo {
    pub level: String,
    pub confirmed_at: DateTime<Utc>,
    pub issued_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationInfo {
    pub video_id: String,
    pub comment_id: String,
}

impl MembershipCardPayload {
    /// Creates a new payload from card components
    pub fn new(
        card_id: Uuid,
        issuer_id: Uuid,
        issuer_name: String,
        issuer_channel_id: String,
        issuer_handle: Option<String>,
        member_display_name: String,
        membership_level: String,
        membership_confirmed_at: DateTime<Utc>,
        issued_at: DateTime<Utc>,
        verification_video_id: String,
        verification_comment_id: String,
    ) -> Self {
        Self {
            card_id: card_id.to_string(),
            issuer: IssuerInfo {
                id: issuer_id.to_string(),
                name: issuer_name,
                channel_id: issuer_channel_id,
                handle: issuer_handle,
            },
            member: MemberInfo {
                display_name: member_display_name,
            },
            membership: MembershipInfo {
                level: membership_level,
                confirmed_at: membership_confirmed_at,
                issued_at,
            },
            verification: VerificationInfo {
                video_id: verification_video_id,
                comment_id: verification_comment_id,
            },
            signature: None,
        }
    }

    /// Serializes the payload to JSON for signing
    fn to_signing_string(&self) -> Result<String, QrGenerationError> {
        Ok(serde_json::to_string(self)?)
    }

    /// Signs the payload and returns the signature
    pub fn sign(&self, signing_key: &[u8]) -> String {
        let payload_str = self.to_signing_string().unwrap_or_default();
        signature::sign(&payload_str, signing_key)
    }

    /// Converts to JSONB value for database storage
    pub fn to_jsonb(&self) -> JsonValue {
        serde_json::to_value(self).unwrap_or(JsonValue::Null)
    }
}

/// Generates a QR code SVG from a signed payload
pub fn generate_qr_svg(
    payload: &MembershipCardPayload,
    signature: &str,
) -> Result<String, QrGenerationError> {
    // Create the final payload with signature included
    let mut final_payload = payload.clone();
    final_payload.signature = Some(signature.to_string());

    // Serialize to JSON
    let json_str = serde_json::to_string(&final_payload)?;

    // Generate QR code
    let code = QrCode::new(json_str.as_bytes())?;

    // Render as SVG
    let svg = code.render::<svg::Color>().min_dimensions(200, 200).build();

    Ok(svg)
}

/// Generates a QR code PNG from a signed payload
pub fn generate_qr_png(
    payload: &MembershipCardPayload,
    signature: &str,
) -> Result<Vec<u8>, QrGenerationError> {
    use image::{ImageBuffer, Luma};

    // Create the final payload with signature included
    let mut final_payload = payload.clone();
    final_payload.signature = Some(signature.to_string());

    // Serialize to JSON
    let json_str = serde_json::to_string(&final_payload)?;

    // Generate QR code
    let code = QrCode::new(json_str.as_bytes())?;

    // Convert QR code to image buffer
    let module_size = 10u32; // Each module is 10x10 pixels
    let width = code.width() as u32;
    let img_size = width * module_size;

    let mut img = ImageBuffer::<Luma<u8>, Vec<u8>>::new(img_size, img_size);

    for (x, y, color) in img.enumerate_pixels_mut() {
        let module_x = x / module_size;
        let module_y = y / module_size;
        let module_color = code[(module_x as usize, module_y as usize)];
        let pixel_value = match module_color {
            qrcode::types::Color::Dark => Luma([0u8]),    // Black
            qrcode::types::Color::Light => Luma([255u8]), // White
        };
        *color = pixel_value;
    }

    // Encode as PNG
    let mut png_data = Vec::new();
    image::DynamicImage::ImageLuma8(img)
        .write_to(
            &mut std::io::Cursor::new(&mut png_data),
            image::ImageFormat::Png,
        )
        .map_err(|_| QrGenerationError::QrCodeError(qrcode::types::QrError::DataTooLong))?;

    Ok(png_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_creation() {
        let payload = MembershipCardPayload::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Test Channel".to_string(),
            "UC123456".to_string(),
            Some("@testchannel".to_string()),
            "Test Member".to_string(),
            "Channel Member".to_string(),
            Utc::now(),
            Utc::now(),
            "video123".to_string(),
            "comment123".to_string(),
        );

        assert_eq!(payload.issuer.name, "Test Channel");
        assert_eq!(payload.member.display_name, "Test Member");
    }

    #[test]
    fn test_payload_signing() {
        let payload = MembershipCardPayload::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Test Channel".to_string(),
            "UC123456".to_string(),
            None,
            "Test Member".to_string(),
            "Channel Member".to_string(),
            Utc::now(),
            Utc::now(),
            "video123".to_string(),
            "comment123".to_string(),
        );

        let key = b"test-signing-key";
        let signature = payload.sign(key);

        assert!(!signature.is_empty());
        assert!(signature.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_qr_svg_generation() {
        let payload = MembershipCardPayload::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            "Test Channel".to_string(),
            "UC123456".to_string(),
            None,
            "Test Member".to_string(),
            "Channel Member".to_string(),
            Utc::now(),
            Utc::now(),
            "video123".to_string(),
            "comment123".to_string(),
        );

        let key = b"test-signing-key";
        let signature = payload.sign(key);
        let svg = generate_qr_svg(&payload, &signature);

        assert!(svg.is_ok());
        let svg_str = svg.unwrap();
        assert!(svg_str.contains("<svg"));
        assert!(svg_str.contains("</svg>"));
    }
}
