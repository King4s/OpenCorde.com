//! Route: Users — public and private profile endpoints.
//! GET    /api/v1/users/@me        — authenticated user profile
//! PATCH  /api/v1/users/@me        — update authenticated user
//! DELETE /api/v1/users/@me        — delete account (GDPR right to erasure)
//! POST   /api/v1/users/@me/avatar — upload user avatar
//! GET    /api/v1/users/{id}       — public user profile

use axum::{Router, routing::{get, post}};

use crate::AppState;

mod avatar;
mod delete;
mod get;
mod update;

pub use get::{UserProfile, PublicUserProfile};
pub use update::UpdateMeRequest;
pub use avatar::upload_avatar;

/// Build the users router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/users/@me",
            get(get::get_me)
                .patch(update::update_me)
                .delete(delete::delete_account),
        )
        .route("/api/v1/users/@me/avatar", post(upload_avatar))
        .route("/api/v1/users/{id}", get(get::get_user_profile))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_profile_serialization() {
        let profile = UserProfile {
            id: "123".to_string(),
            username: "test".to_string(),
            public_key: "abc123def456abc123def456abc123def456abc123def456abc123def456abc1"
                .to_string(),
            email: Some("t@e.com".to_string()),
            avatar_url: Some("https://x.com/a.png".to_string()),
            status: 0,
            bio: None,
            status_message: None,
            totp_enabled: false,
        };
        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("123"));
        assert!(json.contains("test"));
    }
}
