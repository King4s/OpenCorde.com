//! # Discord REST API Client
//! Thin wrapper around twilight-http for sending messages via Discord webhooks.
//!
//! ## Used For
//! - OpenCorde → Discord direction: post messages as the original author
//!   using a Discord webhook with username/avatar overrides
//!
//! ## Depends On
//! - twilight-http (Discord REST API)
//! - twilight-model (Id type markers)

use std::sync::Arc;

use anyhow::Context;
use twilight_http::Client;
use twilight_model::id::{marker::WebhookMarker, Id};

/// Discord REST API wrapper.
///
/// Cheap to clone — the inner `Client` is behind an `Arc`.
#[derive(Clone)]
pub struct DiscordApi {
    client: Arc<Client>,
}

impl DiscordApi {
    /// Create a new client authenticated with the given bot token.
    pub fn new(token: &str) -> Self {
        Self {
            client: Arc::new(Client::new(token.to_string())),
        }
    }

    /// Send a message to a Discord channel via webhook, appearing as a specific user.
    ///
    /// Uses username/avatar override so the message appears to come from the
    /// original OpenCorde user rather than the bot account.
    ///
    /// # Arguments
    /// - `webhook_id`    — Discord webhook ID (u64)
    /// - `webhook_token` — Discord webhook token
    /// - `username`      — Display name to show in Discord
    /// - `content`       — Message text
    /// - `avatar_url`    — Optional avatar image URL for the override
    #[tracing::instrument(skip(self, webhook_token), fields(webhook_id, username))]
    pub async fn send_webhook_message(
        &self,
        webhook_id: u64,
        webhook_token: &str,
        username: &str,
        content: &str,
        avatar_url: Option<&str>,
    ) -> anyhow::Result<()> {
        let id: Id<WebhookMarker> = Id::new(webhook_id);

        // username() and content() return Result<Self, MessageValidationError>
        let req = self
            .client
            .execute_webhook(id, webhook_token)
            .username(username)?
            .content(content)?;

        // avatar_url() is infallible (returns Self), so branch after building base req
        let result = if let Some(url) = avatar_url {
            req.avatar_url(url).await
        } else {
            req.await
        };

        result.context("webhook execute failed")?;

        tracing::debug!(webhook_id, username, "message forwarded to Discord");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discord_api_creation() {
        let _api = DiscordApi::new("Bot test_token_placeholder");
    }

    #[test]
    fn test_discord_api_clone() {
        let api = DiscordApi::new("Bot test_token");
        let _cloned = api.clone();
    }
}
