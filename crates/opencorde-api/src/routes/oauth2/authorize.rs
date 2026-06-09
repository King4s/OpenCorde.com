//! OAuth2 authorize endpoint — stub for future implementation.
//! Currently returns HTML page with authorization prompt placeholder.

use axum::{extract::Query, response::Html};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AuthorizeParams {
    pub client_id: String,
    pub redirect_uri: Option<String>,
    pub scope: Option<String>,
    pub state: Option<String>,
}

pub async fn authorize_page(
    Query(params): Query<AuthorizeParams>,
) -> Html<String> {
    Html(format!(
        "<html><body><h1>Authorize Application</h1>
         <p>Client: {}</p><p>Scope: {}</p>
         <p><em>OAuth2 authorization flow — full implementation pending.</em></p>
         </body></html>",
        params.client_id, params.scope.as_deref().unwrap_or("none")
    ))
}

#[derive(Debug, serde::Deserialize)]
pub struct ApproveBody {
    pub client_id: String,
    pub scope: String,
    pub approved: bool,
}

pub async fn approve(
    axum::extract::Form(body): axum::extract::Form<ApproveBody>,
) -> Html<String> {
    if body.approved {
        Html(format!("<h1>Approved!</h1><p>Authorization granted for {}.</p>", body.scope))
    } else {
        Html("<h1>Denied</h1><p>Authorization was not granted.</p>".into())
    }
}
