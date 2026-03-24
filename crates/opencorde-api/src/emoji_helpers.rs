//! # Emoji Validation & Parsing Helpers
//! Validation and multipart parsing utilities for emoji endpoints.

use axum::extract::Multipart;
use crate::error::ApiError;

pub const MAX_EMOJI_SIZE: u64 = 256 * 1024; // 256 KB
pub const VALID_EMOJI_CONTENT_TYPES: &[&str] = &["image/png", "image/gif", "image/webp"];

/// Extract emoji name and file from multipart form data.
///
/// Expects "name" text field and "file" binary field.
/// Returns (name, content_type, bytes).
pub async fn extract_emoji_from_multipart(
    mut multipart: Multipart,
) -> Result<(String, String, Vec<u8>), ApiError> {
    let mut name: Option<String> = None;
    let mut file_data: Option<(String, Vec<u8>)> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::error!(error = %e, "failed to read multipart field");
        ApiError::BadRequest("invalid multipart form data".into())
    })? {
        match field.name() {
            Some("name") => {
                name = Some(field.text().await.map_err(|e| {
                    tracing::error!(error = %e, "failed to read name field");
                    ApiError::BadRequest("failed to read emoji name".into())
                })?);
            }
            Some("file") => {
                let content_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();

                let bytes = field.bytes().await.map_err(|e| {
                    tracing::error!(error = %e, "failed to read file bytes");
                    ApiError::BadRequest("failed to read file contents".into())
                })?;

                file_data = Some((content_type, bytes.to_vec()));
            }
            _ => {}
        }
    }

    let name = name.ok_or_else(|| {
        tracing::warn!("multipart form missing 'name' field");
        ApiError::BadRequest("multipart form must contain a 'name' field".into())
    })?;

    let (content_type, bytes) = file_data.ok_or_else(|| {
        tracing::warn!("multipart form missing 'file' field");
        ApiError::BadRequest("multipart form must contain a 'file' field".into())
    })?;

    Ok((name, content_type, bytes))
}

/// Validate emoji name: 2-32 chars, lowercase letters, numbers, underscores.
pub fn is_valid_emoji_name(name: &str) -> bool {
    if name.len() < 2 || name.len() > 32 {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Get file extension from MIME type.
pub fn get_emoji_extension(content_type: &str) -> &'static str {
    match content_type {
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "png",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_emoji_names() {
        assert!(is_valid_emoji_name("happy"));
        assert!(is_valid_emoji_name("cool_face"));
        assert!(is_valid_emoji_name("emoji123"));
        assert!(is_valid_emoji_name("_underscore"));
        assert!(is_valid_emoji_name("ab"));
    }

    #[test]
    fn test_invalid_emoji_names() {
        assert!(!is_valid_emoji_name("a")); // Too short
        assert!(!is_valid_emoji_name("VeryLongEmojiNameThatExceeds32Characters"));
        assert!(!is_valid_emoji_name("Happy Face")); // Space
        assert!(!is_valid_emoji_name("emoji-face")); // Hyphen
        assert!(!is_valid_emoji_name("EMOJI")); // Uppercase
        assert!(!is_valid_emoji_name("")); // Empty
    }

    #[test]
    fn test_get_emoji_extension() {
        assert_eq!(get_emoji_extension("image/png"), "png");
        assert_eq!(get_emoji_extension("image/gif"), "gif");
        assert_eq!(get_emoji_extension("image/webp"), "webp");
        assert_eq!(get_emoji_extension("unknown"), "png");
    }
}
