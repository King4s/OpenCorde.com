//! # Integration Tests: dm_repo
//! DM channel get-or-create, messaging, pagination, membership,
//! and the message-request (pending/accepted) flow.
//!
//! ID range: 9_100_000_000 – 9_199_999_999.

use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::dm_repo;

use crate::common::{pool, run, seed_user};

const BASE: i64 = 9_100_000_000;

fn sf(n: i64) -> Snowflake {
    Snowflake::new(n)
}

#[test]
fn get_or_create_creates_channel_with_both_members() {
    run(async {
        let pool = pool().await;
        let (a, b, dm) = (BASE + 1, BASE + 2, BASE + 3);
        seed_user(pool, a).await;
        seed_user(pool, b).await;

        let id = dm_repo::get_or_create_dm(pool, sf(dm), sf(a), sf(b))
            .await
            .expect("create dm");

        assert_eq!(id, dm);
        assert!(
            dm_repo::is_dm_member(pool, sf(dm), sf(a))
                .await
                .expect("query a")
        );
        assert!(
            dm_repo::is_dm_member(pool, sf(dm), sf(b))
                .await
                .expect("query b")
        );
    });
}

#[test]
fn get_or_create_is_idempotent_and_order_insensitive() {
    run(async {
        let pool = pool().await;
        let (a, b, dm) = (BASE + 101, BASE + 102, BASE + 103);
        seed_user(pool, a).await;
        seed_user(pool, b).await;

        let first = dm_repo::get_or_create_dm(pool, sf(dm), sf(a), sf(b))
            .await
            .expect("create");
        // A fresh candidate ID and reversed user order must still find the
        // existing channel instead of creating a duplicate.
        let second = dm_repo::get_or_create_dm(pool, sf(dm + 50), sf(b), sf(a))
            .await
            .expect("reuse");

        assert_eq!(first, second);
    });
}

#[test]
fn request_status_is_set_per_member_on_create() {
    run(async {
        let pool = pool().await;
        let (a, b, dm) = (BASE + 201, BASE + 202, BASE + 203);
        seed_user(pool, a).await;
        seed_user(pool, b).await;

        dm_repo::get_or_create_dm_with_request_status(
            pool,
            sf(dm),
            sf(a),
            sf(b),
            "accepted",
            "pending",
        )
        .await
        .expect("create with statuses");

        // Sender (a) sees a regular DM; recipient (b) sees a message request.
        let a_dms = dm_repo::list_dms_for_user(pool, sf(a))
            .await
            .expect("a dms");
        assert!(a_dms.iter().any(|c| c.id == dm));

        let b_dms = dm_repo::list_dms_for_user(pool, sf(b))
            .await
            .expect("b dms");
        assert!(!b_dms.iter().any(|c| c.id == dm));

        let b_requests = dm_repo::list_message_requests(pool, sf(b))
            .await
            .expect("b requests");
        assert!(
            b_requests
                .iter()
                .any(|c| c.id == dm && c.message_request_status == "pending")
        );
    });
}

#[test]
fn request_status_untouched_when_channel_already_exists() {
    run(async {
        let pool = pool().await;
        let (a, b, dm) = (BASE + 301, BASE + 302, BASE + 303);
        seed_user(pool, a).await;
        seed_user(pool, b).await;

        dm_repo::get_or_create_dm_with_request_status(
            pool,
            sf(dm),
            sf(a),
            sf(b),
            "accepted",
            "accepted",
        )
        .await
        .expect("create accepted");
        // Second call demoting to pending must be a no-op on the existing rows.
        let reused = dm_repo::get_or_create_dm_with_request_status(
            pool,
            sf(dm + 50),
            sf(a),
            sf(b),
            "pending",
            "pending",
        )
        .await
        .expect("reuse");

        assert_eq!(reused, dm);
        let b_dms = dm_repo::list_dms_for_user(pool, sf(b))
            .await
            .expect("b dms");
        assert!(
            b_dms.iter().any(|c| c.id == dm),
            "b's accepted status must survive"
        );
    });
}

#[test]
fn send_message_returns_author_and_marks_sender_read() {
    run(async {
        let pool = pool().await;
        let (a, b, dm, msg) = (BASE + 401, BASE + 402, BASE + 403, BASE + 404);
        seed_user(pool, a).await;
        seed_user(pool, b).await;
        dm_repo::get_or_create_dm(pool, sf(dm), sf(a), sf(b))
            .await
            .expect("dm");

        let row = dm_repo::send_dm_message(pool, sf(msg), sf(dm), sf(a), "hello")
            .await
            .expect("send");

        assert_eq!(row.id, msg);
        assert_eq!(row.content, "hello");
        assert_eq!(row.author_username, format!("user{a}"));

        // Sender's own last_read_id advances to the new message; recipient's stays 0.
        let a_chan = dm_repo::list_dms_for_user(pool, sf(a))
            .await
            .expect("a dms");
        assert_eq!(
            a_chan
                .iter()
                .find(|c| c.id == dm)
                .expect("a row")
                .last_read_id,
            msg
        );
        let b_chan = dm_repo::list_dms_for_user(pool, sf(b))
            .await
            .expect("b dms");
        assert_eq!(
            b_chan
                .iter()
                .find(|c| c.id == dm)
                .expect("b row")
                .last_read_id,
            0
        );
    });
}

#[test]
fn list_messages_newest_first_with_cursor_and_limit() {
    run(async {
        let pool = pool().await;
        let (a, b, dm) = (BASE + 501, BASE + 502, BASE + 503);
        let (m1, m2, m3) = (BASE + 511, BASE + 512, BASE + 513);
        seed_user(pool, a).await;
        seed_user(pool, b).await;
        dm_repo::get_or_create_dm(pool, sf(dm), sf(a), sf(b))
            .await
            .expect("dm");
        for m in [m1, m2, m3] {
            dm_repo::send_dm_message(pool, sf(m), sf(dm), sf(a), "msg")
                .await
                .expect("send");
        }

        let all = dm_repo::list_dm_messages(pool, sf(dm), None, 10)
            .await
            .expect("all");
        assert_eq!(
            all.iter().map(|m| m.id).collect::<Vec<_>>(),
            vec![m3, m2, m1]
        );

        let before = dm_repo::list_dm_messages(pool, sf(dm), Some(sf(m3)), 10)
            .await
            .expect("cursor");
        assert_eq!(
            before.iter().map(|m| m.id).collect::<Vec<_>>(),
            vec![m2, m1]
        );

        let limited = dm_repo::list_dm_messages(pool, sf(dm), None, 2)
            .await
            .expect("limit");
        assert_eq!(
            limited.iter().map(|m| m.id).collect::<Vec<_>>(),
            vec![m3, m2]
        );
    });
}

#[test]
fn list_dms_shows_other_participants_username() {
    run(async {
        let pool = pool().await;
        let (a, b, dm) = (BASE + 601, BASE + 602, BASE + 603);
        seed_user(pool, a).await;
        seed_user(pool, b).await;
        dm_repo::get_or_create_dm(pool, sf(dm), sf(a), sf(b))
            .await
            .expect("dm");

        let a_dms = dm_repo::list_dms_for_user(pool, sf(a))
            .await
            .expect("a dms");
        let row = a_dms.iter().find(|c| c.id == dm).expect("channel row");
        assert_eq!(row.other_user_id, b);
        assert_eq!(row.other_username, format!("user{b}"));
    });
}

#[test]
fn accepting_request_moves_channel_to_dm_list() {
    run(async {
        let pool = pool().await;
        let (a, b, dm) = (BASE + 701, BASE + 702, BASE + 703);
        seed_user(pool, a).await;
        seed_user(pool, b).await;
        dm_repo::get_or_create_dm_with_request_status(
            pool,
            sf(dm),
            sf(a),
            sf(b),
            "accepted",
            "pending",
        )
        .await
        .expect("create");

        dm_repo::update_message_request_status(pool, sf(dm), sf(b), "accepted")
            .await
            .expect("accept");

        let requests = dm_repo::list_message_requests(pool, sf(b))
            .await
            .expect("requests");
        assert!(!requests.iter().any(|c| c.id == dm));
        let dms = dm_repo::list_dms_for_user(pool, sf(b)).await.expect("dms");
        assert!(dms.iter().any(|c| c.id == dm));
    });
}

#[test]
fn is_dm_member_false_for_outsider() {
    run(async {
        let pool = pool().await;
        let (a, b, c, dm) = (BASE + 801, BASE + 802, BASE + 803, BASE + 804);
        seed_user(pool, a).await;
        seed_user(pool, b).await;
        seed_user(pool, c).await;
        dm_repo::get_or_create_dm(pool, sf(dm), sf(a), sf(b))
            .await
            .expect("dm");

        assert!(
            !dm_repo::is_dm_member(pool, sf(dm), sf(c))
                .await
                .expect("outsider")
        );
    });
}

#[test]
fn list_dm_ids_returns_all_channels_regardless_of_status() {
    run(async {
        let pool = pool().await;
        let (a, b, c) = (BASE + 901, BASE + 902, BASE + 903);
        let (dm_ab, dm_ac) = (BASE + 904, BASE + 905);
        seed_user(pool, a).await;
        seed_user(pool, b).await;
        seed_user(pool, c).await;
        dm_repo::get_or_create_dm(pool, sf(dm_ab), sf(a), sf(b))
            .await
            .expect("dm ab");
        dm_repo::get_or_create_dm_with_request_status(
            pool,
            sf(dm_ac),
            sf(c),
            sf(a),
            "accepted",
            "pending",
        )
        .await
        .expect("dm ac (pending for a)");

        let ids = dm_repo::list_dm_ids_for_user(pool, sf(a))
            .await
            .expect("ids");
        assert!(ids.contains(&dm_ab));
        assert!(
            ids.contains(&dm_ac),
            "pending channels still count as membership"
        );
    });
}
