//! Repository: OAuth2 client secrets — create, list, revoke.

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ClientSecretRow {
    pub id: i64,
    pub application_id: i64,
    pub secret_prefix: String,
    pub secret_hash: String,
    pub created_by: i64,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

pub async fn create_secret(
    pool: &PgPool, id: Snowflake, application_id: Snowflake,
    secret_prefix: &str, secret_hash: &str, created_by: Snowflake,
    label: Option<&str>,
) -> Result<ClientSecretRow, sqlx::Error> {
    sqlx::query_as::<_, ClientSecretRow>(
        "INSERT INTO oauth_client_secrets (id, application_id, secret_prefix, secret_hash, created_by, label)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING *"
    )
    .bind(id.as_i64()).bind(application_id.as_i64())
    .bind(secret_prefix).bind(secret_hash)
    .bind(created_by.as_i64()).bind(label)
    .fetch_one(pool).await
}

pub async fn list_secrets(
    pool: &PgPool, application_id: Snowflake,
) -> Result<Vec<ClientSecretRow>, sqlx::Error> {
    sqlx::query_as::<_, ClientSecretRow>(
        "SELECT * FROM oauth_client_secrets WHERE application_id = $1 ORDER BY created_at DESC"
    )
    .bind(application_id.as_i64()).fetch_all(pool).await
}

pub async fn revoke_secret(
    pool: &PgPool, secret_id: Snowflake, application_id: Snowflake,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE oauth_client_secrets SET revoked_at = NOW()
         WHERE id = $1 AND application_id = $2 AND revoked_at IS NULL"
    )
    .bind(secret_id.as_i64()).bind(application_id.as_i64())
    .execute(pool).await?;
    Ok(result.rows_affected() > 0)
}

/// Verify a client secret against all active (non-revoked) secrets for an application.
/// Returns true if the plaintext secret matches any stored Argon2id hash.
pub async fn verify_client_secret(
    pool: &PgPool,
    application_id: Snowflake,
    plaintext_secret: &str,
) -> Result<bool, sqlx::Error> {
    let rows: Vec<ClientSecretRow> = sqlx::query_as::<_, ClientSecretRow>(
        "SELECT * FROM oauth_client_secrets
         WHERE application_id = $1 AND revoked_at IS NULL
         ORDER BY created_at DESC"
    )
    .bind(application_id.as_i64())
    .fetch_all(pool).await?;

    for row in &rows {
        let valid = opencorde_core::password::verify_password(plaintext_secret, &row.secret_hash)
            .map_err(|e| sqlx::Error::Protocol(format!("hash verify failed: {e}")))?;
        if valid {
            return Ok(true);
        }
    }
    Ok(false)
}
