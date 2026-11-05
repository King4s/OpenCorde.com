//! Ordered public MLS relay. Private cryptographic state never enters this module.
use crate::{
    AppState,
    error::ApiError,
    middleware::auth::AuthUser,
    routes::{helpers, permission_check},
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use opencorde_core::{
    permissions::Permissions,
    snowflake::{Snowflake, SnowflakeGenerator},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::Row;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Added {
    pub user_id: String,
    pub welcome: String,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub operation_id: String,
    pub expected_sequence: String,
    pub epoch: String,
    pub kind: String,
    pub ciphertext: String,
    #[serde(default)]
    pub added: Vec<Added>,
    #[serde(default)]
    pub removed: Vec<String>,
    #[serde(default)]
    pub reply_to_id: Option<String>,
    #[serde(default)]
    pub attachments: Option<Value>,
    #[serde(default)]
    pub allow_legacy_rejoin: bool,
}
#[derive(Deserialize)]
pub struct Page {
    #[serde(default)]
    after: Option<String>,
    limit: Option<i64>,
}
#[derive(Deserialize)]
pub struct Ack {
    operation_id: String,
}
fn number(value: &str) -> Result<i64, ApiError> {
    value
        .parse::<i64>()
        .ok()
        .filter(|n| *n >= 0)
        .ok_or_else(|| ApiError::BadRequest("invalid sequence or epoch".into()))
}
fn decode(value: &str) -> Result<Vec<u8>, ApiError> {
    if value.len() > 262144 {
        return Err(ApiError::BadRequest("MLS payload exceeds limit".into()));
    }
    URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| ApiError::BadRequest("invalid base64url".into()))
}
fn conflict() -> ApiError {
    ApiError::Conflict("MLS stream changed; reconcile pending operation and catch up".into())
}
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/channels/{channel_id}/e2ee/v2/events",
            get(events).post(submit),
        )
        .route(
            "/api/v1/channels/{channel_id}/e2ee/v2/welcome",
            get(welcome),
        )
        .route(
            "/api/v1/channels/{channel_id}/e2ee/v2/welcome/ack",
            post(ack),
        )
}
async fn access(
    pool: &sqlx::PgPool,
    user: Snowflake,
    channel: Snowflake,
    send: bool,
) -> Result<(), ApiError> {
    let perms = if send {
        Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES
    } else {
        Permissions::VIEW_CHANNEL
    };
    permission_check::require_channel_perm(pool, user, channel, perms).await?;
    let server: i64 = sqlx::query_scalar("SELECT server_id FROM channels WHERE id=$1")
        .bind(channel.as_i64())
        .fetch_one(pool)
        .await?;
    if send {
        permission_check::require_not_timed_out(pool, user, Snowflake::new(server)).await?;
    }
    Ok(())
}
pub async fn require_active(
    pool: &sqlx::PgPool,
    user: Snowflake,
    channel: Snowflake,
) -> Result<(), ApiError> {
    let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM e2ee_stream_members WHERE channel_id=$1 AND user_id=$2 AND active)")
 .bind(channel.as_i64()).bind(user.as_i64()).fetch_one(pool).await?;
    if active {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
}
pub async fn is_v2(pool: &sqlx::PgPool, channel: Snowflake) -> Result<bool, ApiError> {
    Ok(
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM e2ee_streams WHERE channel_id=$1)")
            .bind(channel.as_i64())
            .fetch_one(pool)
            .await?,
    )
}
pub async fn reject_legacy(pool: &sqlx::PgPool, channel: Snowflake) -> Result<(), ApiError> {
    if is_v2(pool, channel).await? {
        return Err(ApiError::Conflict(
            "this channel requires the updated encrypted client".into(),
        ));
    }
    Ok(())
}
async fn submit(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(op): Json<Operation>,
) -> Result<Json<Value>, ApiError> {
    let channel = helpers::parse_snowflake(&id)?;
    let event = submit_for_user(&state.db, auth.user_id, channel, op).await?;
    let _ = state
        .event_tx
        .send(json!({"type":"E2eeSync","data":{"channel_id":id,"sequence":event["sequence"]}}));
    Ok(Json(event))
}
pub async fn submit_for_user(
    pool: &sqlx::PgPool,
    user: Snowflake,
    channel: Snowflake,
    op: Operation,
) -> Result<Value, ApiError> {
    access(pool, user, channel, true).await?;
    if op.operation_id.len() < 16
        || op.operation_id.len() > 128
        || !op
            .operation_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(ApiError::BadRequest("invalid operation_id".into()));
    }
    let expected = number(&op.expected_sequence)?;
    let epoch = number(&op.epoch)?;
    let bytes = decode(&op.ciphertext)?;
    if op.added.len() > 100
        || op.removed.len() > 100
        || serde_json::to_vec(&op)
            .map_err(|e| ApiError::Internal(e.into()))?
            .len()
            > 1048576
    {
        return Err(ApiError::BadRequest("MLS operation exceeds limits".into()));
    }
    if !matches!(op.kind.as_str(), "init" | "commit" | "application") {
        return Err(ApiError::BadRequest("invalid MLS operation kind".into()));
    }
    if op.kind != "application" && (op.reply_to_id.is_some() || op.attachments.is_some()) {
        return Err(ApiError::BadRequest(
            "only applications carry message metadata".into(),
        ));
    }
    if op.kind != "commit" && (!op.added.is_empty() || !op.removed.is_empty()) {
        return Err(ApiError::BadRequest(
            "only commits change membership".into(),
        ));
    }
    if op.kind == "init" && (!bytes.is_empty() || epoch != 0 || expected != 0) {
        return Err(ApiError::BadRequest("invalid initial MLS operation".into()));
    }
    if op.kind != "init" && bytes.is_empty() {
        return Err(ApiError::BadRequest("empty MLS payload".into()));
    }
    let mut add = Vec::new();
    let mut remove = Vec::new();
    for a in &op.added {
        let id = helpers::parse_snowflake(&a.user_id)?;
        if id == user || add.iter().any(|(other, _)| *other == id.as_i64()) {
            return Err(ApiError::BadRequest("duplicate or self addition".into()));
        }
        let welcome = decode(&a.welcome)?;
        if welcome.is_empty() {
            return Err(ApiError::BadRequest("empty Welcome".into()));
        }
        add.push((id.as_i64(), welcome));
    }
    for id in &op.removed {
        let id = helpers::parse_snowflake(id)?.as_i64();
        if id == user.as_i64() || remove.contains(&id) || add.iter().any(|(a, _)| *a == id) {
            return Err(ApiError::BadRequest("invalid removal".into()));
        }
        remove.push(id);
    }
    let request = serde_json::to_value(&op).map_err(|e| ApiError::Internal(e.into()))?;
    let mut tx = pool.begin().await?;
    // Serialize initialization with other channel writes, including concurrent initializers.
    let channel_row = sqlx::query(
        "SELECT e2ee_enabled,server_id,slowmode_delay FROM channels WHERE id=$1 FOR UPDATE",
    )
    .bind(channel.as_i64())
    .fetch_one(&mut *tx)
    .await?;
    if !channel_row.get::<bool, _>("e2ee_enabled") {
        return Err(ApiError::BadRequest(
            "channel encryption is disabled".into(),
        ));
    }
    if op.kind == "init" {
        let legacy: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM e2ee_groups WHERE channel_id=$1)")
                .bind(channel.as_i64())
                .fetch_one(&mut *tx)
                .await?;
        if legacy && !op.allow_legacy_rejoin {
            return Err(ApiError::Conflict(
                "legacy encrypted channel requires explicit rejoin by all members".into(),
            ));
        }
        sqlx::query("INSERT INTO e2ee_streams(channel_id) VALUES($1) ON CONFLICT DO NOTHING")
            .bind(channel.as_i64())
            .execute(&mut *tx)
            .await?;
    }
    let stream =
        sqlx::query("SELECT sequence,epoch FROM e2ee_streams WHERE channel_id=$1 FOR UPDATE")
            .bind(channel.as_i64())
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| ApiError::NotFound("MLS stream not initialized".into()))?;
    let sequence: i64 = stream.get("sequence");
    let current_epoch: i64 = stream.get("epoch");
    if sequence > 0 {
        let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM e2ee_stream_members WHERE channel_id=$1 AND user_id=$2 AND active)").bind(channel.as_i64()).bind(user.as_i64()).fetch_one(&mut *tx).await?;
        if !active {
            return Err(ApiError::Forbidden);
        }
    }
    authorization::require(
        &mut tx,
        user,
        channel,
        Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES,
        false,
    )
    .await?;
    if let Some(old) = sqlx::query(
        "SELECT sender_id,request,event FROM e2ee_events WHERE channel_id=$1 AND operation_id=$2",
    )
    .bind(channel.as_i64())
    .bind(&op.operation_id)
    .fetch_optional(&mut *tx)
    .await?
    {
        if old.get::<i64, _>("sender_id") != user.as_i64()
            || old.get::<Value, _>("request") != request
        {
            return Err(conflict());
        }
        return Ok(old.get("event"));
    }
    let mut required = Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES;
    if op.kind == "init"
        || (op.kind == "commit" && (!op.added.is_empty() || !op.removed.is_empty()))
    {
        required |= Permissions::MANAGE_CHANNELS;
    }
    authorization::require(&mut tx, user, channel, required, true).await?;
    if expected != sequence || epoch != current_epoch || (op.kind == "init" && sequence != 0) {
        return Err(conflict());
    }
    for (id, _) in &add {
        authorization::require(
            &mut tx,
            Snowflake::new(*id),
            channel,
            Permissions::VIEW_CHANNEL,
            false,
        )
        .await?;
        let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM e2ee_stream_members WHERE channel_id=$1 AND user_id=$2 AND active)").bind(channel.as_i64()).bind(id).fetch_one(&mut *tx).await?;
        if active {
            return Err(ApiError::Conflict("member already active".into()));
        }
    }
    for id in &remove {
        let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM e2ee_stream_members WHERE channel_id=$1 AND user_id=$2 AND active)").bind(channel.as_i64()).bind(id).fetch_one(&mut *tx).await?;
        if !active {
            return Err(ApiError::Conflict("member not active".into()));
        }
    }
    let next = sequence.checked_add(1).ok_or_else(conflict)?;
    let next_epoch = if op.kind == "commit" {
        epoch.checked_add(1).ok_or_else(conflict)?
    } else {
        epoch
    };
    if op.kind == "init" {
        sqlx::query(
            "INSERT INTO e2ee_stream_members(channel_id,user_id,joined_sequence) VALUES($1,$2,$3)",
        )
        .bind(channel.as_i64())
        .bind(user.as_i64())
        .bind(next)
        .execute(&mut *tx)
        .await?;
    }
    for (id, welcome) in &add {
        sqlx::query("INSERT INTO e2ee_stream_members(channel_id,user_id,joined_sequence,welcome_operation,welcome_sequence,welcome_epoch,welcome) VALUES($1,$2,$3,$4,$3,$5,$6) ON CONFLICT(channel_id,user_id) DO UPDATE SET active=TRUE,joined_sequence=$3,welcome_operation=$4,welcome_sequence=$3,welcome_epoch=$5,welcome=$6")
 .bind(channel.as_i64()).bind(id).bind(next).bind(&op.operation_id).bind(next_epoch).bind(welcome).execute(&mut *tx).await?;
    }
    for id in &remove {
        sqlx::query("UPDATE e2ee_stream_members SET active=FALSE,welcome=NULL WHERE channel_id=$1 AND user_id=$2").bind(channel.as_i64()).bind(id).execute(&mut *tx).await?;
    }
    let message_id = if op.kind == "application" {
        let delay: i32 = channel_row.get("slowmode_delay");
        if delay > 0 {
            let last:Option<chrono::DateTime<chrono::Utc>>=sqlx::query_scalar("SELECT created_at FROM messages WHERE channel_id=$1 AND author_id=$2 ORDER BY created_at DESC LIMIT 1").bind(channel.as_i64()).bind(user.as_i64()).fetch_optional(&mut *tx).await?;
            if let Some(last) = last {
                let remaining =
                    i64::from(delay) - chrono::Utc::now().signed_duration_since(last).num_seconds();
                if remaining > 0 {
                    return Err(ApiError::RateLimited {
                        retry_after: remaining as u64,
                    });
                }
            }
        }

        static IDS: std::sync::OnceLock<std::sync::Mutex<SnowflakeGenerator>> =
            std::sync::OnceLock::new();
        let id = IDS
            .get_or_init(|| std::sync::Mutex::new(SnowflakeGenerator::new(3, 1)))
            .lock()
            .map_err(|_| ApiError::InternalServerError("ID generator unavailable".into()))?
            .next_id();
        let reply = op
            .reply_to_id
            .as_deref()
            .map(helpers::parse_snowflake)
            .transpose()?;
        if let Some(reply) = reply {
            let same: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM messages WHERE id=$1 AND channel_id=$2)",
            )
            .bind(reply.as_i64())
            .bind(channel.as_i64())
            .fetch_one(&mut *tx)
            .await?;
            if !same {
                return Err(ApiError::BadRequest(
                    "reply target is not in this channel".into(),
                ));
            }
        }
        let attachments = op.attachments.clone().unwrap_or_else(|| json!([]));
        if !attachments.is_array() || attachments.as_array().is_some_and(|a| a.len() > 10) {
            return Err(ApiError::BadRequest("invalid attachments".into()));
        }
        let content = format!(
            "enc:{}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        );
        sqlx::query("SELECT set_config('opencorde.mls_channel',$1,true)")
            .bind(channel.to_string())
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO messages(id,channel_id,author_id,content,attachments,reply_to_id) VALUES($1,$2,$3,$4,$5,$6)")
 .bind(id.as_i64()).bind(channel.as_i64()).bind(user.as_i64()).bind(content).bind(attachments).bind(reply.map(|v|v.as_i64())).execute(&mut *tx).await?;
        Some(id.to_string())
    } else {
        None
    };
    let event = json!({"operation_id":op.operation_id,"sequence":next.to_string(),"epoch":next_epoch.to_string(),"sender_id":user.to_string(),"kind":op.kind,"ciphertext":op.ciphertext,"added":op.added.iter().map(|a|a.user_id.clone()).collect::<Vec<_>>(),"removed":op.removed,"message_id":message_id,"reply_to_id":op.reply_to_id,"attachments":op.attachments});
    sqlx::query("INSERT INTO e2ee_events(channel_id,sequence,operation_id,sender_id,request,event) VALUES($1,$2,$3,$4,$5,$6)")
 .bind(channel.as_i64()).bind(next).bind(&op.operation_id).bind(user.as_i64()).bind(request).bind(&event).execute(&mut *tx).await?;
    sqlx::query("UPDATE e2ee_streams SET sequence=$2,epoch=$3 WHERE channel_id=$1")
        .bind(channel.as_i64())
        .bind(next)
        .bind(next_epoch)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(event)
}
async fn events(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Query(page): Query<Page>,
) -> Result<Json<Value>, ApiError> {
    events_for_user(
        &state.db,
        auth.user_id,
        helpers::parse_snowflake(&id)?,
        page,
    )
    .await
    .map(Json)
}
async fn events_for_user(
    pool: &sqlx::PgPool,
    user: Snowflake,
    channel: Snowflake,
    page: Page,
) -> Result<Value, ApiError> {
    access(pool, user, channel, false).await?;
    require_active(pool, user, channel).await?;
    let after = number(page.after.as_deref().unwrap_or("0"))?;
    let limit = page.limit.unwrap_or(100).clamp(1, 100);
    let mut tx = pool.begin().await?;
    let row=sqlx::query("SELECT s.sequence,s.epoch,m.joined_sequence FROM e2ee_streams s JOIN e2ee_stream_members m USING(channel_id) WHERE channel_id=$1 AND user_id=$2 AND active FOR SHARE OF s,m")
 .bind(channel.as_i64()).bind(user.as_i64()).fetch_optional(&mut *tx).await?.ok_or(ApiError::Forbidden)?;
    let sequence: i64 = row.get("sequence");
    let epoch: i64 = row.get("epoch");
    let joined: i64 = row.get("joined_sequence");
    let values:Vec<Value>=sqlx::query_scalar("SELECT event FROM e2ee_events WHERE channel_id=$1 AND sequence>$2 ORDER BY sequence LIMIT $3").bind(channel.as_i64()).bind(after.max(joined)).bind(limit+1).fetch_all(&mut *tx).await?;
    let more = values.len() > limit as usize;
    let events: Vec<_> = values.into_iter().take(limit as usize).collect();
    Ok(
        json!({"events":events,"sequence":sequence.to_string(),"epoch":epoch.to_string(),"has_more":more}),
    )
}
async fn welcome(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    welcome_for_user(&state.db, auth.user_id, helpers::parse_snowflake(&id)?)
        .await
        .map(Json)
}
async fn welcome_for_user(
    pool: &sqlx::PgPool,
    user: Snowflake,
    channel: Snowflake,
) -> Result<Value, ApiError> {
    access(pool, user, channel, false).await?;
    let row=sqlx::query("SELECT welcome_operation,welcome_sequence,welcome_epoch,welcome FROM e2ee_stream_members WHERE channel_id=$1 AND user_id=$2 AND active AND welcome IS NOT NULL").bind(channel.as_i64()).bind(user.as_i64()).fetch_optional(pool).await?.ok_or_else(||ApiError::NotFound("no pending Welcome".into()))?;
    Ok(
        json!({"operation_id":row.get::<String,_>("welcome_operation"),"sequence":row.get::<i64,_>("welcome_sequence").to_string(),"epoch":row.get::<i64,_>("welcome_epoch").to_string(),"welcome":URL_SAFE_NO_PAD.encode(row.get::<Vec<u8>,_>("welcome"))}),
    )
}
async fn ack(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(ack): Json<Ack>,
) -> Result<Json<Value>, ApiError> {
    ack_for_user(
        &state.db,
        auth.user_id,
        helpers::parse_snowflake(&id)?,
        &ack.operation_id,
    )
    .await?;
    Ok(Json(json!({"acknowledged":true})))
}
async fn ack_for_user(
    pool: &sqlx::PgPool,
    user: Snowflake,
    channel: Snowflake,
    operation: &str,
) -> Result<(), ApiError> {
    access(pool, user, channel, false).await?;
    let n=sqlx::query("UPDATE e2ee_stream_members SET welcome=NULL WHERE channel_id=$1 AND user_id=$2 AND active AND welcome_operation=$3").bind(channel.as_i64()).bind(user.as_i64()).bind(operation).execute(pool).await?.rows_affected();
    if n != 1 {
        return Err(conflict());
    }
    Ok(())
}
#[cfg(test)]
mod tests;

mod authorization;
