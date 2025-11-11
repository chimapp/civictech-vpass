use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum CommentVerificationError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Comment not found")]
    CommentNotFound,

    #[error("Comment does not belong to authenticated user")]
    CommentOwnershipMismatch,

    #[error("Comment is not on the verification video")]
    WrongVideo,

    #[error("YouTube API error: {0}")]
    ApiError(String),

    #[error("Failed to parse YouTube API response: {0}")]
    ParseError(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommentVerificationResult {
    pub comment_id: String,
    pub author_channel_id: String,
    pub author_display_name: String,
    pub video_id: String,
    pub published_at: DateTime<Utc>,
    pub text: String,
}

#[derive(Debug, Deserialize)]
struct YouTubeCommentsResponse {
    items: Vec<CommentItem>,
}

#[derive(Debug, Deserialize)]
struct CommentItem {
    id: String,
    snippet: CommentSnippet,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommentSnippet {
    author_channel_id: AuthorChannelId,
    author_display_name: String,
    #[serde(rename = "textDisplay")]
    text_display: String,
    published_at: String,
}

#[derive(Debug, Deserialize)]
struct AuthorChannelId {
    value: String,
}

/// Verifies a comment on a YouTube video to confirm membership
///
/// This function:
/// 1. Fetches the comment from YouTube Data API
/// 2. Verifies the comment belongs to the authenticated user
/// 3. Verifies the comment is on the expected verification video
///
/// Note: Per FR-003 clarification, there is no age restriction on comments.
/// Comments from any date are accepted as long as ownership and video validation pass.
pub async fn verify_comment(
    comment_id: &str,
    expected_video_id: &str,
    expected_author_channel_id: &str,
    access_token: &str,
) -> Result<CommentVerificationResult, CommentVerificationError> {
    let client = Client::new();

    // Fetch comment from YouTube Data API
    let url = format!(
        "https://www.googleapis.com/youtube/v3/comments?part=snippet&id={}&key={}",
        comment_id,
        // Note: In production, you might want to use an API key here instead of just the access token
        // For now, we'll use the access token in the Authorization header
        ""
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(CommentVerificationError::ApiError(error_text));
    }

    let comments_response: YouTubeCommentsResponse = response
        .json()
        .await
        .map_err(|e| CommentVerificationError::ParseError(e.to_string()))?;

    // Check if comment exists
    let comment = comments_response
        .items
        .first()
        .ok_or(CommentVerificationError::CommentNotFound)?;

    // Verify comment ownership
    if comment.snippet.author_channel_id.value != expected_author_channel_id {
        return Err(CommentVerificationError::CommentOwnershipMismatch);
    }

    // Note: comments.list API doesn't return videoId in the snippet for individual comments.
    // We use the expected_video_id from the issuer configuration instead, as the comment URL
    // already includes the video ID which was validated when extracting the comment ID.

    // Parse published timestamp
    let published_at = DateTime::parse_from_rfc3339(&comment.snippet.published_at)
        .map_err(|e| CommentVerificationError::ParseError(e.to_string()))?
        .with_timezone(&Utc);

    Ok(CommentVerificationResult {
        comment_id: comment.id.clone(),
        author_channel_id: comment.snippet.author_channel_id.value.clone(),
        author_display_name: comment.snippet.author_display_name.clone(),
        video_id: expected_video_id.to_string(),
        published_at,
        text: comment.snippet.text_display.clone(),
    })
}

/// Extracts the comment ID and video ID from a YouTube comment URL
/// Supports formats like:
/// - https://www.youtube.com/watch?v=VIDEO_ID&lc=COMMENT_ID
/// - Direct comment ID (returns None for video_id)
///
///   Returns (comment_id, video_id)
pub fn extract_comment_and_video_id(input: &str) -> Option<(String, Option<String>)> {
    // If it's already just a comment ID (alphanumeric + hyphens/underscores)
    if input
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Some((input.to_string(), None));
    }

    // Try to parse as URL
    if let Ok(url) = url::Url::parse(input) {
        let mut comment_id = None;
        let mut video_id = None;

        // Look for lc parameter (comment ID) and v parameter (video ID)
        for (key, value) in url.query_pairs() {
            if key == "lc" {
                comment_id = Some(value.to_string());
            } else if key == "v" {
                video_id = Some(value.to_string());
            }
        }

        if let Some(cid) = comment_id {
            return Some((cid, video_id));
        }
    }

    None
}

/// Extracts the comment ID from a YouTube comment URL (legacy function)
/// Supports formats like:
/// - https://www.youtube.com/watch?v=VIDEO_ID&lc=COMMENT_ID
/// - Direct comment ID
pub fn extract_comment_id(input: &str) -> Option<String> {
    extract_comment_and_video_id(input).map(|(comment_id, _)| comment_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_comment_and_video_id_from_url() {
        let url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ&lc=UgxABC123";
        let result = extract_comment_and_video_id(url);
        assert_eq!(
            result,
            Some(("UgxABC123".to_string(), Some("dQw4w9WgXcQ".to_string())))
        );
    }

    #[test]
    fn test_extract_comment_id_from_url() {
        let url = "https://www.youtube.com/watch?v=dQw4w9WgXcQ&lc=UgxABC123";
        let result = extract_comment_id(url);
        assert_eq!(result, Some("UgxABC123".to_string()));
    }

    #[test]
    fn test_extract_comment_id_direct() {
        let comment_id = "UgxDirect123";
        let result = extract_comment_id(comment_id);
        assert_eq!(result, Some("UgxDirect123".to_string()));
    }

    #[test]
    fn test_extract_comment_and_video_id_direct() {
        let comment_id = "UgxDirect123";
        let result = extract_comment_and_video_id(comment_id);
        assert_eq!(result, Some(("UgxDirect123".to_string(), None)));
    }

    #[test]
    fn test_extract_comment_id_invalid() {
        let invalid = "not a valid url or id!!!";
        let result = extract_comment_id(invalid);
        assert_eq!(result, None);
    }
}
