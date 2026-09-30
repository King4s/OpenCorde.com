//! # Integration Tests: integration_log_repo
//! Event logging with optional FK references, per-server listing with
//! limit + recency ordering, and application filtering.
//!
//! ID range: 9_500_000_000 – 9_599_999_999.

use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::integration_log_repo;
use serde_json::json;
use sqlx::PgPool;

use crate::common::{pool, run, seed_application, seed_server, seed_user};

const BASE: i64 = 9_500_000_000;

fn sf(n: i64) -> Snowflake {
    Snowflake::new(n)
}

/// Seed user + server + application; returns (server_id, app_id, user_id).
async fn setup(pool: &PgPool, base: i64) -> (i64, i64, i64) {
    let (user, server, app) = (base + 1, base + 2, base + 3);
    seed_user(pool, user).await;
    seed_server(pool, server, user).await;
    seed_application(pool, app, user).await;
    (server, app, user)
}

/// Backdate a log row by `seconds` so recency ordering is deterministic
/// even when inserts land in the same clock microsecond.
async fn backdate(pool: &PgPool, id: i64, seconds: i64) {
    sqlx::query("UPDATE integration_logs SET created_at = created_at - ($2 || ' seconds')::interval WHERE id = $1")
        .bind(id)
        .bind(seconds.to_string())
        .execute(pool)
        .await
        .expect("backdate log row");
}

#[test]
fn log_event_roundtrips_all_references() {
    run(async {
        let pool = pool().await;
        let (server, app, user) = setup(pool, BASE).await;

        let row = integration_log_repo::log_event(
            pool,
            sf(BASE + 10),
            Some(sf(server)),
            Some(sf(app)),
            Some(sf(user)),
            "webhook.execute",
            "success",
            &json!({"webhook_id": "123", "channel": "general"}),
        )
        .await
        .expect("log");

        assert_eq!(row.server_id, Some(server));
        assert_eq!(row.application_id, Some(app));
        assert_eq!(row.actor_bot_user_id, Some(user));
        assert_eq!(row.action_type, "webhook.execute");
        assert_eq!(row.status, "success");
        assert_eq!(row.metadata["webhook_id"], "123");
    });
}

#[test]
fn log_event_accepts_all_optional_refs_as_none() {
    run(async {
        let pool = pool().await;

        let row = integration_log_repo::log_event(
            pool,
            sf(BASE + 110),
            None,
            None,
            None,
            "system.cleanup",
            "failure",
            &json!({}),
        )
        .await
        .expect("log");

        assert_eq!(row.server_id, None);
        assert_eq!(row.application_id, None);
        assert_eq!(row.actor_bot_user_id, None);
        assert_eq!(row.status, "failure");
    });
}

#[test]
fn list_by_server_newest_first_with_limit() {
    run(async {
        let pool = pool().await;
        let (server, app, _) = setup(pool, BASE + 200).await;
        let (e1, e2, e3) = (BASE + 210, BASE + 211, BASE + 212);
        for id in [e1, e2, e3] {
            integration_log_repo::log_event(
                pool,
                sf(id),
                Some(sf(server)),
                Some(sf(app)),
                None,
                "command.run",
                "success",
                &json!({}),
            )
            .await
            .expect("log");
        }
        backdate(pool, e1, 20).await;
        backdate(pool, e2, 10).await;

        let rows = integration_log_repo::list_by_server(pool, sf(server), 2)
            .await
            .expect("list");
        assert_eq!(rows.iter().map(|r| r.id).collect::<Vec<_>>(), vec![e3, e2]);
    });
}

#[test]
fn list_by_server_excludes_other_servers_and_null_server() {
    run(async {
        let pool = pool().await;
        let (server, app, user) = setup(pool, BASE + 300).await;
        let other_server = BASE + 320;
        seed_server(pool, other_server, user).await;

        integration_log_repo::log_event(
            pool,
            sf(BASE + 310),
            Some(sf(server)),
            Some(sf(app)),
            None,
            "a",
            "success",
            &json!({}),
        )
        .await
        .expect("log mine");
        integration_log_repo::log_event(
            pool,
            sf(BASE + 311),
            Some(sf(other_server)),
            Some(sf(app)),
            None,
            "b",
            "success",
            &json!({}),
        )
        .await
        .expect("log other");
        integration_log_repo::log_event(
            pool,
            sf(BASE + 312),
            None,
            Some(sf(app)),
            None,
            "c",
            "success",
            &json!({}),
        )
        .await
        .expect("log global");

        let rows = integration_log_repo::list_by_server(pool, sf(server), 50)
            .await
            .expect("list");
        assert_eq!(
            rows.iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![BASE + 310]
        );
    });
}

#[test]
fn list_by_server_and_app_filters_application() {
    run(async {
        let pool = pool().await;
        let (server, app, user) = setup(pool, BASE + 400).await;
        let other_app = BASE + 420;
        seed_application(pool, other_app, user).await;

        integration_log_repo::log_event(
            pool,
            sf(BASE + 410),
            Some(sf(server)),
            Some(sf(app)),
            None,
            "a",
            "success",
            &json!({}),
        )
        .await
        .expect("log app");
        integration_log_repo::log_event(
            pool,
            sf(BASE + 411),
            Some(sf(server)),
            Some(sf(other_app)),
            None,
            "b",
            "success",
            &json!({}),
        )
        .await
        .expect("log other app");

        let rows = integration_log_repo::list_by_server_and_app(pool, sf(server), sf(app), 50)
            .await
            .expect("list");
        assert_eq!(
            rows.iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![BASE + 410]
        );
    });
}

#[test]
fn metadata_preserves_nested_json() {
    run(async {
        let pool = pool().await;
        let (server, app, _) = setup(pool, BASE + 500).await;

        let metadata = json!({
            "request": {"path": "/api/v1/webhooks/1", "method": "POST"},
            "latency_ms": 42,
            "tags": ["bot", "webhook"],
        });
        let row = integration_log_repo::log_event(
            pool,
            sf(BASE + 510),
            Some(sf(server)),
            Some(sf(app)),
            None,
            "webhook.execute",
            "success",
            &metadata,
        )
        .await
        .expect("log");

        assert_eq!(row.metadata, metadata);
    });
}
