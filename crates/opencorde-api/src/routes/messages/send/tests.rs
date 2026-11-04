use super::*;
use serde_json::Value;
use sqlx::{Connection, PgPool};

struct SendEffects {
    result: Result<MessageResponse, ApiError>,
    saved: i64,
    events: Vec<Value>,
}

impl SendEffects {
    fn allowed(self) {
        let response = self.result.expect("send should succeed");
        assert_eq!(self.saved, 1);
        assert_eq!(self.events.len(), 1);
        assert_eq!(self.events[0]["type"], "MessageCreate");
        assert_eq!(self.events[0]["data"]["message"]["id"], response.id);
    }

    fn forbidden(self) {
        assert!(matches!(self.result, Err(ApiError::Forbidden)));
        assert_eq!(self.saved, 0, "denial must precede message persistence");
        assert!(self.events.is_empty(), "denial must precede broadcast");
    }

    fn database_error(self) {
        assert!(matches!(self.result, Err(ApiError::Database(_))));
        assert_eq!(self.saved, 0, "lookup failure must fail closed");
        assert!(self.events.is_empty());
    }
}

async fn observe(pool: &PgPool, user_id: i64) -> SendEffects {
    // Only this fixture's channel; also avoids unrelated Snowflake generator
    // collisions between successful sends made within one millisecond.
    sqlx::query("DELETE FROM messages WHERE channel_id = 220")
        .execute(pool)
        .await
        .unwrap();
    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let result = persist_and_broadcast_message(
        pool,
        &tx,
        Snowflake::new(user_id),
        Snowflake::new(220),
        &SendMessageRequest {
            content: "timeout regression".into(),
            reply_to_id: None,
            attachments: None,
        },
    )
    .await
    .map(|(response, _, _, _)| response);
    let saved = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE channel_id = 220")
        .fetch_one(pool)
        .await
        .unwrap();
    let mut events = vec![];
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }
    SendEffects {
        result,
        saved,
        events,
    }
}

// Fixture IDs are disjoint from the WebSocket and notification regressions.
// Run only through the security runner's disposable PostgreSQL database.
#[tokio::test]
#[ignore = "requires disposable PostgreSQL via OPENCORDE_TEST_DATABASE_URL"]
async fn timeouts_gate_channel_persistence_and_events() {
    let url = std::env::var("OPENCORDE_TEST_DATABASE_URL")
        .expect("set OPENCORDE_TEST_DATABASE_URL to a disposable database");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    sqlx::migrate!("../opencorde-db/migrations")
        .run(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(
        "INSERT INTO users(id,username,public_key) VALUES
            (201,'timeout_owner','timeout_owner'),(202,'timeout_member','timeout_member'),
            (203,'timeout_admin','timeout_admin'),(204,'timeout_outsider','timeout_outsider');
         INSERT INTO servers(id,name,owner_id) VALUES (210,'timeout_test',201);
         INSERT INTO server_members(user_id,server_id) VALUES (201,210),(202,210),(203,210);
         INSERT INTO channels(id,server_id,name) VALUES (220,210,'timeout_channel');
         INSERT INTO roles(id,server_id,name,permissions) VALUES (230,210,'administrator',8);
         INSERT INTO member_roles(user_id,server_id,role_id) VALUES (203,210,230);
         INSERT INTO member_timeouts(server_id,user_id,timeout_until) VALUES
            (210,201,CURRENT_TIMESTAMP + INTERVAL '1 hour'),
            (210,202,CURRENT_TIMESTAMP + INTERVAL '1 hour'),
            (210,203,CURRENT_TIMESTAMP + INTERVAL '1 hour');",
    )
    .execute(&pool)
    .await
    .unwrap();

    observe(&pool, 202).await.forbidden(); // active ordinary-member timeout
    observe(&pool, 203).await.forbidden(); // ADMINISTRATOR does not bypass moderation
    observe(&pool, 201).await.allowed(); // protected owner, even with a stale timeout row
    observe(&pool, 204).await.forbidden(); // non-member remains denied

    sqlx::query("UPDATE member_timeouts SET timeout_until = CURRENT_TIMESTAMP - INTERVAL '1 second' WHERE server_id = 210 AND user_id = 202")
        .execute(&pool).await.unwrap();
    observe(&pool, 202).await.allowed(); // expiration needs no cleanup job
    sqlx::query("UPDATE member_timeouts SET timeout_until = CURRENT_TIMESTAMP + INTERVAL '1 hour' WHERE server_id = 210 AND user_id = 202")
        .execute(&pool).await.unwrap();
    observe(&pool, 202).await.forbidden(); // new timeout applies to same identity
    sqlx::query("DELETE FROM member_timeouts WHERE server_id = 210 AND user_id IN (202,203)")
        .execute(&pool).await.unwrap();
    observe(&pool, 202).await.allowed(); // explicit timeout removal
    observe(&pool, 203).await.allowed();

    // Normal channel authorization is still required once a timeout is gone.
    sqlx::query("INSERT INTO channel_permission_overrides(channel_id,target_type,target_id,deny_bits) VALUES (220,'member',202,1024)")
        .execute(&pool).await.unwrap();
    observe(&pool, 202).await.forbidden(); // VIEW_CHANNEL denied, SEND_MESSAGES retained
    sqlx::query("UPDATE channel_permission_overrides SET deny_bits = 2048 WHERE channel_id = 220 AND target_id = 202")
        .execute(&pool).await.unwrap();
    observe(&pool, 202).await.forbidden(); // SEND_MESSAGES denied, VIEW_CHANNEL retained
    sqlx::query("DELETE FROM channel_permission_overrides WHERE channel_id = 220")
        .execute(&pool).await.unwrap();
    observe(&pool, 202).await.allowed();
    sqlx::query("DELETE FROM server_members WHERE server_id = 210 AND user_id = 202")
        .execute(&pool).await.unwrap();
    observe(&pool, 202).await.forbidden(); // current membership is rechecked
    sqlx::query("INSERT INTO server_members(user_id,server_id) VALUES (202,210)")
        .execute(&pool).await.unwrap();

    // Break only the timeout lookup while messages remain writable. Clear the
    // prepared statement cache so a renamed table's OID cannot mask the fault.
    sqlx::query("ALTER TABLE member_timeouts RENAME TO timeout_test_hidden_timeouts")
        .execute(&pool).await.unwrap();
    pool.acquire()
        .await
        .unwrap()
        .clear_cached_statements()
        .await
        .unwrap();
    let failed_lookup = observe(&pool, 202).await;
    sqlx::query("ALTER TABLE timeout_test_hidden_timeouts RENAME TO member_timeouts")
        .execute(&pool).await.unwrap();
    failed_lookup.database_error();
    observe(&pool, 202).await.allowed();
    pool.close().await;
}
