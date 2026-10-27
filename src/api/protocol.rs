//! Pure protocol logic for the automation API: errors, authentication,
//! rate limiting, key generation, E.164 normalization and HMAC signing.
//!
//! This module MUST stay free of Qt/Tauri/Electron dependencies so it stays
//! trivially testable.

use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;

pub const LOCAL_API_VERSION: u32 = 1;
/// Default rate limit: 60 requests per rolling 60 seconds.
pub const RATE_LIMIT_MAX: usize = 60;
pub const RATE_LIMIT_WINDOW_SECS: u64 = 60;
/// Body cap for JSON request bodies (1 MB), mirrors the Python fork.
pub const MAX_BODY_BYTES: usize = 1_048_576;

/// Protocol error with a ready-to-serve HTTP status and opaque code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    pub status: u16,
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: 400,
            code: "bad_request",
            message: message.into(),
        }
    }

    pub fn unauthorized() -> Self {
        Self {
            status: 401,
            code: "unauthorized",
            message: "missing or invalid API key".to_string(),
        }
    }

    pub fn rate_limited() -> Self {
        Self {
            status: 429,
            code: "rate_limited",
            message: "too many requests, slow down".to_string(),
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            status: 503,
            code: "page_unavailable",
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: 500,
            code: "internal_error",
            message: message.into(),
        }
    }
}

#[derive(Serialize)]
pub struct ErrorPayload {
    pub ok: bool,
    pub error: &'static str,
    pub message: String,
}

pub fn error_payload(err: &ApiError) -> ErrorPayload {
    ErrorPayload {
        ok: false,
        error: err.code,
        message: err.message.clone(),
    }
}

/// Rolling-window rate limiter (single instance, synchronized).
pub struct RateLimiter {
    hits: std::sync::Mutex<std::collections::VecDeque<std::time::Instant>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            hits: std::sync::Mutex::new(std::collections::VecDeque::new()),
        }
    }

    /// Record a hit unless the rolling window is full.
    pub fn allow(&self) -> bool {
        let now = std::time::Instant::now();
        let window = std::time::Duration::from_secs(RATE_LIMIT_WINDOW_SECS);
        let mut hits = match self.hits.lock() {
            Ok(guard) => guard,
            Err(_) => return true, // fail open on poison; the server stays alive
        };
        while let Some(front) = hits.front() {
            if now.duration_since(*front) > window {
                hits.pop_front();
            } else {
                break;
            }
        }
        if hits.len() >= RATE_LIMIT_MAX {
            return false;
        }
        hits.push_back(now);
        true
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract the caller-provided API key from headers: `X-API-Key: ...` first,
/// then `Authorization: Bearer ...`.
pub fn extract_api_key(
    header_x_api_key: Option<&str>,
    header_authorization: Option<&str>,
) -> Option<String> {
    if let Some(raw) = header_x_api_key {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    if let Some(raw) = header_authorization {
        let rest = raw.trim();
        if let Some(token) = rest
            .strip_prefix("Bearer ")
            .or_else(|| rest.strip_prefix("bearer "))
        {
            let token = token.trim();
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }
    None
}

/// Generate a random API key with a recognizable prefix.
pub fn generate_api_key() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    format!("zpk_{}", hex::encode(bytes))
}

/// Normalize a phone identifier to digits-only E.164 form.
///
/// Returns digits without a leading `+`; caller adds one when building URLs.
pub fn normalize_e164(raw: &str) -> Result<String, ApiError> {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 5 || digits.len() > 32 {
        return Err(ApiError::bad_request(
            "phone must contain between 5 and 32 digits",
        ));
    }
    Ok(digits)
}

/// Validate a text message body.
pub fn validate_message(raw: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request("message must not be empty"));
    }
    if trimmed.chars().count() > 4096 {
        return Err(ApiError::bad_request("message is too long (max 4096)"));
    }
    Ok(trimmed.to_string())
}

/// HMAC-SHA256 signature in the `sha256=<hex>` webhook header format.
pub fn sign_payload(secret: &str, body: &[u8]) -> String {
    if secret.is_empty() {
        return String::new();
    }
    match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
        Ok(mut mac) => {
            mac.update(body);
            format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
        }
        Err(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_key_is_prefixed_and_hex() {
        let key = generate_api_key();
        assert!(key.starts_with("zpk_"));
        assert_eq!(key["zpk_".len()..].len(), 64);
        let other = generate_api_key();
        assert_ne!(key, other);
    }

    #[test]
    fn extract_prefers_x_api_key() {
        let key = extract_api_key(Some(" k1 "), Some("Bearer k2"));
        assert_eq!(key.as_deref(), Some("k1"));
        let bearer = extract_api_key(None, Some("Bearer k2"));
        assert_eq!(bearer.as_deref(), Some("k2"));
        assert!(extract_api_key(None, None).is_none());
        assert!(extract_api_key(None, Some("Basic k")).is_none());
    }

    #[test]
    fn limiter_closes_at_capacity() {
        let limiter = RateLimiter::new();
        for _ in 0..RATE_LIMIT_MAX {
            assert!(limiter.allow());
        }
        assert!(!limiter.allow());
    }

    #[test]
    fn normalize_e164_strips_noise() {
        assert_eq!(
            normalize_e164("+55 11 (99999) 0000").unwrap(),
            "5511999990000"
        );
        assert!(normalize_e164("").is_err());
        assert!(normalize_e164("+1").is_err());
    }

    #[test]
    fn message_validator_bounds() {
        assert!(validate_message("hola").is_ok());
        assert!(validate_message("   ").is_err());
        let long = "x".repeat(5000);
        assert!(validate_message(&long).is_err());
    }

    #[test]
    fn hmac_signature_format() {
        let signature = sign_payload("secret", b"body");
        assert!(signature.starts_with("sha256="));
        assert_eq!(signature.len(), "sha256=".len() + 64);
        assert_eq!(sign_secret_len_short(""), String::new());
    }

    fn sign_secret_len_short(secret: &str) -> String {
        sign_payload(secret, b"body")
    }
}
