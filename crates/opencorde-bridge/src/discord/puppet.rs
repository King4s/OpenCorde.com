//! # Ghost User Management
//! Creates and looks up OpenCorde user accounts that mirror Discord users.
//!
//! Ghost users are lightweight users (no email, no password) created when
//! Discord messages arrive in bridged channels. They let Discord users appear
//! as real participants in OpenCorde without requiring a registered account.
//!
//! ## Flow
//! 1. Discord MESSAGE_CREATE arrives
//! 2. Look up discord_user_id in bridge_ghost_users
//! 3. If missing, create a user row + insert into bridge_ghost_users
//! 4. Return the opencorde_user_id for message insertion
//!
//! ## Depends On
//! - sqlx (database)
//! - opencorde_core::SnowflakeGenerator (ID generation)
//! - rand (random public key for ghost user)

use opencorde_core::SnowflakeGenerator;
use rand::RngCore;
use sqlx::{PgPool, Row};
use std::sync::Mutex;

static GENERATOR: Mutex<Option<SnowflakeGenerator>> = Mutex::new(None);

fn next_id() -> i64 {
    let mut guard = GENERATOR.lock().expect("snowflake generator poisoned");
    let sg = guard.get_or_insert_with(|| SnowflakeGenerator::new(1, 1));
    sg.next_id().as_i64()
}

/// Generate a 64-char hex public key (ghost users don't use E2EE).
fn random_public_key() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Sanitise a Discord username to fit OpenCorde's alphanumeric+underscore rules.
fn sanitise_username(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

/// Build a unique ghost username: up to 22 chars of sanitised name + `_dc` + 5-digit suffix.
fn ghost_username(discord_username: &str, discord_user_id: u64) -> String {
    let base: String = sanitise_username(discord_username)
        .chars()
        .take(22)
        .collect();
    let suffix = discord_user_id % 100_000;
    format!("{}_dc{:05}", base, suffix)
}

/// Find or create the OpenCorde ghost user for a Discord user.
///
/// Returns the OpenCorde `user_id` (i64) to use as `author_id` when inserting messages.
#[tracing::instrument(skip(db), fields(discord_user_id, discord_username))]
pub async fn find_or_create(
    db: &PgPool,
    discord_user_id: u64,
    discord_username: &str,
    discord_avatar_url: Option<&str>,
) -> anyhow::Result<i64> {
    // Fast path: ghost user already exists
    if let Some(row) = sqlx::query(
        "SELECT opencorde_user_id FROM bridge_ghost_users WHERE discord_user_id = $1",
    )
    .bind(discord_user_id as i64)
    .fetch_optional(db)
    .await?
    {
        // Update last_seen and avatar in background (non-critical)
        let _ = sqlx::query(
            "UPDATE bridge_ghost_users SET last_seen = NOW(), discord_avatar_url = $1
             WHERE discord_user_id = $2",
        )
        .bind(discord_avatar_url)
        .bind(discord_user_id as i64)
        .execute(db)
        .await;

        return Ok(row.get("opencorde_user_id"));
    }

    // Slow path: create ghost user
    let user_id = next_id();
    let username = ghost_username(discord_username, discord_user_id);
    let public_key = random_public_key();

    sqlx::query(
        "INSERT INTO users (id, username, public_key, avatar_url)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (username) DO NOTHING",
    )
    .bind(user_id)
    .bind(&username)
    .bind(&public_key)
    .bind(discord_avatar_url)
    .execute(db)
    .await?;

    // Resolve actual user_id (may differ if username collision → ON CONFLICT DO NOTHING)
    // In that case, generate another unique ID with a longer suffix
    let actual_id = sqlx::query("SELECT id FROM users WHERE username = $1")
        .bind(&username)
        .fetch_one(db)
        .await?
        .get::<i64, _>("id");

    sqlx::query(
        "INSERT INTO bridge_ghost_users
            (discord_user_id, opencorde_user_id, discord_username, discord_avatar_url)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (discord_user_id) DO UPDATE
             SET opencorde_user_id = EXCLUDED.opencorde_user_id,
                 discord_avatar_url = EXCLUDED.discord_avatar_url,
                 last_seen = NOW()",
    )
    .bind(discord_user_id as i64)
    .bind(actual_id)
    .bind(discord_username)
    .bind(discord_avatar_url)
    .execute(db)
    .await?;

    tracing::info!(
        discord_user_id,
        opencorde_user_id = actual_id,
        username = %username,
        "ghost user created"
    );

    Ok(actual_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ghost_username_sanitises() {
        let name = ghost_username("alice smith", 12345);
        assert!(name.chars().all(|c| c.is_alphanumeric() || c == '_'));
        assert!(name.len() <= 32);
    }

    #[test]
    fn test_ghost_username_long_input() {
        let long = "a".repeat(50);
        let name = ghost_username(&long, 99999);
        assert!(name.len() <= 32);
    }

    #[test]
    fn test_random_public_key_length() {
        let key = random_public_key();
        assert_eq!(key.len(), 64);
    }
}
