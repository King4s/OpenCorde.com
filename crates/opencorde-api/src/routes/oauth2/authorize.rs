//! OAuth2 authorize endpoint — authorization prompt and approval.
//!
//! GET  /api/v1/oauth2/authorize  — show authorization prompt
//! POST /api/v1/oauth2/authorize  — user approves, generates authorization code, redirects

use axum::extract::{Query, State};
use axum::response::{Html, Redirect};
use opencorde_core::snowflake::SnowflakeGenerator;
use serde::Deserialize;

use crate::AppState;
use crate::middleware::AuthUser;
use opencorde_db::repos;

#[derive(Debug, Deserialize)]
pub struct AuthorizeParams {
    pub client_id: String,
    pub redirect_uri: Option<String>,
    pub scope: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApproveBody {
    pub client_id: String,
    pub scope: String,
    pub redirect_uri: String,
    pub state: String,
    pub approved: bool,
}

/// GET /api/v1/oauth2/authorize
///
/// Renders an authorization prompt. In production this should be a full page
/// with app name, icon, and requested scopes. For now, a minimal HTML form.
pub async fn authorize_page(
    Query(params): Query<AuthorizeParams>,
) -> Html<String> {
    let client_name = &params.client_id; // In production: look up app name from DB
    let scope = params.scope.as_deref().unwrap_or("identify");
    let state = params.state.as_deref().unwrap_or("");
    let redirect = params.redirect_uri.as_deref().unwrap_or("");

    // Serialize params as hidden form fields
    let fields = format!(
        r#"<input type="hidden" name="client_id" value="{cid}">
           <input type="hidden" name="scope" value="{scope}">
           <input type="hidden" name="state" value="{state}">
           <input type="hidden" name="redirect_uri" value="{redirect}">"#,
        cid = client_name,
        scope = scope,
        state = state,
        redirect = redirect,
    );

    Html(format!(
        r#"<!DOCTYPE html>
<html>
<head><title>Authorize Application</title>
<style>
  body {{ font-family: system-ui, sans-serif; display: flex; justify-content: center;
         align-items: center; min-height: 100vh; margin: 0; background: #1e1f22; color: #dbdee1; }}
  .card {{ background: #2b2d31; border-radius: 8px; padding: 32px; max-width: 440px; width: 100%; }}
  h1 {{ font-size: 24px; margin: 0 0 16px; }}
  p {{ color: #949ba4; margin: 0 0 24px; line-height: 1.5; }}
  .scopes {{ background: #1e1f22; border-radius: 4px; padding: 12px; margin-bottom: 24px; }}
  .scope-item {{ display: flex; align-items: center; gap: 8px; padding: 4px 0; }}
  .scope-item::before {{ content: "✓"; color: #23a55a; font-weight: bold; }}
  .buttons {{ display: flex; gap: 12px; justify-content: flex-end; }}
  button {{ padding: 10px 20px; border-radius: 4px; border: none; cursor: pointer; font-size: 14px; font-weight: 500; }}
  .btn-authorize {{ background: #5865f2; color: white; }}
  .btn-authorize:hover {{ background: #4752c4; }}
  .btn-cancel {{ background: #4e5058; color: #dbdee1; }}
  .btn-cancel:hover {{ background: #6d6f78; }}
</style>
</head>
<body>
<div class="card">
  <h1>Authorize {name}</h1>
  <p>{name} wants to access your OpenCorde account.</p>
  <div class="scopes">
    <div class="scope-item">{scope_desc}</div>
  </div>
  <form method="post">
    {fields}
    <div class="buttons">
      <button type="submit" name="approved" value="0" class="btn-cancel">Cancel</button>
      <button type="submit" name="approved" value="1" class="btn-authorize">Authorize</button>
    </div>
  </form>
</div>
</body>
</html>"#,
        name = client_name,
        scope_desc = scope_display(scope),
        fields = fields,
    ))
}

fn scope_display(scope: &str) -> &str {
    match scope {
        "identify" => "Read your username and avatar",
        "email" => "Read your email address",
        "guilds" => "Read what servers you're in",
        "identify email" => "Read your username, avatar, and email address",
        _ => scope,
    }
}

/// POST /api/v1/oauth2/authorize
///
/// Handles the user's approval decision. If approved, generates a one-time
/// authorization code and redirects the browser to the client's redirect_uri.
pub async fn approve(
    State(state): State<AppState>,
    auth: AuthUser,
    axum::extract::Form(body): axum::extract::Form<ApproveBody>,
) -> Result<Redirect, Html<String>> {
    if !body.approved {
        return Err(Html(
            "<h1>Denied</h1><p>Authorization was not granted.</p>".into(),
        ));
    }

    // Parse client_id
    let application_id: i64 = body.client_id.parse().map_err(|_| {
        Html("<h1>Error</h1><p>Invalid client ID.</p>".into())
    })?;
    let app_id = opencorde_core::snowflake::Snowflake::new(application_id);

    // Verify the application exists
    let app = repos::app_repo::get_by_id(&state.db, app_id)
        .await
        .map_err(|e| {
            tracing::error!(?e, "failed to look up application");
            Html("<h1>Error</h1><p>Failed to look up application.</p>".into())
        })?;

    if app.is_none() {
        return Err(Html("<h1>Error</h1><p>Unknown application.</p>".into()));
    }

    // Generate authorization code
    let scope = body.scope.clone();
    let redirect_uri = body.redirect_uri.clone();

    let mut generator = SnowflakeGenerator::new(10, 0);
    let code_id = generator.next_id();

    let (plaintext_code, _row) = repos::oauth_auth_code_repo::create_auth_code(
        &state.db,
        code_id,
        app_id,
        auth.user_id,
        &redirect_uri,
        &scope,
        10, // 10-minute TTL
    )
    .await
    .map_err(|e| {
        tracing::error!(?e, "failed to create auth code");
        Html("<h1>Error</h1><p>Failed to generate authorization code.</p>".into())
    })?;

    // Build redirect URL
    let redirect_url = if body.state.is_empty() {
        format!("{}?code={}", redirect_uri, plaintext_code)
    } else {
        format!(
            "{}?code={}&state={}",
            redirect_uri, plaintext_code, body.state
        )
    };

    Ok(Redirect::to(&redirect_url))
}
