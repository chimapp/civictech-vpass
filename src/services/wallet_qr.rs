use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum WalletQrError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Wallet API error: {0}")]
    ApiError(String),

    #[error("Missing VC UID")]
    MissingVcUid,
}

#[derive(Debug, Serialize)]
pub struct WalletQrField {
    pub ename: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WalletQrRequest {
    vc_uid: String,
    fields: Vec<WalletQrField>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalletQrResponse {
    pub transaction_id: String,
    pub qr_code: String,
    pub deep_link: String,
}

/// Generates a QR code data for Taiwan Digital Wallet
///
/// This function calls the Taiwan Digital Wallet API to generate QR code data
/// that can be scanned by the wallet app.
#[tracing::instrument(skip(api_url, access_token))]
pub async fn generate_wallet_qr(
    api_url: &str,
    access_token: &str,
    vc_uid: &str,
    fields: Vec<WalletQrField>,
) -> Result<WalletQrResponse, WalletQrError> {
    let client = Client::new();

    tracing::debug!(
        vc_uid = %vc_uid,
        field_count = fields.len(),
        "Generating wallet QR code"
    );

    let request_body = WalletQrRequest {
        vc_uid: vc_uid.to_string(),
        fields,
    };

    let response = client
        .post(api_url)
        .header("Access-Token", access_token)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        tracing::error!(
            status = %status,
            error = %error_text,
            "Wallet API request failed"
        );
        return Err(WalletQrError::ApiError(format!(
            "Status {}: {}",
            status, error_text
        )));
    }

    let wallet_response: WalletQrResponse = response
        .json()
        .await
        .map_err(|e| WalletQrError::ApiError(format!("Failed to parse response: {}", e)))?;

    tracing::info!(
        transaction_id = %wallet_response.transaction_id,
        "Wallet QR code generated successfully"
    );

    Ok(wallet_response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_qr_request_serialization() {
        let request = WalletQrRequest {
            vc_uid: "test_uid".to_string(),
            fields: vec![WalletQrField {
                ename: "name".to_string(),
                content: "Test Name".to_string(),
            }],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("vcUid"));
        assert!(json.contains("test_uid"));
        assert!(json.contains("name"));
    }
}
