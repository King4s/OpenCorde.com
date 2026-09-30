//! # Integration Tests: interaction_repo
//! Interaction creation, one-time token lookup (pending + unexpired only),
//! and the single-shot respond transition.
//!
//! ID range: 9_300_000_000 – 9_399_999_999.

use chrono::{Duration, Utc};
use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::interaction_repo;
use serde_json::json;
use sqlx::PgPool;

use crate::common::{pool, run, seed_application, seed_channel, seed_server, seed_user};

const BASE: i64 = 9_300_000_000;

fn sf(n: i64) -> Snowflake {
    Snowflake::new(n)
}

/// Seed user + application; returns (app_id, user_id).
async fn setup(pool: &PgPool, base: i64) -> (i64, i64) {
    let (user, app) = (base + 1, base + 2);
    seed_user(pool, user).await;
    seed_application(pool, app, user).await;
    (app, user)
}

/// Create a minimal command interaction expiring in the future.
async fn create(pool: &PgPool, id: i64, app: i64, user: i64, token_hash: &str) {
    interaction_repo::create_interaction(
        pool,
        sf(id),
        sf(app),
        token_hash,
        2, // command
        None,
        None,
        None,
        sf(user),
        &json!({"name": "ping"}),
        Utc::now() + Duration::minutes(15),
    )
    .await
    .expect("create interaction");
}

#[test]
fn create_returns_pending_row_with_data() {
    run(async {
        let pool = pool().await;
        let (app, user) = setup(pool, BASE).await;

        let row = interaction_repo::create_interaction(
            pool,
            sf(BASE + 10),
            sf(app),
            "ihash-create",
            2,
            None,
            None,
            None,
            sf(user),
            &json!({"name": "ping", "options": [{"n": 1}]}),
            Utc::now() + Duration::minutes(15),
        )
        .await
        .expect("create");

        assert_eq!(row.response_state, "pending");
        assert!(row.responded_at.is_none());
        assert_eq!(row.data["name"], "ping");
        assert_eq!(row.data["options"][0]["n"], 1);
    });
}

#[test]
fn create_with_server_and_channel_context() {
    run(async {
        let pool = pool().await;
        let (app, user) = setup(pool, BASE + 100).await;
        let (server, channel) = (BASE + 120, BASE + 121);
        seed_server(pool, server, user).await;
        seed_channel(pool, channel, server).await;

        let row = interaction_repo::create_interaction(
            pool,
            sf(BASE + 110),
            sf(app),
            "ihash-ctx",
            3, // component
            None,
            Some(sf(server)),
            Some(sf(channel)),
            sf(user),
            &json!({}),
            Utc::now() + Duration::minutes(15),
        )
        .await
        .expect("create");

        assert_eq!(row.server_id, Some(server));
        assert_eq!(row.channel_id, Some(channel));
        assert_eq!(row.interaction_type, 3);
    });
}

#[test]
fn find_by_token_matches_pending_interaction() {
    run(async {
        let pool = pool().await;
        let (app, user) = setup(pool, BASE + 200).await;
        let id = BASE + 210;
        create(pool, id, app, user, "ihash-find").await;

        let found = interaction_repo::find_by_token(pool, sf(id), "ihash-find")
            .await
            .expect("query");
        assert_eq!(found.expect("interaction").id, id);
    });
}

#[test]
fn find_by_token_rejects_wrong_hash_or_id() {
    run(async {
        let pool = pool().await;
        let (app, user) = setup(pool, BASE + 300).await;
        let id = BASE + 310;
        create(pool, id, app, user, "ihash-right").await;

        let wrong_hash = interaction_repo::find_by_token(pool, sf(id), "ihash-wrong")
            .await
            .expect("query");
        assert!(wrong_hash.is_none());

        let wrong_id = interaction_repo::find_by_token(pool, sf(id + 50), "ihash-right")
            .await
            .expect("query");
        assert!(wrong_id.is_none());
    });
}

#[test]
fn find_by_token_rejects_expired_interaction() {
    run(async {
        let pool = pool().await;
        let (app, user) = setup(pool, BASE + 400).await;
        let id = BASE + 410;
        interaction_repo::create_interaction(
            pool,
            sf(id),
            sf(app),
            "ihash-expired",
            2,
            None,
            None,
            None,
            sf(user),
            &json!({}),
            Utc::now() - Duration::minutes(1),
        )
        .await
        .expect("create expired");

        let found = interaction_repo::find_by_token(pool, sf(id), "ihash-expired")
            .await
            .expect("query");
        assert!(
            found.is_none(),
            "expired interaction tokens must not resolve"
        );
    });
}

#[test]
fn mark_responded_transitions_state_once() {
    run(async {
        let pool = pool().await;
        let (app, user) = setup(pool, BASE + 500).await;
        let id = BASE + 510;
        create(pool, id, app, user, "ihash-respond").await;

        assert!(
            interaction_repo::mark_responded(pool, sf(id), "responded")
                .await
                .expect("first respond")
        );
        // Second respond attempt hits a non-pending row and must report failure.
        assert!(
            !interaction_repo::mark_responded(pool, sf(id), "responded")
                .await
                .expect("second respond")
        );
    });
}

#[test]
fn token_is_single_use_after_respond() {
    run(async {
        let pool = pool().await;
        let (app, user) = setup(pool, BASE + 600).await;
        let id = BASE + 610;
        create(pool, id, app, user, "ihash-once").await;

        interaction_repo::mark_responded(pool, sf(id), "responded")
            .await
            .expect("respond");

        let found = interaction_repo::find_by_token(pool, sf(id), "ihash-once")
            .await
            .expect("query");
        assert!(
            found.is_none(),
            "responded interactions must not resolve again"
        );
    });
}

#[test]
fn mark_responded_unknown_id_returns_false() {
    run(async {
        let pool = pool().await;
        setup(pool, BASE + 700).await;

        let updated = interaction_repo::mark_responded(pool, sf(BASE + 799), "responded")
            .await
            .expect("query");
        assert!(!updated);
    });
}
