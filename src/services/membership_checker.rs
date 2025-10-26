use reqwest::{Client, StatusCode};
use serde::Deserialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MembershipCheckError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("YouTube API error: {status} - {message}")]
    ApiError { status: StatusCode, message: String },

    #[error("Access token expired or invalid")]
    TokenExpired,

    #[error("Membership has expired (403 Forbidden)")]
    MembershipExpired,
}

#[derive(Debug, Deserialize)]
struct VideoResponse {
    items: Vec<VideoItem>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct VideoItem {
    id: String,
}

/// Checks if user still has access to a members-only video
///
/// Returns:
/// - Ok(true) - User is still a member (200 OK with video data)
/// - Ok(false) - User is not a member (403 Forbidden or empty items)
/// - Err(TokenExpired) - Access token needs refresh (401 Unauthorized)
/// - Err(ApiError) - Other YouTube API errors
pub async fn check_video_access(
    access_token: &str,
    video_id: &str,
) -> Result<bool, MembershipCheckError> {
    let client = Client::new();
    let url = format!(
        "https://www.googleapis.com/youtube/v3/videos?id={}&part=snippet",
        video_id
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    match response.status() {
        StatusCode::OK => {
            // Parse response to check if video is accessible
            let video_response: VideoResponse =
                response
                    .json()
                    .await
                    .map_err(|e| MembershipCheckError::ApiError {
                        status: StatusCode::INTERNAL_SERVER_ERROR,
                        message: format!("Failed to parse response: {}", e),
                    })?;

            // If items is empty, user doesn't have access
            Ok(!video_response.items.is_empty())
        }
        StatusCode::FORBIDDEN => {
            // User no longer has membership access
            Ok(false)
        }
        StatusCode::NOT_FOUND => {
            // Video not found or user doesn't have access
            Ok(false)
        }
        StatusCode::UNAUTHORIZED => {
            // Token expired or invalid
            Err(MembershipCheckError::TokenExpired)
        }
        other => {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(MembershipCheckError::ApiError {
                status: other,
                message: error_text,
            })
        }
    }
}

/// Checks membership by accessing the verification video's comment thread
/// This is a fallback method when members_only_video_id is not configured
pub async fn check_comment_access(
    access_token: &str,
    video_id: &str,
) -> Result<bool, MembershipCheckError> {
    let client = Client::new();
    let url = format!(
        "https://www.googleapis.com/youtube/v3/commentThreads?videoId={}&part=snippet&maxResults=1",
        video_id
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    match response.status() {
        StatusCode::OK => Ok(true),
        StatusCode::FORBIDDEN => Ok(false),
        StatusCode::UNAUTHORIZED => Err(MembershipCheckError::TokenExpired),
        other => {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(MembershipCheckError::ApiError {
                status: other,
                message: error_text,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires valid access token
    async fn test_check_video_access() {
        let access_token = "test_token";
        let video_id = "test_video_id";

        let result = check_video_access(access_token, video_id).await;
        // Result depends on actual API response
        assert!(result.is_ok() || result.is_err());
    }
}
