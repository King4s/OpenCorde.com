//! # Integration Tests: bot_token_repo
//! Token creation, hash lookup for gateway auth, revocation scoping,
//! and the revoke-and-replace rotation transaction.
//!
//! ID range: 9_200_000_000 – 9_299_999_999.

use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::bot_token_repo;
use sqlx::PgPool;

use crate::common::{pool, run, seed_application, seed_user};

const BASE: i64 = 9_200_000_000;

fn sf(n: i64) -> Snowflake {
    Snowflake::new(n)
}

/// Seed creator + bot user + application; returns (app_id, bot_user_id, creator_id).
async fn setup(pool: &PgPool, base: i64) -> (i64, i64, i64) {
    let (creator, bot_user, app) = (base + 1, base + 2, base + 3);
    seed_user(pool, creator).await;
    seed_user(pool, bot_user).await;
    seed_application(pool, app, creator).await;
    (app, bot_user, creator)
}

#[test]
fn create_token_returns_full_row() {
    run(async {
        let pool = pool().await;
        let (app, bot, creator) = setup(pool, BASE).await;

        let row = bot_token_repo::create_token(
            pool,
            sf(BASE + 10),
            sf(app),
            sf(bot),
            "oc_abc",
            "hash-create",
            7,
            sf(creator),
            Some("primary"),
        )
        .await
        .expect("create");

        assert_eq!(row.application_id, app);
        assert_eq!(row.bot_user_id, bot);
        assert_eq!(row.token_prefix, "oc_abc");
        assert_eq!(row.intents, 7);
        assert_eq!(row.label.as_deref(), Some("primary"));
        assert!(row.revoked_at.is_none());
        assert!(row.last_used_at.is_none());
    });
}

#[test]
fn list_tokens_returns_active_and_revoked() {
    run(async {
        let pool = pool().await;
        let (app, bot, creator) = setup(pool, BASE + 100).await;
        let (t1, t2) = (BASE + 110, BASE + 111);
        for (id, hash) in [(t1, "hash-l1"), (t2, "hash-l2")] {
            bot_token_repo::create_token(
                pool,
                sf(id),
                sf(app),
                sf(bot),
                "oc_l",
                hash,
                0,
                sf(creator),
                None,
            )
            .await
            .expect("create");
        }
        bot_token_repo::revoke_token(pool, sf(t1), sf(app))
            .await
            .expect("revoke");

        let rows = bot_token_repo::list_tokens(pool, sf(app))
            .await
            .expect("list");
        assert_eq!(rows.len(), 2, "revoked tokens stay listed for audit");
        assert!(rows.iter().any(|r| r.id == t1 && r.revoked_at.is_some()));
        assert!(rows.iter().any(|r| r.id == t2 && r.revoked_at.is_none()));
    });
}

#[test]
fn find_by_hash_returns_active_token() {
    run(async {
        let pool = pool().await;
        let (app, bot, creator) = setup(pool, BASE + 200).await;
        bot_token_repo::create_token(
            pool,
            sf(BASE + 210),
            sf(app),
            sf(bot),
            "oc_f",
            "hash-find",
            3,
            sf(creator),
            None,
        )
        .await
        .expect("create");

        let found = bot_token_repo::find_by_hash(pool, "hash-find")
            .await
            .expect("query");
        assert_eq!(found.expect("token").intents, 3);

        let missing = bot_token_repo::find_by_hash(pool, "hash-nonexistent")
            .await
            .expect("query");
        assert!(missing.is_none());
    });
}

#[test]
fn find_by_hash_excludes_revoked_token() {
    run(async {
        let pool = pool().await;
        let (app, bot, creator) = setup(pool, BASE + 300).await;
        let t = BASE + 310;
        bot_token_repo::create_token(
            pool,
            sf(t),
            sf(app),
            sf(bot),
            "oc_r",
            "hash-revoked",
            0,
            sf(creator),
            None,
        )
        .await
        .expect("create");
        bot_token_repo::revoke_token(pool, sf(t), sf(app))
            .await
            .expect("revoke");

        let found = bot_token_repo::find_by_hash(pool, "hash-revoked")
            .await
            .expect("query");
        assert!(found.is_none(), "revoked tokens must not authenticate");
    });
}

#[test]
fn revoke_is_scoped_to_application() {
    run(async {
        let pool = pool().await;
        let (app, bot, creator) = setup(pool, BASE + 400).await;
        let (other_app, t) = (BASE + 420, BASE + 410);
        seed_application(pool, other_app, creator).await;
        bot_token_repo::create_token(
            pool,
            sf(t),
            sf(app),
            sf(bot),
            "oc_s",
            "hash-scoped",
            0,
            sf(creator),
            None,
        )
        .await
        .expect("create");

        // Revoking through the wrong application must not touch the token.
        let revoked = bot_token_repo::revoke_token(pool, sf(t), sf(other_app))
            .await
            .expect("query");
        assert!(!revoked);
        assert!(
            bot_token_repo::find_by_hash(pool, "hash-scoped")
                .await
                .expect("query")
                .is_some()
        );
    });
}

#[test]
fn revoke_twice_returns_false_second_time() {
    run(async {
        let pool = pool().await;
        let (app, bot, creator) = setup(pool, BASE + 500).await;
        let t = BASE + 510;
        bot_token_repo::create_token(
            pool,
            sf(t),
            sf(app),
            sf(bot),
            "oc_t",
            "hash-twice",
            0,
            sf(creator),
            None,
        )
        .await
        .expect("create");

        assert!(
            bot_token_repo::revoke_token(pool, sf(t), sf(app))
                .await
                .expect("first")
        );
        assert!(
            !bot_token_repo::revoke_token(pool, sf(t), sf(app))
                .await
                .expect("second")
        );
    });
}

#[test]
fn rotate_revokes_old_and_preserves_identity() {
    run(async {
        let pool = pool().await;
        let (app, bot, creator) = setup(pool, BASE + 600).await;
        let t = BASE + 610;
        bot_token_repo::create_token(
            pool,
            sf(t),
            sf(app),
            sf(bot),
            "oc_old",
            "hash-rot-old",
            1,
            sf(creator),
            Some("prod"),
        )
        .await
        .expect("create");

        let new_row =
            bot_token_repo::rotate_token(pool, sf(t), sf(app), "oc_new", "hash-rot-new", 5)
                .await
                .expect("rotate")
                .expect("token was active");

        assert_ne!(new_row.id, t);
        assert_eq!(new_row.bot_user_id, bot, "bot identity survives rotation");
        assert_eq!(new_row.created_by, creator);
        assert_eq!(new_row.label.as_deref(), Some("prod"));
        assert_eq!(new_row.intents, 5, "rotation applies the new intents");
        assert!(
            bot_token_repo::find_by_hash(pool, "hash-rot-old")
                .await
                .expect("query")
                .is_none()
        );
        assert!(
            bot_token_repo::find_by_hash(pool, "hash-rot-new")
                .await
                .expect("query")
                .is_some()
        );
    });
}

#[test]
fn rotate_returns_none_for_revoked_or_unknown_token() {
    run(async {
        let pool = pool().await;
        let (app, bot, creator) = setup(pool, BASE + 700).await;
        let t = BASE + 710;
        bot_token_repo::create_token(
            pool,
            sf(t),
            sf(app),
            sf(bot),
            "oc_g",
            "hash-gone",
            0,
            sf(creator),
            None,
        )
        .await
        .expect("create");
        bot_token_repo::revoke_token(pool, sf(t), sf(app))
            .await
            .expect("revoke");

        let rotated = bot_token_repo::rotate_token(pool, sf(t), sf(app), "oc_x", "hash-x", 0)
            .await
            .expect("query");
        assert!(rotated.is_none(), "revoked token cannot be rotated");

        let unknown =
            bot_token_repo::rotate_token(pool, sf(BASE + 799), sf(app), "oc_y", "hash-y", 0)
                .await
                .expect("query");
        assert!(unknown.is_none());
    });
}
