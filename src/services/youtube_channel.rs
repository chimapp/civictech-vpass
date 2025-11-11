use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Error, Debug)]
pub enum YouTubeChannelError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Invalid channel URL format")]
    InvalidUrl,

    #[error("Channel not found")]
    NotFound,

    #[error("YouTube API error: {0}")]
    ApiError(String),

    #[error("Rate limit exceeded after retries")]
    RateLimitExceeded,

    #[error("Service unavailable after retries")]
    ServiceUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub channel_id: String,
    pub channel_name: String,
    pub channel_handle: Option<String>,
}

/// Extract channel handle from various YouTube URL formats
/// Supports:
/// - https://www.youtube.com/@handle
/// - https://youtube.com/@handle
/// - @handle
pub fn extract_channel_handle(url: &str) -> Option<String> {
    let url = url.trim();

    // If it's just @handle
    if url.starts_with('@') {
        return Some(url.to_string());
    }

    // Parse URL and extract handle from path
    if let Some(idx) = url.find("youtube.com/@") {
        let after_domain = &url[idx + "youtube.com/".len()..];
        if let Some(handle_end) = after_domain.find(&['/', '?', '#'][..]) {
            return Some(after_domain[..handle_end].to_string());
        } else {
            return Some(after_domain.to_string());
        }
    }

    None
}

/// Retry logic for YouTube API calls with exponential backoff
/// Implements FR-009a: Max 3 attempts over 30 seconds with exponential backoff
async fn retry_youtube_api<F, Fut, T>(
    operation: F,
    operation_name: &str,
) -> Result<T, YouTubeChannelError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, YouTubeChannelError>>,
{
    const MAX_ATTEMPTS: u32 = 3;
    const INITIAL_DELAY_MS: u64 = 1000; // 1 second

    let mut attempt = 0;

    loop {
        attempt += 1;

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    tracing::info!(
                        operation = operation_name,
                        attempt = attempt,
                        "YouTube API request succeeded after retry"
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                let is_retryable = matches!(
                    &e,
                    YouTubeChannelError::ApiError(msg) if msg.starts_with("HTTP 429") || msg.starts_with("HTTP 503")
                );

                if !is_retryable || attempt >= MAX_ATTEMPTS {
                    tracing::error!(
                        error = ?e,
                        operation = operation_name,
                        attempt = attempt,
                        retryable = is_retryable,
                        "YouTube API request failed"
                    );

                    if attempt >= MAX_ATTEMPTS {
                        if matches!(&e, YouTubeChannelError::ApiError(msg) if msg.starts_with("HTTP 429")) {
                            return Err(YouTubeChannelError::RateLimitExceeded);
                        } else if matches!(&e, YouTubeChannelError::ApiError(msg) if msg.starts_with("HTTP 503")) {
                            return Err(YouTubeChannelError::ServiceUnavailable);
                        }
                    }

                    return Err(e);
                }

                // Calculate exponential backoff delay: 1s, 2s, 4s
                let delay_ms = INITIAL_DELAY_MS * (2_u64.pow(attempt - 1));

                tracing::warn!(
                    error = ?e,
                    operation = operation_name,
                    attempt = attempt,
                    next_attempt_in_ms = delay_ms,
                    "YouTube API request failed, retrying with exponential backoff"
                );

                sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }
}

/// Fetch channel information from YouTube Data API v3
/// This uses the channel handle to look up channel details
pub async fn fetch_channel_info(
    handle_or_url: &str,
    api_key: &str,
) -> Result<ChannelInfo, YouTubeChannelError> {
    let handle = extract_channel_handle(handle_or_url).ok_or(YouTubeChannelError::InvalidUrl)?;
    let api_key = api_key.to_string();
    let handle_for_closure = handle.clone();

    retry_youtube_api(
        || {
            let handle = handle_for_closure.clone();
            let api_key = api_key.clone();
            async move {
                // YouTube Data API v3 endpoint
                let url = format!(
                    "https://www.googleapis.com/youtube/v3/channels?part=snippet&forHandle={}&key={}",
                    handle.trim_start_matches('@'),
                    api_key
                );

                tracing::debug!(handle = %handle, "Fetching channel info from YouTube API");

                let client = reqwest::Client::new();
                let response = client
                    .get(&url)
                    .header("Accept", "application/json")
                    .send()
                    .await?;

                let status = response.status();

                if !status.is_success() {
                    let body = response.text().await.unwrap_or_default();
                    tracing::error!(status = %status, body = %body, "YouTube API request failed");
                    return Err(YouTubeChannelError::ApiError(format!(
                        "HTTP {}: {}",
                        status, body
                    )));
                }

                let api_response: YouTubeApiResponse = response.json().await?;

                let item = api_response
                    .items
                    .into_iter()
                    .next()
                    .ok_or(YouTubeChannelError::NotFound)?;

                Ok(ChannelInfo {
                    channel_id: item.id,
                    channel_name: item.snippet.title,
                    channel_handle: Some(handle),
                })
            }
        },
        "fetch_channel_info",
    )
    .await
}

#[derive(Debug, Deserialize)]
struct YouTubeApiResponse {
    items: Vec<YouTubeChannelItem>,
}

#[derive(Debug, Deserialize)]
struct YouTubeChannelItem {
    id: String,
    snippet: YouTubeChannelSnippet,
}

#[derive(Debug, Deserialize)]
struct YouTubeChannelSnippet {
    title: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_channel_handle() {
        assert_eq!(
            extract_channel_handle("https://www.youtube.com/@Dokibird"),
            Some("@Dokibird".to_string())
        );
        assert_eq!(
            extract_channel_handle("https://youtube.com/@Dokibird"),
            Some("@Dokibird".to_string())
        );
        assert_eq!(
            extract_channel_handle("@Dokibird"),
            Some("@Dokibird".to_string())
        );
        assert_eq!(
            extract_channel_handle("https://www.youtube.com/@Dokibird/videos"),
            Some("@Dokibird".to_string())
        );
        assert_eq!(
            extract_channel_handle("https://www.youtube.com/@Dokibird?feature=shared"),
            Some("@Dokibird".to_string())
        );
        assert_eq!(extract_channel_handle("not a valid url"), None);
    }
}
