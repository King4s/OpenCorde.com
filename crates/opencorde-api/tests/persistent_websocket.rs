//! Real gateway sessions against disposable PostgreSQL; no live services/accounts.
use std::{sync::Arc, time::Duration};

use futures::{SinkExt, StreamExt};
use opencorde_api::{
    AppState,
    config::{Config, Environment, RegistrationMode},
    email::EmailService,
    identity::ServerIdentity,
    jwt,
    middleware::rate_limit::{RateLimitConfig, RateLimitState},
    routes, ws,
};
use opencorde_core::Snowflake;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::{net::TcpStream, sync::broadcast};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;
const SECRET: &str = "disposable-ws-test-signing-secret-not-for-deployment";

fn config() -> Config {
    // Do not load developer/live environment configuration, credentials or endpoints.
    Config {
        allow_external_integrations: false,
        public_registry_sync: false,
        database_url: String::new(),
        redis_url: String::new(),
        api_host: "127.0.0.1".into(),
        api_port: 0,
        jwt_secret: SECRET.into(),
        jwt_access_expiry: 300,
        jwt_refresh_expiry: 300,
        minio_endpoint: "http://127.0.0.1:1".into(),
        files_public_url: String::new(),
        minio_access_key: "unused".into(),
        minio_secret_key: "unused".into(),
        minio_bucket: "unused".into(),
        livekit_url: String::new(),
        livekit_public_url: String::new(),
        livekit_api_key: String::new(),
        livekit_api_secret: String::new(),
        environment: Environment::Development,
        mesh_hostname: "ws-test.invalid".into(),
        admin_user_ids: vec![],
        smtp_host: None,
        smtp_port: 25,
        smtp_username: None,
        smtp_password: None,
        smtp_from: "unused@ws-test.invalid".into(),
        base_url: String::new(),
        registration_mode: RegistrationMode::Closed,
        rate_limit_rps: 100,
        rate_limit_burst: 200,
        trusted_proxy_ip: None,
        steam_api_key: None,
        fcm_server_key: None,
        registration_invite_code: None,
        search_index_path: None,
        vapid_private_key: None,
    }
}

#[test]
fn privacy_master_switch_overrides_registry_opt_in() {
    let mut config = config();
    for (master, registry, expected) in [
        (false, false, false),
        (false, true, false),
        (true, false, false),
        (true, true, true),
    ] {
        config.allow_external_integrations = master;
        config.public_registry_sync = registry;
        assert_eq!(config.registry_sync_enabled(), expected);
        assert_eq!(config.require_external_integrations().is_ok(), master);
    }
}

#[tokio::test]
async fn disabled_push_does_not_connect_to_database() {
    let config = config();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    tokio::time::timeout(
        Duration::from_millis(500),
        opencorde_api::push_sender::send_push(&pool, &config, 1, "fixture", "fixture"),
    )
    .await
    .expect("disabled push must return before database or network access");
}

fn token(user: i64) -> String {
    jwt::create_access_token(Snowflake::new(user), "ws_fixture", SECRET, 300).unwrap()
}

async fn receive(socket: &mut Socket) -> Value {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match socket
                .next()
                .await
                .expect("gateway unexpectedly closed")
                .unwrap()
            {
                Message::Text(text) => {
                    let value: Value = serde_json::from_str(&text).unwrap();
                    if value["type"] == "Heartbeat" {
                        socket
                            .send(Message::Text(
                                json!({"type":"HeartbeatAck"}).to_string().into(),
                            ))
                            .await
                            .unwrap();
                    } else {
                        return value;
                    }
                }
                Message::Ping(bytes) => socket.send(Message::Pong(bytes)).await.unwrap(),
                Message::Pong(_) => {}
                other => panic!("unexpected gateway frame: {other:?}"),
            }
        }
    })
    .await
    .expect("gateway did not reach the next event/barrier")
}

async fn connect(url: &str, user: i64) -> Socket {
    let (mut socket, _) = connect_async(url).await.unwrap();
    assert_eq!(receive(&mut socket).await["type"], "Hello");
    socket
        .send(Message::Text(
            json!({"type":"Identify","data":{"token":token(user)}})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    let ready = receive(&mut socket).await;
    assert_eq!(ready["type"], "Ready");
    assert_eq!(ready["data"]["user"]["id"], user.to_string());
    // READY precedes subscription: our own presence proves the main loop subscribed.
    let presence = receive(&mut socket).await;
    assert_eq!(presence["type"], "PresenceUpdate");
    assert_eq!(presence["data"]["user_id"], user.to_string());
    socket
}

async fn assert_delivery(
    tx: &broadcast::Sender<Value>,
    sockets: &mut [Socket; 3],
    events: &[Value],
    allowed: [bool; 3],
    phase: &str,
) {
    // Broadcast FIFO + a public sentinel proves all preceding events were processed.
    // Absence therefore does not depend on a short sleep or a timing window.
    let barrier = json!({"type":"PresenceUpdate","data":{"user_id":"2001","online":true,"test_barrier":uuid::Uuid::new_v4().to_string()}});
    tx.send(barrier.clone()).unwrap();
    for (index, socket) in sockets.iter_mut().enumerate() {
        let mut received = vec![];
        loop {
            let event = receive(socket).await;
            if event == barrier {
                break;
            }
            // Other connections' initial online presence may still be queued.
            if event["type"] != "PresenceUpdate" {
                received.push(event);
            }
        }
        let expected = if allowed[index] {
            events.to_vec()
        } else {
            vec![]
        };
        assert_eq!(
            received, expected,
            "{phase}: socket {index} (owner/member/outsider)"
        );
    }
}

fn channel_events(channel: &str) -> Vec<Value> {
    let mut events = vec![];
    for kind in ["MessageCreate", "MessageUpdate"] {
        events.push(json!({"type":kind,"data":{"message":{"id":"2050","channel_id":channel,"content":"private"}}}));
    }
    for kind in [
        "MessageDelete",
        "TypingStart",
        "ReactionAdd",
        "ReactionRemove",
        "ChannelDelete",
    ] {
        events.push(json!({"type":kind,"data":{"channel_id":channel,"server_id":"2010"}}));
    }
    for kind in ["ChannelCreate", "ChannelUpdate"] {
        events.push(json!({"type":kind,"data":{"channel":{"id":channel,"server_id":"2010","name":"private"}}}));
    }
    events
}

async fn publish_channels(
    tx: &broadcast::Sender<Value>,
    sockets: &mut [Socket; 3],
    channel: &str,
    allowed: [bool; 3],
    phase: &str,
) {
    let events = channel_events(channel);
    for event in &events {
        tx.send(event.clone()).unwrap();
    }
    assert_delivery(tx, sockets, &events, allowed, phase).await;
}

async fn execute(pool: &PgPool, sql: &str) {
    sqlx::raw_sql(sql).execute(pool).await.unwrap();
}

async fn run(identity: ServerIdentity) {
    let url = std::env::var("OPENCORDE_TEST_DATABASE_URL")
        .expect("set OPENCORDE_TEST_DATABASE_URL to an empty disposable PostgreSQL database");
    let pool = PgPool::connect(&url).await.unwrap();
    sqlx::migrate!("../opencorde-db/migrations")
        .run(&pool)
        .await
        .unwrap();
    execute(&pool, "INSERT INTO users(id,username,public_key) VALUES (2001,'network_owner','network_owner'),(2002,'network_member','network_member'),(2003,'network_outsider','network_outsider');
        INSERT INTO servers(id,name,owner_id) VALUES (2010,'network_test',2001);
        INSERT INTO server_members(user_id,server_id) VALUES (2001,2010),(2002,2010);
        INSERT INTO channels(id,server_id,name) VALUES (2020,2010,'network_private');
        INSERT INTO roles(id,server_id,name,permissions) VALUES (2030,2010,'reader',0);
        INSERT INTO member_roles(user_id,server_id,role_id) VALUES (2002,2010,2030);
        INSERT INTO channel_permission_overrides(channel_id,target_type,target_id,deny_bits) VALUES (2020,'role',2010,1024);").await;

    let (tx, _) = broadcast::channel(128);
    // Explicit static config prevents AWS credential/metadata discovery or network IO.
    let s3_config = aws_sdk_s3::config::Builder::new()
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new("us-east-1"))
        .credentials_provider(aws_sdk_s3::config::Credentials::new(
            "unused", "unused", None, None, "test",
        ))
        .endpoint_url("http://127.0.0.1:1")
        .build();
    let state = AppState {
        db: pool.clone(),
        config: Arc::new(config()),
        search: None,
        s3: Arc::new(aws_sdk_s3::Client::from_conf(s3_config)),
        email_service: EmailService::new(
            None,
            25,
            None,
            None,
            "unused@ws-test.invalid".into(),
            String::new(),
        ),
        event_tx: Arc::new(tx.clone()),
        identity: Arc::new(identity),
        unfurl_cache: routes::unfurl::new_cache(),
        rate_limit_state: RateLimitState::new(RateLimitConfig {
            requests_per_second: 100,
            burst_size: 200,
            enabled: true,
        }),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = ws::handler::router()
        .merge(routes::dms::router())
        .with_state(state);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let gateway = format!("ws://{address}/api/v1/gateway");
    let mut sockets = [
        connect(&gateway, 2001).await,
        connect(&gateway, 2002).await,
        connect(&gateway, 2003).await,
    ];

    publish_channels(
        &tx,
        &mut sockets,
        "2020",
        [true, false, false],
        "initial deny",
    )
    .await;
    execute(&pool,"INSERT INTO channel_permission_overrides(channel_id,target_type,target_id,allow_bits) VALUES (2020,'role',2030,1024)").await;
    publish_channels(
        &tx,
        &mut sockets,
        "2020",
        [true, true, false],
        "grant without reconnect",
    )
    .await;
    execute(&pool,"INSERT INTO channel_permission_overrides(channel_id,target_type,target_id,deny_bits) VALUES (2020,'member',2002,1024)").await;
    publish_channels(
        &tx,
        &mut sockets,
        "2020",
        [true, false, false],
        "member deny overrides role grant",
    )
    .await;
    execute(&pool,"DELETE FROM channel_permission_overrides WHERE channel_id=2020 AND target_type='member' AND target_id=2002").await;
    publish_channels(
        &tx,
        &mut sockets,
        "2020",
        [true, true, false],
        "restore grant",
    )
    .await;
    execute(
        &pool,
        "DELETE FROM member_roles WHERE user_id=2002 AND server_id=2010",
    )
    .await;
    publish_channels(
        &tx,
        &mut sockets,
        "2020",
        [true, false, false],
        "role revoked",
    )
    .await;
    execute(&pool,"INSERT INTO member_roles(user_id,server_id,role_id) VALUES (2002,2010,2030);
        INSERT INTO channels(id,server_id,name) VALUES (2021,2010,'created_after_connect');
        INSERT INTO channel_permission_overrides(channel_id,target_type,target_id,deny_bits) VALUES (2021,'role',2010,1024);
        INSERT INTO channel_permission_overrides(channel_id,target_type,target_id,allow_bits) VALUES (2021,'role',2030,1024);").await;
    publish_channels(
        &tx,
        &mut sockets,
        "2021",
        [true, true, false],
        "channel created after connect",
    )
    .await;
    execute(
        &pool,
        "DELETE FROM server_members WHERE user_id=2002 AND server_id=2010",
    )
    .await;
    publish_channels(
        &tx,
        &mut sockets,
        "2020",
        [true, false, false],
        "kick without reconnect",
    )
    .await;
    publish_channels(
        &tx,
        &mut sockets,
        "2021",
        [true, false, false],
        "kick also revokes new channel",
    )
    .await;

    for (index, user) in [2001, 2002, 2003].into_iter().enumerate() {
        let ack = json!({"type":"ChannelAck","data":{"user_id":user.to_string(),"channel_id":"2020","message_id":"2050"}});
        tx.send(ack.clone()).unwrap();
        let mut allowed = [false; 3];
        allowed[index] = true;
        assert_delivery(
            &tx,
            &mut sockets,
            &[ack],
            allowed,
            "ack only reaches its user",
        )
        .await;
    }

    // Create the DM AFTER connection too, then exercise the production HTTP emitter.
    execute(&pool,"INSERT INTO dm_channels(id) VALUES (2040); INSERT INTO dm_channel_members(dm_channel_id,user_id) VALUES (2040,2001),(2040,2002)").await;
    let http = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    let dm_url = format!("http://{address}/api/v1/channels/@dms/2040/messages");
    for (content, allowed) in [
        ("current member receives", [true, true, false]),
        ("removed member denied", [true, false, false]),
    ] {
        let response = http
            .post(&dm_url)
            .bearer_auth(token(2001))
            .json(&json!({"content":content}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::CREATED);
        let message: Value = response.json().await.unwrap();
        assert_eq!(message["dm_id"], "2040");
        assert_eq!(message["content"], content);
        let event = json!({"type":"DmMessageCreate","data":{"message":message}});
        assert_delivery(&tx, &mut sockets, &[event], allowed, content).await;
        execute(
            &pool,
            "DELETE FROM dm_channel_members WHERE dm_channel_id=2040 AND user_id=2002",
        )
        .await;
    }
    let forbidden = http
        .post(&dm_url)
        .bearer_auth(token(2003))
        .json(&json!({"content":"outsider must not send"}))
        .send()
        .await
        .unwrap();
    assert_eq!(forbidden.status(), reqwest::StatusCode::FORBIDDEN);
    assert_delivery(
        &tx,
        &mut sockets,
        &[],
        [false; 3],
        "rejected DM emits nothing",
    )
    .await;

    execute(&pool, "DELETE FROM channels WHERE id=2020").await;
    publish_channels(
        &tx,
        &mut sockets,
        "2020",
        [false; 3],
        "deleted channel fails closed",
    )
    .await;
    pool.close().await;
    publish_channels(
        &tx,
        &mut sockets,
        "2021",
        [false; 3],
        "database failure fails closed",
    )
    .await;
    let dm = json!({"type":"DmMessageCreate","data":{"message":{"dm_id":"2040","content":"database closed"}}});
    tx.send(dm.clone()).unwrap();
    assert_delivery(
        &tx,
        &mut sockets,
        &[dm],
        [false; 3],
        "DM lookup failure fails closed",
    )
    .await;
    for socket in &mut sockets {
        socket.close(None).await.unwrap();
    }
    server.abort();
    let _ = server.await;
}

// This integration binary has exactly one test. Configure the temporary identity
// before starting any async/runtime threads, and restore the environment immediately.
// No real identity is read or changed; all configuration above is explicit.
#[test]
#[ignore = "requires disposable PostgreSQL via OPENCORDE_TEST_DATABASE_URL"]
fn persistent_websocket_rechecks_current_authorization() {
    let path = std::env::temp_dir().join(format!("opencorde-ws-test-{}.key", uuid::Uuid::new_v4()));
    let previous = std::env::var_os("IDENTITY_KEY_PATH");
    // SAFETY: this binary has one test, and no runtime/network threads exist yet.
    unsafe {
        std::env::set_var("IDENTITY_KEY_PATH", &path);
    }
    let identity = ServerIdentity::load_or_generate();
    unsafe {
        match previous {
            Some(value) => std::env::set_var("IDENTITY_KEY_PATH", value),
            None => std::env::remove_var("IDENTITY_KEY_PATH"),
        }
    }
    std::fs::remove_file(&path).expect("remove disposable identity file");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(run(identity.unwrap()));
}
