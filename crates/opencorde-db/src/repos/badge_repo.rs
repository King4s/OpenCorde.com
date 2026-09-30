//! # Repository: User Badges
//! CRUD operations for user badges (profile decorations).
//!
//! Provides functions to list, award, and revoke badges for users.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// Row type for reading user badges from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserBadgeRow {
    pub user_id: i64,
    pub badge_type: String,
    pub awarded_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Serializable badge entry for API responses.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UserBadge {
    pub badge_type: String,
    pub awarded_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<UserBadgeRow> for UserBadge {
    fn from(row: UserBadgeRow) -> Self {
        Self {
            badge_type: row.badge_type,
            awarded_at: row.awarded_at,
            expires_at: row.expires_at,
        }
    }
}

/// List all badges for a user.
#[tracing::instrument(skip(pool))]
pub async fn list_by_user(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<UserBadge>, sqlx::Error> {
    let rows = sqlx::query_as::<_, UserBadgeRow>(
        "SELECT user_id, badge_type, awarded_at, expires_at \
         FROM user_badges WHERE user_id = $1 \
         ORDER BY awarded_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(UserBadge::from).collect())
}

/// Award a badge to a user. No-op if the badge already exists.
#[tracing::instrument(skip(pool))]
pub async fn award_badge(
    pool: &PgPool,
    user_id: i64,
    badge_type: &str,
    expires_at: Option<DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO user_badges (user_id, badge_type, expires_at) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (user_id, badge_type) DO NOTHING"
    )
    .bind(user_id)
    .bind(badge_type)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Revoke a badge from a user.
#[tracing::instrument(skip(pool))]
pub async fn revoke_badge(
    pool: &PgPool,
    user_id: i64,
    badge_type: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM user_badges WHERE user_id = $1 AND badge_type = $2"
    )
    .bind(user_id)
    .bind(badge_type)
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_badge_from_row() {
        let now = Utc::now();
        let row = UserBadgeRow {
            user_id: 1,
            badge_type: "early_adopter".to_string(),
            awarded_at: now,
            expires_at: None,
        };
        let badge = UserBadge::from(row);
        assert_eq!(badge.badge_type, "early_adopter");
        assert_eq!(badge.expires_at, None);
    }
}
