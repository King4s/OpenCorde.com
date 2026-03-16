//! # Repository: Users
//! CRUD operations for user accounts.
//!
//! Provides functions to create, read, and update user data in PostgreSQL.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading users from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRow {
    pub id: i64,
    pub username: String,
    pub public_key: String,    // Ed25519 public key (hex-encoded, 64 chars)
    pub email: Option<String>, // Optional: for password recovery
    pub password_hash: Option<String>, // Optional: for email+password login
    pub avatar_url: Option<String>,
    pub status: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new user in the database.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID for the user
/// * `username` - User's display name (max 32 chars)
/// * `public_key` - Ed25519 public key (hex-encoded, 64 chars). Primary identity.
/// * `email` - Optional: user's email address (for password recovery)
/// * `password_hash` - Optional: Argon2 password hash (only if email is provided)
///
/// # Errors
/// Returns sqlx::Error if the insert fails (e.g., duplicate username/public_key).
#[tracing::instrument(skip(pool, password_hash))]
pub async fn create_user(
    pool: &PgPool,
    id: Snowflake,
    username: &str,
    public_key: &str,
    email: Option<&str>,
    password_hash: Option<&str>,
) -> Result<UserRow, sqlx::Error> {
    tracing::info!(username = %username, "creating user with Ed25519 keypair");

    let row = sqlx::query_as::<_, UserRow>(
        "INSERT INTO users (id, username, public_key, email, password_hash) \
         VALUES ($1, $2, $3, $4, $5) RETURNING *",
    )
    .bind(id.as_i64())
    .bind(username)
    .bind(public_key)
    .bind(email)
    .bind(password_hash)
    .fetch_one(pool)
    .await?;

    tracing::info!(user_id = row.id, "user created successfully");
    Ok(row)
}

/// Get a user by their Snowflake ID.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_by_id(pool: &PgPool, id: Snowflake) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = $1")
        .bind(id.as_i64())
        .fetch_optional(pool)
        .await
}

/// Get a user by their email address.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_by_email(pool: &PgPool, email: &str) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await
}

/// Get a user by their username.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await
}

/// Update a user's avatar URL.
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool))]
pub async fn update_avatar(
    pool: &PgPool,
    id: Snowflake,
    avatar_url: &str,
) -> Result<(), sqlx::Error> {
    tracing::info!(user_id = id.as_i64(), "updating user avatar");

    sqlx::query("UPDATE users SET avatar_url = $1, updated_at = NOW() WHERE id = $2")
        .bind(avatar_url)
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

/// Update a user's status (0=Online, 1=Idle, 2=DND, 3=Offline).
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool))]
pub async fn update_status(pool: &PgPool, id: Snowflake, status: i16) -> Result<(), sqlx::Error> {
    tracing::info!(
        user_id = id.as_i64(),
        status = status,
        "updating user status"
    );

    sqlx::query("UPDATE users SET status = $1, updated_at = NOW() WHERE id = $2")
        .bind(status)
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_row_creation() {
        let now = Utc::now();
        let row = UserRow {
            id: 123456789,
            username: "testuser".to_string(),
            public_key: "abc123def456abc123def456abc123def456abc123def456abc123def456abc1"
                .to_string(),
            email: Some("test@example.com".to_string()),
            password_hash: Some("hash".to_string()),
            avatar_url: None,
            status: 3,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(row.id, 123456789);
        assert_eq!(row.username, "testuser");
        assert_eq!(row.public_key.len(), 64);
        assert_eq!(row.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_snowflake_conversion() {
        let sf = Snowflake::new(999888777);
        let as_i64 = sf.as_i64();
        assert_eq!(as_i64, 999888777);
    }
}
