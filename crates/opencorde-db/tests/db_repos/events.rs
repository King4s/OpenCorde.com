//! # Integration Tests: event_repo
//! Create/read round trip for scheduled events, including the recurrence
//! columns added in migration 061, and the list_by_server filter.
//!
//! ID range: 9_600_000_000 – 9_699_999_999.

use chrono::{Duration, Utc};
use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::event_repo;
use sqlx::PgPool;

use crate::common::{pool, run, seed_server, seed_user};

const BASE: i64 = 9_600_000_000;

fn sf(n: i64) -> Snowflake {
    Snowflake::new(n)
}

/// Seed creator + server; returns (server_id, creator_id).
async fn setup(pool: &PgPool, base: i64) -> (i64, i64) {
    let (creator, server) = (base + 1, base + 2);
    seed_user(pool, creator).await;
    seed_server(pool, server, creator).await;
    (server, creator)
}

#[test]
fn create_event_round_trips_through_get_by_id() {
    run(async {
        let pool = pool().await;
        let (server, creator) = setup(pool, BASE).await;
        let starts = Utc::now() + Duration::hours(2);

        let created = event_repo::create_event(
            pool,
            sf(BASE + 10),
            sf(server),
            None,
            sf(creator),
            "Standup",
            Some("daily sync"),
            "external",
            Some("Room 1"),
            starts,
            None,
            None,
            None,
        )
        .await
        .expect("create event");

        assert_eq!(created.server_id, server);
        assert_eq!(created.creator_id, creator);
        assert_eq!(created.title, "Standup");
        assert_eq!(created.status, "scheduled");
        assert_eq!(created.rsvp_count, 0);
        assert!(!created.is_recurring);
        assert!(created.recurrence_rule.is_none());
        assert!(created.parent_event_id.is_none());

        let fetched = event_repo::get_by_id(pool, sf(BASE + 10))
            .await
            .expect("get")
            .expect("exists");
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.creator_username, created.creator_username);
    });
}

#[test]
fn recurring_event_persists_rule_and_flag() {
    run(async {
        let pool = pool().await;
        let (server, creator) = setup(pool, BASE + 100).await;
        let starts = Utc::now() + Duration::days(1);
        let rule = serde_json::json!({"freq": "weekly", "interval": 1, "by_day": ["mon"]});
        let until = starts + Duration::days(60);

        let created = event_repo::create_event(
            pool,
            sf(BASE + 110),
            sf(server),
            None,
            sf(creator),
            "Weekly",
            None,
            "voice",
            None,
            starts,
            Some(starts + Duration::hours(1)),
            Some(&rule),
            Some(until),
        )
        .await
        .expect("create recurring event");

        assert!(created.is_recurring);
        assert_eq!(created.recurrence_rule, Some(rule));
        assert_eq!(
            created.recurrence_end_date.map(|d| d.timestamp()),
            Some(until.timestamp())
        );
    });
}

#[test]
fn list_by_server_hides_completed_unless_include_past() {
    run(async {
        let pool = pool().await;
        let (server, creator) = setup(pool, BASE + 200).await;
        let starts = Utc::now() + Duration::hours(1);

        for (n, title) in [(210, "open"), (211, "done")] {
            event_repo::create_event(
                pool,
                sf(BASE + n),
                sf(server),
                None,
                sf(creator),
                title,
                None,
                "external",
                None,
                starts,
                None,
                None,
                None,
            )
            .await
            .expect("create");
        }
        event_repo::update_status(pool, sf(BASE + 211), "completed")
            .await
            .expect("complete");

        let upcoming = event_repo::list_by_server(pool, sf(server), false)
            .await
            .expect("list upcoming");
        assert_eq!(upcoming.len(), 1);
        assert_eq!(upcoming[0].title, "open");

        let all = event_repo::list_by_server(pool, sf(server), true)
            .await
            .expect("list all");
        assert_eq!(all.len(), 2);
    });
}
