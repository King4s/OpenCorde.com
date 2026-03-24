//! # Push Notification Sender
//! Delivers push notifications to registered device tokens.
//!
//! ## Platforms
//! - `web`  — Web Push (VAPID, via `web-push` crate with `IsahcWebPushClient`)
//! - `fcm`  — Firebase Cloud Messaging (Android), legacy HTTP endpoint
//! - `apns` — Apple Push Notification Service (iOS) — stub, logs warning
//!
//! ## Usage
//! ```ignore
//! push_sender::send_push(&state.db, &state.config, user_id, "Title", "Body").await;
//! ```
//!
//! ## Depends On
//! - sqlx::PgPool (push_tokens table lookup)
//! - crate::config::Config (FCM_SERVER_KEY, VAPID keys)
//! - web_push (VAPID Web Push sending via IsahcWebPushClient)
//! - reqwest (FCM HTTP calls)

use sqlx::PgPool;
use tracing::instrument;

use crate::config::Config;

/// A token row fetched from `push_tokens`.
struct PushToken {
    token: String,
    platform: String,
}

/// Send a push notification to all registered tokens for `user_id`.
///
/// Fetches all tokens from `push_tokens`, dispatches by platform.
/// Errors per-token are logged but do not abort the remaining sends.
///
/// # Arguments
/// * `pool`    — active PostgreSQL connection pool
/// * `config`  — application config (VAPID / FCM keys)
/// * `user_id` — recipient user Snowflake ID
/// * `title`   — notification title
/// * `body`    — notification body text
#[instrument(skip(pool, config), fields(user_id = user_id))]
pub async fn send_push(pool: &PgPool, config: &Config, user_id: i64, title: &str, body: &str) {
    // Fetch all tokens registered for this user
    let tokens: Vec<PushToken> = match sqlx::query_as!(
        PushToken,
        "SELECT token, platform FROM push_tokens WHERE user_id = $1",
        user_id
    )
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(user_id, error = %e, "failed to fetch push tokens");
            return;
        }
    };

    if tokens.is_empty() {
        tracing::debug!(user_id, "no push tokens registered, skipping");
        return;
    }

    tracing::info!(user_id, count = tokens.len(), "dispatching push notifications");

    for tok in &tokens {
        match tok.platform.as_str() {
            "web" => {
                send_web_push(config, &tok.token, title, body).await;
            }
            "fcm" => {
                send_fcm_push(config, &tok.token, title, body).await;
            }
            "apns" => {
                tracing::warn!(
                    user_id,
                    "APNS not yet configured — skipping iOS push notification"
                );
            }
            other => {
                tracing::warn!(user_id, platform = other, "unknown push platform, skipping");
            }
        }
    }
}

/// Send a Web Push notification via VAPID.
///
/// `subscription_json` is the JSON-serialised `PushSubscription` object from
/// the browser, which contains `endpoint`, `keys.p256dh`, and `keys.auth`.
/// The VAPID private key must be URL-safe base64 encoded (no padding).
async fn send_web_push(config: &Config, subscription_json: &str, title: &str, body: &str) {
    let private_key = match &config.vapid_private_key {
        Some(k) => k.clone(),
        None => {
            tracing::warn!("VAPID_PRIVATE_KEY not configured, cannot send Web Push");
            return;
        }
    };

    // Parse the browser PushSubscription JSON: { endpoint, keys: { p256dh, auth } }
    let parsed: serde_json::Value = match serde_json::from_str(subscription_json) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "failed to parse Web Push subscription JSON");
            return;
        }
    };

    let endpoint = match parsed.get("endpoint").and_then(|v| v.as_str()) {
        Some(e) => e.to_string(),
        None => {
            tracing::warn!("Web Push subscription missing endpoint");
            return;
        }
    };
    let p256dh = parsed
        .pointer("/keys/p256dh")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let auth = parsed
        .pointer("/keys/auth")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let subscription_info = web_push::SubscriptionInfo::new(endpoint, p256dh, auth);

    let payload = serde_json::json!({ "title": title, "body": body }).to_string();

    let sig_builder = match web_push::VapidSignatureBuilder::from_base64(
        &private_key,
        web_push::URL_SAFE_NO_PAD,
        &subscription_info,
    ) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error = %e, "failed to build VAPID signature builder");
            return;
        }
    };

    let sig = match sig_builder.build() {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "failed to build VAPID signature");
            return;
        }
    };

    let mut msg_builder = web_push::WebPushMessageBuilder::new(&subscription_info);
    msg_builder.set_payload(web_push::ContentEncoding::Aes128Gcm, payload.as_bytes());
    msg_builder.set_vapid_signature(sig);

    let message = match msg_builder.build() {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, "failed to build Web Push message");
            return;
        }
    };

    let client = match web_push::IsahcWebPushClient::new() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to create Web Push client");
            return;
        }
    };

    use web_push::WebPushClient;
    if let Err(e) = client.send(message).await {
        tracing::warn!(error = %e, "Web Push send failed");
    } else {
        tracing::debug!("Web Push sent successfully");
    }
}

/// Send a push via Firebase Cloud Messaging (Android).
///
/// Uses the legacy FCM HTTP endpoint. Requires `FCM_SERVER_KEY` in config.
async fn send_fcm_push(config: &Config, device_token: &str, title: &str, body: &str) {
    let server_key = match &config.fcm_server_key {
        Some(k) => k.clone(),
        None => {
            tracing::warn!("FCM_SERVER_KEY not configured, cannot send FCM push");
            return;
        }
    };

    let payload = serde_json::json!({
        "to": device_token,
        "notification": {
            "title": title,
            "body": body
        }
    });

    let client = reqwest::Client::new();
    let res = client
        .post("https://fcm.googleapis.com/fcm/send")
        .header("Authorization", format!("key={}", server_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await;

    match res {
        Ok(r) if r.status().is_success() => {
            tracing::debug!("FCM push sent successfully");
        }
        Ok(r) => {
            tracing::warn!(status = %r.status(), "FCM push returned non-success status");
        }
        Err(e) => {
            tracing::warn!(error = %e, "FCM push HTTP request failed");
        }
    }
}
