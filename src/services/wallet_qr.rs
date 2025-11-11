use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
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

    #[error("Invalid JWT format: {0}")]
    InvalidJwt(String),

    #[error("Credential not ready yet")]
    CredentialNotReady,
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

/// Checks if the wallet API is available
/// Returns Ok(()) if the API is reachable, otherwise returns an error
#[tracing::instrument(skip(api_base_url, access_token))]
pub async fn check_wallet_health(
    api_base_url: &str,
    access_token: &str,
) -> Result<(), WalletQrError> {
    let client = Client::new();
    let base = api_base_url.trim_end_matches('/');

    // Simple health check: try to hit the base API endpoint
    let url = format!("{}/api/qrcode/data", base);

    // Use HEAD request if available, otherwise use a minimal POST
    let response = client
        .head(&url)
        .header("Access-Token", access_token)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await?;

    if response.status().is_server_error() {
        return Err(WalletQrError::ApiError(format!(
            "Wallet API unavailable: HTTP {}",
            response.status()
        )));
    }

    Ok(())
}

/// Generates a QR code data for Taiwan Digital Wallet
///
/// This function calls the Taiwan Digital Wallet API to generate QR code data
/// that can be scanned by the wallet app.
#[tracing::instrument(skip(api_base_url, access_token))]
pub async fn generate_wallet_qr(
    api_base_url: &str,
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

    let base = api_base_url.trim_end_matches('/');
    let url = format!("{}/api/qrcode/data", base);

    let response = client
        .post(&url)
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

#[derive(Debug, Deserialize)]
pub struct CredentialResponse {
    pub credential: String,
}

#[derive(Debug, Deserialize)]
struct WalletApiErrorBody {
    code: Option<String>,
    #[allow(dead_code)]
    message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct JwtClaims {
    jti: String,
    #[serde(flatten)]
    _other: serde_json::Value,
}

/// Polls the Taiwan Digital Wallet API to check if the credential is ready
///
/// Returns the credential JWT if ready, or CredentialNotReady error if not yet scanned
#[tracing::instrument(skip(api_base_url))]
pub async fn poll_credential_status(
    api_base_url: &str,
    access_token: Option<&str>,
    transaction_id: &str,
) -> Result<CredentialResponse, WalletQrError> {
    let client = Client::new();

    let base = api_base_url.trim_end_matches('/');
    let url = format!("{}/api/credential/nonce/{}", base, transaction_id);

    tracing::debug!(
        transaction_id = %transaction_id,
        url = %url,
        "Polling credential status"
    );

    let mut request = client.get(&url);

    if let Some(token) = access_token {
        request = request.header("Access-Token", token);
    }

    let response = request.send().await?;
    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "Failed to read response body".to_string());

    if !status.is_success() {
        let is_not_ready = serde_json::from_str::<WalletApiErrorBody>(&body)
            .ok()
            .and_then(|err| err.code)
            .map(|code| code == "61010")
            .unwrap_or(false);

        if status.as_u16() == 404 || is_not_ready {
            return Err(WalletQrError::CredentialNotReady);
        }

        tracing::error!(
            status = %status,
            error = %body,
            "Credential polling failed"
        );
        return Err(WalletQrError::ApiError(format!(
            "Status {}: {}",
            status, body
        )));
    }

    // Some issuer responses return JSON with code/message even on non-error HTTP statuses.
    if let Ok(error_body) = serde_json::from_str::<WalletApiErrorBody>(&body) {
        if matches!(error_body.code.as_deref(), Some("61010")) {
            return Err(WalletQrError::CredentialNotReady);
        }
    }

    let credential_response: CredentialResponse = serde_json::from_str(&body).map_err(|e| {
        WalletQrError::ApiError(format!("Failed to parse credential response: {}", e))
    })?;

    tracing::info!(
        transaction_id = %transaction_id,
        "Credential retrieved successfully"
    );

    Ok(credential_response)
}

/// Extracts the CID from the credential JWT token
///
/// The CID is extracted from the `jti` field in the JWT payload.
/// Example jti: "https://issuer-vc.wallet.gov.tw/api/credential/a16187e9-755e-48ca-a9c0-622f76fe1360"
/// The CID would be: "a16187e9-755e-48ca-a9c0-622f76fe1360"
#[tracing::instrument(skip(jwt_token))]
pub fn extract_cid_from_jwt(jwt_token: &str) -> Result<String, WalletQrError> {
    // JWT tokens can be decoded without verification for extracting claims
    // We use insecure decoding here because we only need to extract the jti field
    // and don't need to verify the signature

    // Decode the JWT token without verification to extract the jti field
    // JWT format: header.payload.signature
    let parts: Vec<&str> = jwt_token.split('.').collect();
    if parts.len() != 3 {
        return Err(WalletQrError::InvalidJwt(
            "JWT token does not have 3 parts".to_string(),
        ));
    }

    // Decode the payload (second part) from base64url
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| WalletQrError::InvalidJwt(format!("Failed to decode base64: {}", e)))?;

    // Parse JSON payload
    let claims: JwtClaims = serde_json::from_slice(&payload_bytes)
        .map_err(|e| WalletQrError::InvalidJwt(format!("Failed to parse JSON: {}", e)))?;

    let jti = claims.jti;

    tracing::debug!(jti = %jti, "Extracted jti from JWT");

    // Extract CID from jti URL
    // Format: "https://issuer-vc.wallet.gov.tw/api/credential/{CID}"
    let cid = jti
        .split('/')
        .next_back()
        .ok_or_else(|| WalletQrError::InvalidJwt("jti does not contain a valid URL".to_string()))?
        .to_string();

    if cid.is_empty() {
        return Err(WalletQrError::InvalidJwt(
            "CID extracted from jti is empty".to_string(),
        ));
    }

    tracing::info!(cid = %cid, "Successfully extracted CID from credential");

    Ok(cid)
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
