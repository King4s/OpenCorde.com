use super::*;
use crate::routes::messages::validation::extract_mentions;
use serde_json::Value;
use sqlx::{Connection, PgPool};

#[derive(Debug, PartialEq)]
struct Effects {
    pushes: Vec<i64>,
    unread: Vec<(i64, i32)>,
    events: Vec<Value>,
}

impl Effects {
    fn none() -> Self {
        Self {
            pushes: vec![],
            unread: vec![],
            events: vec![],
        }
    }

    fn reader() -> Self {
        Self {
            pushes: vec![1004],
            unread: vec![(1004, 1)],
            events: vec![serde_json::json!({
                "type": "ChannelUnreadUpdate",
                "data": { "user_id": "1004", "channel_id": "1020" }
            })],
        }
    }
}

async fn observe(pool: &PgPool, content: &str) -> Effects {
    sqlx::query("DELETE FROM channel_read_state WHERE channel_id = 1020")
        .execute(pool)
        .await
        .unwrap();
    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let mut pushes = vec![];
    dispatch_with_push(
        pool,
        &extract_mentions(content),
        Snowflake::new(1001),
        1020,
        1010,
        &tx,
        |user_id| {
            pushes.push(user_id);
            std::future::ready(())
        },
    )
    .await;
    pushes.sort_unstable();
    let unread = sqlx::query_as(
        "SELECT user_id, mention_count FROM channel_read_state WHERE channel_id = 1020 ORDER BY user_id",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    let mut events = vec![];
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }
    Effects {
        pushes,
        unread,
        events,
    }
}

#[tokio::test]
async fn lookup_failure_never_attempts_push_or_unread_event() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1/unused")
        .unwrap();
    pool.close().await;
    let (tx, mut rx) = tokio::sync::broadcast::channel(16);
    let mut pushes = vec![];
    dispatch_with_push(
        &pool,
        &extract_mentions("<@1004>"),
        Snowflake::new(1001),
        1020,
        1010,
        &tx,
        |user_id| {
            pushes.push(user_id);
            std::future::ready(())
        },
    )
    .await;
    assert!(pushes.is_empty());
    assert!(rx.try_recv().is_err());
}

// Invoked by the security runner against its disposable database after the WS
// regression. Fixture IDs are disjoint; never run this against application data.
#[tokio::test]
#[ignore = "requires disposable PostgreSQL via OPENCORDE_TEST_DATABASE_URL"]
async fn current_access_gates_notification_side_effects() {
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
            (1001,'notify_owner','notify_owner'),(1002,'notify_denied','notify_denied'),
            (1003,'notify_outsider','notify_outsider'),(1004,'notify_reader','notify_reader');
         INSERT INTO servers(id,name,owner_id) VALUES (1010,'notify_test',1001);
         INSERT INTO server_members(user_id,server_id) VALUES (1001,1010),(1002,1010),(1004,1010);
         INSERT INTO channels(id,server_id,name) VALUES (1020,1010,'private');
         INSERT INTO roles(id,server_id,name,permissions) VALUES (1030,1010,'mentioned',0);
         INSERT INTO member_roles(user_id,server_id,role_id) VALUES (1002,1010,1030),(1004,1010,1030);
         INSERT INTO channel_permission_overrides(channel_id,target_type,target_id,deny_bits)
            VALUES (1020,'role',1010,1024);
         INSERT INTO channel_permission_overrides(channel_id,target_type,target_id,allow_bits)
            VALUES (1020,'member',1004,1024);",
    )
    .execute(&pool)
    .await
    .unwrap();

    // Direct mentions, including the old fallback path, cannot bypass access.
    assert_eq!(observe(&pool, "<@1002>").await, Effects::none());
    assert_eq!(observe(&pool, "<@1003>").await, Effects::none());
    assert_eq!(observe(&pool, "<@1004>").await, Effects::reader());
    for content in ["@everyone", "@here", "<@&1030>", "<@1002> <@1003> <@1004>"] {
        assert_eq!(
            observe(&pool, content).await,
            Effects::reader(),
            "{content}"
        );
    }

    // Current membership and overwrites are rechecked across dispatches.
    sqlx::query("DELETE FROM server_members WHERE user_id = 1004 AND server_id = 1010")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(observe(&pool, "<@1004>").await, Effects::none());
    sqlx::query("INSERT INTO server_members(user_id,server_id) VALUES (1004,1010)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE channel_permission_overrides SET allow_bits = 0, deny_bits = 1024 WHERE channel_id = 1020 AND target_type = 'member' AND target_id = 1004")
        .execute(&pool).await.unwrap();
    assert_eq!(observe(&pool, "<@1004>").await, Effects::none());
    sqlx::query("UPDATE channel_permission_overrides SET allow_bits = 1024, deny_bits = 0 WHERE channel_id = 1020 AND target_type = 'member' AND target_id = 1004")
        .execute(&pool).await.unwrap();

    // Preserve the existing direct-mention push-only suppression behavior, but
    // never use it to deliver a denied member's or outsider's message preview.
    sqlx::query("INSERT INTO server_notification_settings(user_id,server_id,suppress_everyone) VALUES (1004,1010,TRUE)")
        .execute(&pool).await.unwrap();
    assert_eq!(observe(&pool, "@everyone").await, Effects::none());
    assert_eq!(
        observe(&pool, "@everyone <@1002> <@1003> <@1004>").await,
        Effects {
            pushes: vec![1004],
            ..Effects::none()
        },
    );

    // Fail inside authorization and preference resolution while the database
    // remains writable. The latter specifically guards the old error->fallback
    // leak. Restore each table before asserting, even if the regression returns
    // unexpected side effects.
    for (hide, restore) in [
        (
            "ALTER TABLE channel_permission_overrides RENAME TO notification_test_hidden_overrides",
            "ALTER TABLE notification_test_hidden_overrides RENAME TO channel_permission_overrides",
        ),
        (
            "ALTER TABLE channel_notification_settings RENAME TO notification_test_hidden_settings",
            "ALTER TABLE notification_test_hidden_settings RENAME TO channel_notification_settings",
        ),
    ] {
        sqlx::query(hide).execute(&pool).await.unwrap();
        // Force fresh preparation so no cached query can retain the renamed
        // table's OID and defeat the intended lookup failure.
        pool.acquire()
            .await
            .unwrap()
            .clear_cached_statements()
            .await
            .unwrap();
        let effects = observe(&pool, "<@1004>").await;
        sqlx::query(restore).execute(&pool).await.unwrap();
        assert_eq!(effects, Effects::none(), "{hide}");
    }
    assert_eq!(observe(&pool, "<@1004>").await, Effects::reader());
    pool.close().await;
}
