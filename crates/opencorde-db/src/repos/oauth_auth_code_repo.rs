//! Repository: OAuth2 authorization codes — create, consume (one-time use).

use chrono::{DateTime, Duration, Utc};
use opencorde_core::password;
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuthCodeRow {
    pub id: i64,
    pub application_id: i64,
    pub user_id: i64,
    pub code_hash: String,
    pub redirect_uri: String,
    pub scope: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

/// Generate a fresh authorization code, store it hashed, and return the plaintext code
/// (shown to the user once, passed to the client via redirect).
pub async fn create_auth_code(
    pool: &PgPool,
    id: Snowflake,
    application_id: Snowflake,
    user_id: Snowflake,
    redirect_uri: &str,
    scope: &str,
    ttl_minutes: i64,
) -> Result<(String, AuthCodeRow), sqlx::Error> {
    let plaintext_code = Uuid::new_v4().to_string();

    let code_hash = password::hash_password(&plaintext_code)
        .map_err(|e| sqlx::Error::Protocol(format!("hashing failed: {e}")))?;

    let expires_at = Utc::now() + Duration::minutes(ttl_minutes);

    let row = sqlx::query_as::<_, AuthCodeRow>(
        "INSERT INTO oauth_authorization_codes (id, application_id, user_id, code_hash, redirect_uri, scope, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *"
    )
    .bind(id.as_i64()).bind(application_id.as_i64()).bind(user_id.as_i64())
    .bind(&code_hash).bind(redirect_uri).bind(scope).bind(expires_at)
    .fetch_one(pool).await?;

    Ok((plaintext_code, row))
}

/// Find an unused, non-expired authorization code for the given application,
/// verify the plaintext code, mark it consumed, and return the row.
/// Returns None if not found, expired, already used, hash mismatch, or redirect_uri mismatch.
pub async fn consume_auth_code(
    pool: &PgPool,
    application_id: Snowflake,
    plaintext_code: &str,
    redirect_uri: &str,
) -> Result<Option<AuthCodeRow>, sqlx::Error> {
    let candidates: Vec<AuthCodeRow> = sqlx::query_as::<_, AuthCodeRow>(
        "SELECT * FROM oauth_authorization_codes
         WHERE application_id = $1 AND used_at IS NULL AND expires_at > NOW()
         ORDER BY created_at DESC LIMIT 10",
    )
    .bind(application_id.as_i64())
    .fetch_all(pool)
    .await?;

    for row in candidates {
        let valid = password::verify_password(plaintext_code, &row.code_hash)
            .map_err(|e| sqlx::Error::Protocol(format!("hash verify failed: {e}")))?;

        if valid {
            if row.redirect_uri != redirect_uri {
                return Ok(None);
            }

            // Atomic consumption — only one caller wins
            let result = sqlx::query(
                "UPDATE oauth_authorization_codes SET used_at = NOW()
                 WHERE id = $1 AND used_at IS NULL",
            )
            .bind(row.id)
            .execute(pool)
            .await?;

            if result.rows_affected() == 0 {
                return Ok(None);
            }

            return Ok(Some(row));
        }
    }

    Ok(None)
}
