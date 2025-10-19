use serde::{Deserialize, Serialize};
use thiserror::Error;

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

/// Fetch channel information from YouTube Data API v3
/// This uses the channel handle to look up channel details
pub async fn fetch_channel_info(
    handle_or_url: &str,
    api_key: &str,
) -> Result<ChannelInfo, YouTubeChannelError> {
    let handle = extract_channel_handle(handle_or_url).ok_or(YouTubeChannelError::InvalidUrl)?;

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

    if !response.status().is_success() {
        let status = response.status();
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
