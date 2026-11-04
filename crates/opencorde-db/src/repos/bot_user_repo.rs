//! # Repository: Bot Users
//! CRUD operations for bot user accounts linked to applications.
//!
//! Bot users are special user accounts (is_bot=true in users table) with
//! additional metadata in the bot_users table.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading bot users from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BotUserRow {
    pub id: i64,
    pub application_id: i64,
    pub username: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Create a bot user account.
///
/// Creates both the users row (with is_bot=true) and the bot_users row in a transaction.
///
/// # Errors
/// Returns sqlx::Error if the insert fails (e.g., duplicate username).
#[tracing::instrument(skip(pool))]
pub async fn create_bot_user(
    pool: &PgPool,
    id: Snowflake,
    application_id: Snowflake,
    username: &str,
    public_key: &str,
    avatar_url: Option<&str>,
) -> Result<BotUserRow, sqlx::Error> {
    tracing::info!(
        bot_id = id.as_i64(),
        app_id = application_id.as_i64(),
        username = %username,
        "creating bot user"
    );

    let mut tx = pool.begin().await?;

    // Insert the user row with is_bot=true and application_id set
    sqlx::query(
        r#"
        INSERT INTO users (id, username, public_key, is_bot, application_id, avatar_url)
        VALUES ($1, $2, $3, TRUE, $4, $5)
        "#,
    )
    .bind(id.as_i64())
    .bind(username)
    .bind(public_key)
    .bind(application_id.as_i64())
    .bind(avatar_url)
    .execute(&mut *tx)
    .await?;

    // Insert the bot_users row
    let row = sqlx::query_as::<_, BotUserRow>(
        r#"
        INSERT INTO bot_users (id, application_id, username, avatar_url)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(id.as_i64())
    .bind(application_id.as_i64())
    .bind(username)
    .bind(avatar_url)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!(bot_id = id.as_i64(), "bot user created successfully");
    Ok(row)
}

/// Get a bot user by their ID.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_by_id(pool: &PgPool, id: Snowflake) -> Result<Option<BotUserRow>, sqlx::Error> {
    sqlx::query_as::<_, BotUserRow>("SELECT * FROM bot_users WHERE id = $1")
        .bind(id.as_i64())
        .fetch_optional(pool)
        .await
}

/// List all bot users for an application.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_by_application(
    pool: &PgPool,
    application_id: Snowflake,
) -> Result<Vec<BotUserRow>, sqlx::Error> {
    tracing::debug!(
        app_id = application_id.as_i64(),
        "listing bot users for application"
    );

    sqlx::query_as::<_, BotUserRow>(
        "SELECT * FROM bot_users WHERE application_id = $1 ORDER BY created_at",
    )
    .bind(application_id.as_i64())
    .fetch_all(pool)
    .await
}

/// Update a bot user's avatar.
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool))]
pub async fn update_avatar(
    pool: &PgPool,
    id: Snowflake,
    avatar_url: &str,
) -> Result<(), sqlx::Error> {
    tracing::info!(bot_id = id.as_i64(), "updating bot avatar");

    sqlx::query("UPDATE bot_users SET avatar_url = $1 WHERE id = $2")
        .bind(avatar_url)
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

/// Delete a bot user.
///
/// # Errors
/// Returns sqlx::Error if the delete fails.
#[tracing::instrument(skip(pool))]
pub async fn delete_bot_user(pool: &PgPool, id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(bot_id = id.as_i64(), "deleting bot user");

    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM bot_users WHERE id = $1")
        .bind(id.as_i64())
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id.as_i64())
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bot_user_row_creation() {
        let now = Utc::now();
        let row = BotUserRow {
            id: 5000,
            application_id: 1000,
            username: "HelperBot".to_string(),
            avatar_url: Some("https://example.com/bot.png".to_string()),
            created_at: now,
        };

        assert_eq!(row.id, 5000);
        assert_eq!(row.application_id, 1000);
        assert_eq!(row.username, "HelperBot");
        assert!(row.avatar_url.is_some());
    }

    #[test]
    fn test_bot_user_row_no_avatar() {
        let now = Utc::now();
        let row = BotUserRow {
            id: 6000,
            application_id: 2000,
            username: "NoAvatarBot".to_string(),
            avatar_url: None,
            created_at: now,
        };

        assert_eq!(row.username, "NoAvatarBot");
        assert!(row.avatar_url.is_none());
    }
}
