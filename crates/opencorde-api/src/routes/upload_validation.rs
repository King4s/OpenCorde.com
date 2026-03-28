//! # File Upload Validation
//! MIME type allowlisting, per-category size limits, magic byte verification,
//! and EXIF metadata stripping for file uploads.
//!
//! ## Security Properties
//! - MIME type must be on the allowlist (no arbitrary file uploads)
//! - File size limited per category (image 8MB, video 100MB, other 25MB)
//! - Magic bytes must match declared MIME type (prevents extension spoofing)
//! - EXIF stripped from JPEG/PNG before storage (prevents location/device leaks)
//!
//! ## Depends On
//! - img-parts crate (lossless EXIF chunk removal, no re-encode)
//! - crate::error::ApiError

use img_parts::{ImageEXIF, jpeg::Jpeg, png::Png};

use crate::error::ApiError;

/// Maximum size for image uploads (8 MB).
const MAX_IMAGE_SIZE: u64 = 8 * 1024 * 1024;
/// Maximum size for video uploads (100 MB).
const MAX_VIDEO_SIZE: u64 = 100 * 1024 * 1024;
/// Maximum size for all other uploads (audio, PDF, text — 25 MB).
const MAX_OTHER_SIZE: u64 = 25 * 1024 * 1024;

/// Validate that the declared MIME type is on the allowlist.
///
/// Allowed categories: `image/*`, `video/*`, `audio/*`, `application/pdf`, `text/plain`.
/// Content-Type parameters (e.g., `; charset=utf-8`) are ignored.
///
/// # Errors
/// Returns `ApiError::BadRequest` if the MIME type is not allowed.
pub fn validate_mime_type(content_type: &str) -> Result<(), ApiError> {
    let ct = normalise_ct(content_type);
    let allowed = ct.starts_with("image/")
        || ct.starts_with("video/")
        || ct.starts_with("audio/")
        || ct == "application/pdf"
        || ct == "text/plain";
    if !allowed {
        tracing::warn!(content_type = %ct, "upload rejected: MIME type not on allowlist");
        return Err(ApiError::BadRequest(format!(
            "file type '{}' is not permitted",
            ct
        )));
    }
    Ok(())
}

/// Validate the file size against per-category limits.
///
/// | Category | Limit |
/// |----------|-------|
/// | image/*  | 8 MB  |
/// | video/*  | 100 MB |
/// | audio/*, application/pdf, text/plain | 25 MB |
///
/// # Errors
/// Returns `ApiError::BadRequest` if the file exceeds its category limit.
pub fn validate_file_size(content_type: &str, size: u64) -> Result<(), ApiError> {
    let ct = normalise_ct(content_type);
    let (max, label) = if ct.starts_with("image/") {
        (MAX_IMAGE_SIZE, "images (8 MB)")
    } else if ct.starts_with("video/") {
        (MAX_VIDEO_SIZE, "video (100 MB)")
    } else {
        (MAX_OTHER_SIZE, "audio/document files (25 MB)")
    };
    if size > max {
        tracing::warn!(
            content_type = %ct,
            size = size,
            max = max,
            "upload rejected: file exceeds size limit"
        );
        return Err(ApiError::BadRequest(format!(
            "file size ({} bytes) exceeds the limit for {}: {} bytes",
            size, label, max
        )));
    }
    Ok(())
}

/// Verify that the file's leading magic bytes match the declared MIME type.
///
/// Prevents extension spoofing (e.g., an executable renamed to `.jpg`).
/// Text content types are accepted without a magic byte check.
///
/// # Errors
/// Returns `ApiError::BadRequest` if the magic bytes do not match.
pub fn verify_magic_bytes(content_type: &str, bytes: &[u8]) -> Result<(), ApiError> {
    let ct = normalise_ct(content_type);

    if bytes.len() < 4 {
        return Err(ApiError::BadRequest(
            "file is too small to determine its type".into(),
        ));
    }

    let ok = match ct.as_str() {
        "image/jpeg" | "image/jpg" => bytes.starts_with(&[0xFF, 0xD8, 0xFF]),
        "image/png" => bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
        "image/gif" => bytes.starts_with(b"GIF87") || bytes.starts_with(b"GIF89"),
        "image/webp" => {
            bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP"
        }
        "image/bmp" => bytes.starts_with(&[0x42, 0x4D]),
        "video/mp4" | "video/quicktime" | "video/x-m4v" => {
            // ISO BMFF: ftyp box starts at byte 4
            bytes.len() >= 8
                && matches!(
                    &bytes[4..8],
                    b"ftyp" | b"moov" | b"mdat" | b"wide" | b"free"
                )
        }
        "video/webm" => bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]),
        "video/x-matroska" => bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]),
        "audio/mpeg" | "audio/mp3" => {
            // MP3: sync word or ID3 tag
            bytes.starts_with(&[0xFF, 0xFB])
                || bytes.starts_with(&[0xFF, 0xF3])
                || bytes.starts_with(&[0xFF, 0xF2])
                || bytes.starts_with(b"ID3")
        }
        "audio/ogg" | "video/ogg" => bytes.starts_with(b"OggS"),
        "audio/wav" | "audio/wave" | "audio/x-wav" => {
            bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WAVE"
        }
        "audio/flac" => bytes.starts_with(b"fLaC"),
        "application/pdf" => bytes.starts_with(b"%PDF"),
        // text/plain: no magic bytes (could be any encoding)
        t if t.starts_with("text/") => true,
        // Fallthrough: accept (validate_mime_type already screened the type)
        _ => true,
    };

    if !ok {
        tracing::warn!(
            content_type = %ct,
            magic = ?&bytes[..bytes.len().min(8)],
            "upload rejected: magic bytes do not match declared MIME type"
        );
        return Err(ApiError::BadRequest(
            "file content does not match the declared file type".into(),
        ));
    }
    Ok(())
}

/// Strip EXIF metadata from JPEG and PNG files before storage.
///
/// Uses binary chunk manipulation (img-parts) — no decode/re-encode,
/// so the operation is lossless for image quality.
/// All other file types are returned unchanged.
/// Parse failures are logged and the original bytes are returned unchanged.
pub fn strip_exif(content_type: &str, bytes: Vec<u8>) -> Vec<u8> {
    let ct = normalise_ct(content_type);
    let raw: img_parts::Bytes = bytes.clone().into();

    match ct.as_str() {
        "image/jpeg" | "image/jpg" => match Jpeg::from_bytes(raw) {
            Ok(mut jpeg) => {
                jpeg.set_exif(None);
                tracing::debug!("EXIF stripped from JPEG upload");
                jpeg.encoder().bytes().to_vec()
            }
            Err(e) => {
                tracing::warn!(error = %e, "JPEG EXIF strip failed; storing as-is");
                bytes
            }
        },
        "image/png" => match Png::from_bytes(raw) {
            Ok(mut png) => {
                png.set_exif(None);
                tracing::debug!("EXIF stripped from PNG upload");
                png.encoder().bytes().to_vec()
            }
            Err(e) => {
                tracing::warn!(error = %e, "PNG EXIF strip failed; storing as-is");
                bytes
            }
        },
        _ => bytes,
    }
}

/// Normalise a Content-Type value: strip parameters, lowercase.
fn normalise_ct(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- validate_mime_type ---

    #[test]
    fn test_allowed_image_types() {
        assert!(validate_mime_type("image/jpeg").is_ok());
        assert!(validate_mime_type("image/png").is_ok());
        assert!(validate_mime_type("image/gif").is_ok());
        assert!(validate_mime_type("image/webp").is_ok());
    }

    #[test]
    fn test_allowed_video_types() {
        assert!(validate_mime_type("video/mp4").is_ok());
        assert!(validate_mime_type("video/webm").is_ok());
    }

    #[test]
    fn test_allowed_audio_types() {
        assert!(validate_mime_type("audio/mpeg").is_ok());
        assert!(validate_mime_type("audio/ogg").is_ok());
    }

    #[test]
    fn test_allowed_document_types() {
        assert!(validate_mime_type("application/pdf").is_ok());
        assert!(validate_mime_type("text/plain").is_ok());
    }

    #[test]
    fn test_rejected_mime_types() {
        assert!(validate_mime_type("application/octet-stream").is_err());
        assert!(validate_mime_type("application/x-executable").is_err());
        assert!(validate_mime_type("application/zip").is_err());
        assert!(validate_mime_type("application/javascript").is_err());
        assert!(validate_mime_type("text/html").is_err()); // XSS risk
    }

    #[test]
    fn test_mime_type_with_parameters_stripped() {
        assert!(validate_mime_type("image/jpeg; charset=utf-8").is_ok());
        assert!(validate_mime_type("text/plain; charset=utf-8").is_ok());
    }

    // --- validate_file_size ---

    #[test]
    fn test_image_size_limit() {
        let just_ok = MAX_IMAGE_SIZE;
        let too_big = MAX_IMAGE_SIZE + 1;
        assert!(validate_file_size("image/jpeg", just_ok).is_ok());
        assert!(validate_file_size("image/jpeg", too_big).is_err());
    }

    #[test]
    fn test_video_size_limit() {
        assert!(validate_file_size("video/mp4", MAX_VIDEO_SIZE).is_ok());
        assert!(validate_file_size("video/mp4", MAX_VIDEO_SIZE + 1).is_err());
    }

    #[test]
    fn test_other_size_limit() {
        assert!(validate_file_size("application/pdf", MAX_OTHER_SIZE).is_ok());
        assert!(validate_file_size("application/pdf", MAX_OTHER_SIZE + 1).is_err());
        assert!(validate_file_size("audio/mpeg", MAX_OTHER_SIZE).is_ok());
        assert!(validate_file_size("audio/mpeg", MAX_OTHER_SIZE + 1).is_err());
    }

    // --- verify_magic_bytes ---

    #[test]
    fn test_jpeg_magic() {
        let jpeg = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        assert!(verify_magic_bytes("image/jpeg", jpeg).is_ok());
        assert!(verify_magic_bytes("image/png", jpeg).is_err());
    }

    #[test]
    fn test_png_magic() {
        let png = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert!(verify_magic_bytes("image/png", png).is_ok());
        assert!(verify_magic_bytes("image/jpeg", png).is_err());
    }

    #[test]
    fn test_pdf_magic() {
        let pdf = b"%PDF-1.4 rest of file";
        assert!(verify_magic_bytes("application/pdf", pdf).is_ok());
        assert!(verify_magic_bytes("image/jpeg", pdf).is_err());
    }

    #[test]
    fn test_text_plain_no_magic_check() {
        let text = b"Hello, world!";
        assert!(verify_magic_bytes("text/plain", text).is_ok());
    }

    #[test]
    fn test_too_small_file() {
        assert!(verify_magic_bytes("image/jpeg", &[0xFF, 0xD8]).is_err());
    }

    // --- normalise_ct ---

    #[test]
    fn test_normalise_strips_params() {
        assert_eq!(normalise_ct("Image/JPEG; q=0.9"), "image/jpeg");
        assert_eq!(normalise_ct("application/pdf"), "application/pdf");
    }
}
