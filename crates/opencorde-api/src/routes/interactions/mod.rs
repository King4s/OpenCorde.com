//! Interaction routes — command/component/modal callbacks.
use axum::{Router, routing::post};
use crate::AppState;

mod callback;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/interactions/{id}/{token}/callback", post(callback::interaction_callback))
}
