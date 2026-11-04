use super::*;
use serde_json::{Value, json};

fn channel_events(id: &str) -> Vec<Value> {
    let mut events = vec![];
    for kind in ["MessageCreate", "MessageUpdate"] {
        events.push(
            json!({"type": kind, "data": {"message": {"channel_id": id, "content": "private"}}}),
        );
    }
    for kind in [
        "MessageDelete",
        "TypingStart",
        "ReactionAdd",
        "ReactionRemove",
        "ChannelDelete",
    ] {
        events.push(json!({"type": kind, "data": {"channel_id": id, "server_id": "10"}}));
    }
    for kind in ["ChannelCreate", "ChannelUpdate"] {
        events.push(json!({"type": kind, "data": {"channel": {"id": id, "server_id": "10", "name": "private"}}}));
    }
    events
}

async fn expect_channels(pool: &PgPool, user: i64, expected: bool) {
    for event in channel_events("20") {
        assert_eq!(
            should_dispatch(&event, pool, Snowflake::new(user)).await,
            expected,
            "{event}"
        );
    }
}

#[tokio::test]
async fn malformed_events_and_lookup_failures_deny_and_ack_is_private() {
    // A closed lazy pool proves failures deny without requiring a running DB.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1/unused")
        .unwrap();
    pool.close().await;
    for event in channel_events("20") {
        assert!(!should_dispatch(&event, &pool, Snowflake::new(2)).await);
    }
    for kind in [
        "MessageCreate",
        "MessageUpdate",
        "MessageDelete",
        "TypingStart",
        "ReactionAdd",
        "ReactionRemove",
        "ChannelCreate",
        "ChannelUpdate",
        "ChannelDelete",
        "ChannelAck",
        "RoleUpdate",
        "Unknown",
        "DmMessageCreate",
    ] {
        assert!(
            !should_dispatch(&json!({"type": kind, "data": {}}), &pool, Snowflake::new(2)).await
        );
    }
    for event in channel_events("invalid") {
        assert_eq!(audience(&event), Audience::Deny);
    }
    let ack = json!({"type": "ChannelAck", "data": {"user_id": "2", "channel_id": "20"}});
    assert!(should_dispatch(&ack, &pool, Snowflake::new(2)).await);
    assert!(!should_dispatch(&ack, &pool, Snowflake::new(3)).await);
}

// Run only against an EMPTY disposable PostgreSQL database, never application DB.
#[tokio::test]
#[ignore = "requires disposable PostgreSQL via OPENCORDE_TEST_DATABASE_URL"]
async fn current_permissions_gate_every_delivery() {
    let url = std::env::var("OPENCORDE_TEST_DATABASE_URL")
        .expect("set OPENCORDE_TEST_DATABASE_URL to an empty disposable database");
    let pool = PgPool::connect(&url).await.unwrap();
    sqlx::migrate!("../opencorde-db/migrations")
        .run(&pool)
        .await
        .unwrap();
    sqlx::raw_sql("INSERT INTO users(id,username,public_key) VALUES (1,'ws_owner','ws_owner'),(2,'ws_member','ws_member'),(3,'ws_outsider','ws_outsider');
        INSERT INTO servers(id,name,owner_id) VALUES (10,'ws_test',1);
        INSERT INTO server_members(user_id,server_id) VALUES (1,10),(2,10);
        INSERT INTO channels(id,server_id,name) VALUES (20,10,'private');
        INSERT INTO roles(id,server_id,name,permissions) VALUES (30,10,'reader',0);
        INSERT INTO member_roles(user_id,server_id,role_id) VALUES (2,10,30);
        INSERT INTO channel_permission_overrides(channel_id,target_type,target_id,deny_bits) VALUES (20,'role',10,1024);")
        .execute(&pool).await.unwrap();
    expect_channels(&pool, 2, false).await; // membership alone is insufficient
    expect_channels(&pool, 3, false).await;
    expect_channels(&pool, 1, true).await; // owner permission bypass preserved

    // Grant, revoke, and member-over-role precedence on the SAME session identity.
    sqlx::raw_sql("INSERT INTO channel_permission_overrides(channel_id,target_type,target_id,allow_bits) VALUES (20,'role',30,1024)")
        .execute(&pool).await.unwrap();
    expect_channels(&pool, 2, true).await;
    sqlx::raw_sql("INSERT INTO channel_permission_overrides(channel_id,target_type,target_id,deny_bits) VALUES (20,'member',2,1024)")
        .execute(&pool).await.unwrap();
    expect_channels(&pool, 2, false).await;
    sqlx::raw_sql("DELETE FROM channel_permission_overrides WHERE target_type='member'")
        .execute(&pool)
        .await
        .unwrap();
    expect_channels(&pool, 2, true).await;
    sqlx::raw_sql("DELETE FROM member_roles WHERE user_id=2")
        .execute(&pool)
        .await
        .unwrap();
    expect_channels(&pool, 2, false).await;
    sqlx::raw_sql("INSERT INTO member_roles(user_id,server_id,role_id) VALUES (2,10,30)")
        .execute(&pool)
        .await
        .unwrap();
    expect_channels(&pool, 2, true).await;
    sqlx::raw_sql("DELETE FROM server_members WHERE user_id=2")
        .execute(&pool)
        .await
        .unwrap();
    expect_channels(&pool, 2, false).await; // kicked socket loses access
    let role = json!({"type":"RoleUpdate", "data":{"server_id":"10"}});
    assert!(!should_dispatch(&role, &pool, Snowflake::new(2)).await);
    assert!(should_dispatch(&role, &pool, Snowflake::new(1)).await);

    sqlx::raw_sql("INSERT INTO dm_channels(id) VALUES (40); INSERT INTO dm_channel_members(dm_channel_id,user_id) VALUES (40,2)")
        .execute(&pool).await.unwrap();
    let dm = json!({"type":"DmMessageCreate", "data":{"message":{"dm_id":"40"}}});
    assert!(should_dispatch(&dm, &pool, Snowflake::new(2)).await);
    assert!(!should_dispatch(&dm, &pool, Snowflake::new(3)).await);
    sqlx::raw_sql("DELETE FROM dm_channel_members WHERE user_id=2")
        .execute(&pool)
        .await
        .unwrap();
    assert!(!should_dispatch(&dm, &pool, Snowflake::new(2)).await);

    sqlx::raw_sql("DELETE FROM channels WHERE id=20")
        .execute(&pool)
        .await
        .unwrap();
    expect_channels(&pool, 1, false).await; // deleted ACL cannot authorize even owner
    pool.close().await;
    expect_channels(&pool, 1, false).await;
}
