//! # Repository: Applications
//! CRUD operations for registered OAuth2 applications.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use serde_json::Value;
use sqlx::PgPool;

/// Row type for reading applications from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApplicationRow {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub owner_user_id: i64,
    pub flags: i64,
    pub is_public: bool,
    pub redirect_uris: Value, // JSONB — array of strings
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new application.
///
/// # Errors
/// Returns sqlx::Error if the insert fails (e.g., duplicate name).
#[tracing::instrument(skip(pool))]
pub async fn create_application(
    pool: &PgPool,
    id: Snowflake,
    name: &str,
    description: Option<&str>,
    icon_url: Option<&str>,
    owner_user_id: Snowflake,
    flags: i64,
    is_public: bool,
    redirect_uris: &[String],
) -> Result<ApplicationRow, sqlx::Error> {
    tracing::info!(app_id = id.as_i64(), name = %name, "creating application");

    let redirect_uris_json = serde_json::to_value(redirect_uris).unwrap_or_default();

    sqlx::query_as::<_, ApplicationRow>(
        r#"
        INSERT INTO applications (id, name, description, icon_url, owner_user_id, flags, is_public, redirect_uris)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(id.as_i64())
    .bind(name)
    .bind(description)
    .bind(icon_url)
    .bind(owner_user_id.as_i64())
    .bind(flags)
    .bind(is_public)
    .bind(&redirect_uris_json)
    .fetch_one(pool)
    .await
}

/// Get an application by its Snowflake ID.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_by_id(
    pool: &PgPool,
    id: Snowflake,
) -> Result<Option<ApplicationRow>, sqlx::Error> {
    sqlx::query_as::<_, ApplicationRow>("SELECT * FROM applications WHERE id = $1")
        .bind(id.as_i64())
        .fetch_optional(pool)
        .await
}

/// List all applications owned by a user.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_by_owner(
    pool: &PgPool,
    owner_user_id: Snowflake,
) -> Result<Vec<ApplicationRow>, sqlx::Error> {
    tracing::debug!(
        owner_id = owner_user_id.as_i64(),
        "listing applications by owner"
    );

    sqlx::query_as::<_, ApplicationRow>(
        "SELECT * FROM applications WHERE owner_user_id = $1 ORDER BY created_at DESC",
    )
    .bind(owner_user_id.as_i64())
    .fetch_all(pool)
    .await
}

/// Update an application's metadata.
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool))]
pub async fn update_application(
    pool: &PgPool,
    id: Snowflake,
    name: &str,
    description: Option<&str>,
    icon_url: Option<&str>,
    is_public: bool,
    redirect_uris: &[String],
) -> Result<(), sqlx::Error> {
    tracing::info!(app_id = id.as_i64(), name = %name, "updating application");

    let redirect_uris_json = serde_json::to_value(redirect_uris).unwrap_or_default();

    sqlx::query(
        r#"
        UPDATE applications
        SET name = $2, description = $3, icon_url = $4, is_public = $5, redirect_uris = $6, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id.as_i64())
    .bind(name)
    .bind(description)
    .bind(icon_url)
    .bind(is_public)
    .bind(&redirect_uris_json)
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete an application.
///
/// # Errors
/// Returns sqlx::Error if the delete fails.
#[tracing::instrument(skip(pool))]
pub async fn delete_application(pool: &PgPool, id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(app_id = id.as_i64(), "deleting application");

    sqlx::query("DELETE FROM applications WHERE id = $1")
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

/// Get an application by name (unique).
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_by_name(pool: &PgPool, name: &str) -> Result<Option<ApplicationRow>, sqlx::Error> {
    tracing::debug!(name = %name, "fetching application by name");

    sqlx::query_as::<_, ApplicationRow>("SELECT * FROM applications WHERE name = $1")
        .bind(name)
        .fetch_optional(pool)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_row_creation() {
        let now = Utc::now();
        let row = ApplicationRow {
            id: 1000,
            name: "TestApp".to_string(),
            description: Some("A test app".to_string()),
            icon_url: None,
            owner_user_id: 200,
            flags: 0,
            is_public: true,
            redirect_uris: serde_json::json!(["https://example.com/callback"]),
            created_at: now,
            updated_at: now,
        };

        assert_eq!(row.id, 1000);
        assert_eq!(row.name, "TestApp");
        assert!(row.is_public);
        assert_eq!(row.owner_user_id, 200);
    }

    #[test]
    fn test_application_row_empty_redirects() {
        let now = Utc::now();
        let row = ApplicationRow {
            id: 2000,
            name: "PrivateApp".to_string(),
            description: None,
            icon_url: None,
            owner_user_id: 300,
            flags: 0,
            is_public: false,
            redirect_uris: serde_json::json!([]),
            created_at: now,
            updated_at: now,
        };

        assert!(!row.is_public);
        assert!(row.description.is_none());
    }
}
