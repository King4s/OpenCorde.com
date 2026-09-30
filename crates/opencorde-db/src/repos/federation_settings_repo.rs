//! # Repository: Federation Settings
//! Instance-wide federation policy configuration.
//!
//! Manages the federation_settings singleton table.
//!
//! ## Depends On
//! - sqlx — Database access

use sqlx::PgPool;

/// Federation policy row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FederationSettingsRow {
    pub id: i16,
    pub allow_remote_dms: bool,
    pub dm_policy: i16,
    pub forward_reports: bool,
    pub instance_blocklist: String,
}

/// Get the singleton federation settings row.
pub async fn get(pool: &PgPool) -> Result<FederationSettingsRow, sqlx::Error> {
    sqlx::query_as::<_, FederationSettingsRow>("SELECT * FROM federation_settings WHERE id = 1")
        .fetch_one(pool)
        .await
}

/// Update the federation settings singleton.
pub async fn update(
    pool: &PgPool,
    allow_remote_dms: bool,
    dm_policy: i16,
    forward_reports: bool,
    instance_blocklist: &str,
) -> Result<FederationSettingsRow, sqlx::Error> {
    sqlx::query_as::<_, FederationSettingsRow>(
        "UPDATE federation_settings \
         SET allow_remote_dms = $1, dm_policy = $2, forward_reports = $3, instance_blocklist = $4 \
         WHERE id = 1 RETURNING *",
    )
    .bind(allow_remote_dms)
    .bind(dm_policy)
    .bind(forward_reports)
    .bind(instance_blocklist)
    .fetch_one(pool)
    .await
}
