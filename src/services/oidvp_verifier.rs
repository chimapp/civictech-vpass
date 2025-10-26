use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(thiserror::Error, Debug)]
pub enum OidvpError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("OIDVP API error: {0}")]
    ApiError(String),

    #[error("Missing verifier configuration")]
    MissingConfig,

    #[error("Verification not ready yet")]
    NotReady,

    #[error("Verification expired")]
    Expired,

    #[error("Verification failed: {0}")]
    VerificationFailed(String),
}

/// Request to generate verification QR code
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeRequest {
    pub ref_code: String,
    pub transaction_id: String,
}

/// Response from QR code generation
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QrCodeResponse {
    pub transaction_id: String,
    pub qrcode_image: String, // base64 encoded PNG
    pub auth_uri: String,      // deep link
}

/// Request to check verification result
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultRequest {
    pub transaction_id: String,
}

/// Claim data from verifiable presentation
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClaimData {
    pub ename: String,
    pub cname: String,
    pub value: String,
}

/// Credential data from verifiable presentation
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialData {
    pub credential_type: String,
    pub claims: Vec<ClaimData>,
}

/// Response from result checking
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultResponse {
    pub verify_result: bool,
    pub result_description: String,
    pub transaction_id: String,
    pub data: Option<Vec<CredentialData>>,
}

/// Generate a verification QR code
///
/// Calls GET /api/oidvp/qrcode with ref and transactionId
#[tracing::instrument(skip(api_base_url, access_token))]
pub async fn request_verification_qr(
    api_base_url: &str,
    access_token: &str,
    ref_code: &str,
) -> Result<QrCodeResponse, OidvpError> {
    let client = Client::new();
    let transaction_id = Uuid::new_v4().to_string();

    tracing::debug!(
        transaction_id = %transaction_id,
        ref_code = %ref_code,
        "Requesting verification QR code"
    );

    let base = api_base_url.trim_end_matches('/');
    let url = format!(
        "{}/api/oidvp/qrcode?ref={}&transactionId={}",
        base, ref_code, transaction_id
    );

    let response = client
        .get(&url)
        .header("Access-Token", access_token)
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
            "OIDVP QR code request failed"
        );
        return Err(OidvpError::ApiError(format!(
            "Status {}: {}",
            status, error_text
        )));
    }

    let qr_response: QrCodeResponse = response.json().await.map_err(|e| {
        OidvpError::ApiError(format!("Failed to parse QR code response: {}", e))
    })?;

    tracing::info!(
        transaction_id = %qr_response.transaction_id,
        "Verification QR code generated successfully"
    );

    Ok(qr_response)
}

/// Poll for verification result
///
/// Calls POST /api/oidvp/result with transactionId
#[tracing::instrument(skip(api_base_url, access_token))]
pub async fn poll_verification_result(
    api_base_url: &str,
    access_token: &str,
    transaction_id: &str,
) -> Result<ResultResponse, OidvpError> {
    let client = Client::new();

    tracing::debug!(
        transaction_id = %transaction_id,
        "Polling verification result"
    );

    let base = api_base_url.trim_end_matches('/');
    let url = format!("{}/api/oidvp/result", base);

    let request_body = ResultRequest {
        transaction_id: transaction_id.to_string(),
    };

    let response = client
        .post(&url)
        .header("Access-Token", access_token)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    let status = response.status();

    // Handle 404 as "not ready yet"
    if status.as_u16() == 404 {
        return Err(OidvpError::NotReady);
    }

    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());

        // Check if it's a "not ready" error
        if error_text.contains("not ready") || error_text.contains("61010") || error_text.contains("verify result not found") {
            return Err(OidvpError::NotReady);
        }

        tracing::error!(
            status = %status,
            error = %error_text,
            "OIDVP result polling failed"
        );
        return Err(OidvpError::ApiError(format!(
            "Status {}: {}",
            status, error_text
        )));
    }

    let result_response: ResultResponse = response.json().await.map_err(|e| {
        OidvpError::ApiError(format!("Failed to parse result response: {}", e))
    })?;

    tracing::info!(
        transaction_id = %result_response.transaction_id,
        verify_result = result_response.verify_result,
        "Verification result retrieved"
    );

    Ok(result_response)
}

/// Extract member information from claims
///
/// Looks for specific claim fields like "name", "memberLevel", etc.
pub fn extract_member_info(credentials: &[CredentialData]) -> Option<serde_json::Value> {
    if credentials.is_empty() {
        return None;
    }

    let mut member_info = serde_json::Map::new();

    for credential in credentials {
        member_info.insert(
            "credentialType".to_string(),
            serde_json::Value::String(credential.credential_type.clone()),
        );

        for claim in &credential.claims {
            member_info.insert(
                claim.ename.clone(),
                serde_json::Value::String(claim.value.clone()),
            );
        }
    }

    Some(serde_json::Value::Object(member_info))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_member_info() {
        let credentials = vec![CredentialData {
            credential_type: "MembershipCard".to_string(),
            claims: vec![
                ClaimData {
                    ename: "name".to_string(),
                    cname: "姓名".to_string(),
                    value: "Test User".to_string(),
                },
                ClaimData {
                    ename: "memberLevel".to_string(),
                    cname: "會員等級".to_string(),
                    value: "Premium".to_string(),
                },
            ],
        }];

        let info = extract_member_info(&credentials).unwrap();
        assert_eq!(info["name"], "Test User");
        assert_eq!(info["memberLevel"], "Premium");
        assert_eq!(info["credentialType"], "MembershipCard");
    }
}
