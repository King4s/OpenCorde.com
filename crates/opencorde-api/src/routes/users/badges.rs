//! GET /api/v1/users/{id}/badges handler.
//! Returns all badges awarded to a user.

use axum::{
    Json,
    extract::{Path, State},
};
use opencorde_db::repos::badge_repo;
use serde::Serialize;

use super::super::helpers::parse_snowflake;
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

/// Badge entry in API response.
#[derive(Debug, Serialize)]
pub struct BadgeEntry {
    pub badge_type: String,
}

/// GET /api/v1/users/{id}/badges — Get a user's public badges.
#[tracing::instrument(skip(state, auth))]
pub async fn get_user_badges(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Vec<BadgeEntry>>, ApiError> {
    tracing::info!(user_id = %auth.user_id, target_id = %id, "fetching user badges");

    let target_id = parse_snowflake(&id)?;

    let badges = badge_repo::list_by_user(&state.db, target_id.as_i64())
        .await
        .map_err(ApiError::Database)?;

    let entries: Vec<BadgeEntry> = badges
        .into_iter()
        .map(|b| BadgeEntry {
            badge_type: b.badge_type,
        })
        .collect();

    Ok(Json(entries))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_entry_serialization() {
        let entry = BadgeEntry {
            badge_type: "early_adopter".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("early_adopter"));
    }
}
