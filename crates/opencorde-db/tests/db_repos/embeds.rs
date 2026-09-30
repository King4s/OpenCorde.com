//! # Integration Tests: embed_repo
//! Embed creation, position-ordered retrieval, message scoping,
//! and cascade deletion with the parent message.
//!
//! ID range: 9_400_000_000 – 9_499_999_999.

use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::embed_repo;
use serde_json::json;
use sqlx::PgPool;

use crate::common::{pool, run, seed_channel, seed_message, seed_server, seed_user};

const BASE: i64 = 9_400_000_000;

fn sf(n: i64) -> Snowflake {
    Snowflake::new(n)
}

/// Seed user → server → channel → message; returns the message ID.
async fn setup_message(pool: &PgPool, base: i64) -> i64 {
    let (user, server, channel, message) = (base + 1, base + 2, base + 3, base + 4);
    seed_user(pool, user).await;
    seed_server(pool, server, user).await;
    seed_channel(pool, channel, server).await;
    seed_message(pool, message, channel, user).await;
    message
}

#[test]
fn create_embed_roundtrips_payload() {
    run(async {
        let pool = pool().await;
        let msg = setup_message(pool, BASE).await;

        let payload = json!({
            "title": "Release notes",
            "color": 5814783,
            "fields": [{"name": "Version", "value": "1.2.0", "inline": true}],
        });
        let row = embed_repo::create_embed(pool, sf(BASE + 10), sf(msg), 0, &payload)
            .await
            .expect("create");

        assert_eq!(row.message_id, msg);
        assert_eq!(row.position, 0);
        assert_eq!(row.payload, payload, "nested JSON must survive storage");
    });
}

#[test]
fn get_embeds_orders_by_position_not_insertion() {
    run(async {
        let pool = pool().await;
        let msg = setup_message(pool, BASE + 100).await;

        // Insert out of order: positions 2, 0, 1.
        for (i, pos) in [(0i64, 2i16), (1, 0), (2, 1)] {
            embed_repo::create_embed(pool, sf(BASE + 110 + i), sf(msg), pos, &json!({"p": pos}))
                .await
                .expect("create");
        }

        let rows = embed_repo::get_embeds(pool, sf(msg)).await.expect("get");
        let positions: Vec<i16> = rows.iter().map(|r| r.position).collect();
        assert_eq!(positions, vec![0, 1, 2]);
    });
}

#[test]
fn get_embeds_empty_for_message_without_embeds() {
    run(async {
        let pool = pool().await;
        let msg = setup_message(pool, BASE + 200).await;

        let rows = embed_repo::get_embeds(pool, sf(msg)).await.expect("get");
        assert!(rows.is_empty());
    });
}

#[test]
fn embeds_are_scoped_to_their_message() {
    run(async {
        let pool = pool().await;
        let msg_a = setup_message(pool, BASE + 300).await;
        let msg_b = setup_message(pool, BASE + 350).await;
        embed_repo::create_embed(pool, sf(BASE + 310), sf(msg_a), 0, &json!({"for": "a"}))
            .await
            .expect("embed a");
        embed_repo::create_embed(pool, sf(BASE + 311), sf(msg_b), 0, &json!({"for": "b"}))
            .await
            .expect("embed b");

        let rows = embed_repo::get_embeds(pool, sf(msg_a)).await.expect("get");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].payload["for"], "a");
    });
}

#[test]
fn embeds_cascade_delete_with_message() {
    run(async {
        let pool = pool().await;
        let msg = setup_message(pool, BASE + 400).await;
        embed_repo::create_embed(pool, sf(BASE + 410), sf(msg), 0, &json!({}))
            .await
            .expect("create");

        sqlx::query("DELETE FROM messages WHERE id = $1")
            .bind(msg)
            .execute(pool)
            .await
            .expect("delete message");

        let rows = embed_repo::get_embeds(pool, sf(msg)).await.expect("get");
        assert!(
            rows.is_empty(),
            "ON DELETE CASCADE must remove orphaned embeds"
        );
    });
}

#[test]
fn multiple_embeds_allowed_on_one_message() {
    run(async {
        let pool = pool().await;
        let msg = setup_message(pool, BASE + 500).await;
        for i in 0..3i64 {
            embed_repo::create_embed(
                pool,
                sf(BASE + 510 + i),
                sf(msg),
                i as i16,
                &json!({"i": i}),
            )
            .await
            .expect("create");
        }

        let rows = embed_repo::get_embeds(pool, sf(msg)).await.expect("get");
        assert_eq!(rows.len(), 3);
    });
}
