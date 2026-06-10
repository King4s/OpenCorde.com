//! Repository: 2FA recovery codes — generate, consume, list, clear.

use chrono::{DateTime, Utc};
use opencorde_core::password;
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecoveryCodeRow {
    pub id: i64,
    pub user_id: i64,
    pub code_hash: String,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RecoveryCodeInfo {
    pub id: i64,
    pub used: bool,
    pub created_at: DateTime<Utc>,
}

/// Generate 10 one-time recovery codes for a user. Returns the plaintext codes
/// (to be shown once) and stores only Argon2id hashes.
pub async fn generate_codes(
    pool: &PgPool,
    user_id: Snowflake,
    generator: &mut opencorde_core::snowflake::SnowflakeGenerator,
) -> Result<Vec<String>, sqlx::Error> {
    // Clear any existing unused codes first
    clear_codes(pool, user_id).await?;

    let mut plaintext_codes = Vec::with_capacity(10);

    for _ in 0..10 {
        let plaintext = Uuid::new_v4().to_string().replace('-', "").to_uppercase();
        let code_hash = password::hash_password(&plaintext)
            .map_err(|e| sqlx::Error::Protocol(format!("hashing failed: {e}")))?;

        let id = generator.next_id();
        sqlx::query(
            "INSERT INTO totp_recovery_codes (id, user_id, code_hash) VALUES ($1, $2, $3)"
        )
        .bind(id.as_i64()).bind(user_id.as_i64()).bind(&code_hash)
        .execute(pool).await?;

        plaintext_codes.push(plaintext);
    }

    Ok(plaintext_codes)
}

/// Attempt to consume a recovery code. Returns true if consumed, false if invalid/used.
/// This is atomic — only one caller wins if multiple concurrent attempts.
pub async fn consume_code(
    pool: &PgPool,
    user_id: Snowflake,
    plaintext_code: &str,
) -> Result<bool, sqlx::Error> {
    // Fetch active codes for this user
    let candidates: Vec<RecoveryCodeRow> = sqlx::query_as::<_, RecoveryCodeRow>(
        "SELECT * FROM totp_recovery_codes WHERE user_id = $1 AND used_at IS NULL ORDER BY created_at"
    )
    .bind(user_id.as_i64())
    .fetch_all(pool).await?;

    for row in candidates {
        let valid = password::verify_password(plaintext_code, &row.code_hash)
            .map_err(|e| sqlx::Error::Protocol(format!("hash verify failed: {e}")))?;

        if valid {
            // Atomic consumption — UPDATE WHERE used_at IS NULL
            let result = sqlx::query(
                "UPDATE totp_recovery_codes SET used_at = NOW() WHERE id = $1 AND used_at IS NULL"
            )
            .bind(row.id)
            .execute(pool).await?;

            return Ok(result.rows_affected() > 0);
        }
    }

    Ok(false)
}

/// List recovery codes for a user (no plaintext — just status).
pub async fn list_codes(
    pool: &PgPool,
    user_id: Snowflake,
) -> Result<Vec<RecoveryCodeInfo>, sqlx::Error> {
    let rows: Vec<RecoveryCodeRow> = sqlx::query_as::<_, RecoveryCodeRow>(
        "SELECT * FROM totp_recovery_codes WHERE user_id = $1 ORDER BY created_at"
    )
    .bind(user_id.as_i64())
    .fetch_all(pool).await?;

    Ok(rows.into_iter().map(|r| RecoveryCodeInfo {
        id: r.id,
        used: r.used_at.is_some(),
        created_at: r.created_at,
    }).collect())
}

/// Count unused recovery codes remaining for a user.
pub async fn count_remaining(pool: &PgPool, user_id: Snowflake) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM totp_recovery_codes WHERE user_id = $1 AND used_at IS NULL"
    )
    .bind(user_id.as_i64())
    .fetch_one(pool).await?;
    Ok(row.0)
}

/// Delete all recovery codes for a user (on disable or regenerate).
pub async fn clear_codes(pool: &PgPool, user_id: Snowflake) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM totp_recovery_codes WHERE user_id = $1")
        .bind(user_id.as_i64())
        .execute(pool).await?;
    Ok(())
}
