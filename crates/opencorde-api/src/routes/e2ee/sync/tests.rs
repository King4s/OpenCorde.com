use super::*;
fn op(id: &str, seq: i64, epoch: i64, kind: &str) -> Operation {
    Operation {
        operation_id: format!("test-operation-{id}"),
        expected_sequence: seq.to_string(),
        epoch: epoch.to_string(),
        kind: kind.into(),
        ciphertext: if kind == "init" {
            String::new()
        } else {
            URL_SAFE_NO_PAD.encode(b"opaque authenticated MLS bytes")
        },
        added: vec![],
        removed: vec![],
        reply_to_id: None,
        attachments: None,
        allow_legacy_rejoin: false,
    }
}
#[tokio::test]
#[ignore = "requires disposable PostgreSQL via OPENCORDE_TEST_DATABASE_URL"]
async fn relay_order_retry_membership_and_atomicity() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&std::env::var("OPENCORDE_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    sqlx::migrate!("../opencorde-db/migrations")
        .run(&pool)
        .await
        .unwrap();
    sqlx::raw_sql("DELETE FROM servers WHERE id=89010;DELETE FROM users WHERE id IN(89001,89002,89003);
 INSERT INTO users(id,username,public_key) VALUES(89001,'sync_owner','owner'),(89002,'sync_peer','peer'),(89003,'sync_outsider','outsider');
 INSERT INTO servers(id,name,owner_id) VALUES(89010,'sync_qa',89001);
 INSERT INTO server_members(user_id,server_id) VALUES(89001,89010),(89002,89010);
 INSERT INTO channels(id,server_id,name,e2ee_enabled) VALUES(89020,89010,'sync',TRUE);").execute(&pool).await.unwrap();
    let owner = Snowflake::new(89001);
    let peer = Snowflake::new(89002);
    let outsider = Snowflake::new(89003);
    let ch = Snowflake::new(89020);
    // Upload retries never reoffer an already consumed or deleted one-time package.
    super::super::key_packages::publish_for_user(&pool, peer, b"one-time-public-package")
        .await
        .unwrap();
    super::super::key_packages::publish_for_user(&pool, peer, b"one-time-public-package")
        .await
        .unwrap();
    let packages: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM e2ee_key_packages WHERE user_id=89002")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(packages, 1);
    sqlx::query("DELETE FROM e2ee_key_packages WHERE user_id=89002")
        .execute(&pool)
        .await
        .unwrap();
    super::super::key_packages::publish_for_user(&pool, peer, b"one-time-public-package")
        .await
        .unwrap();
    let packages: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM e2ee_key_packages WHERE user_id=89002")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(packages, 0);
    // A stale legacy preflight cannot write after transition committed.
    sqlx::query(
        "INSERT INTO channels(id,server_id,name,e2ee_enabled) VALUES(89021,89010,'race',TRUE)",
    )
    .execute(&pool)
    .await
    .unwrap();
    let race = Snowflake::new(89021);
    reject_legacy(&pool, race).await.unwrap();
    submit_for_user(&pool, owner, race, op("race-init", 0, 0, "init"))
        .await
        .unwrap();
    let stale_write=sqlx::query("INSERT INTO messages(id,channel_id,author_id,content) VALUES(89031,89021,89001,'plaintext stale preflight')").execute(&pool).await;
    assert!(
        stale_write.is_err(),
        "database guard rejects legacy mutation after raced transition"
    );
    assert!(
        sqlx::query(
            "INSERT INTO e2ee_groups(channel_id,user_id,group_state) VALUES(89021,89001,''::bytea)"
        )
        .execute(&pool)
        .await
        .is_err()
    );
    // Legacy migration is explicit and preserves historical private blobs/messages.
    sqlx::raw_sql("INSERT INTO channels(id,server_id,name,e2ee_enabled) VALUES(89022,89010,'legacy',TRUE);
      INSERT INTO e2ee_groups(channel_id,user_id,group_state) VALUES(89022,89001,convert_to('preserved-private-legacy-state','UTF8'));
      INSERT INTO messages(id,channel_id,author_id,content) VALUES(89032,89022,89001,'enc:legacy-ciphertext');")
        .execute(&pool).await.unwrap();
    let legacy=Snowflake::new(89022);
    let mut migrate=op("legacy-init",0,0,"init");
    assert!(matches!(submit_for_user(&pool,owner,legacy,migrate.clone()).await,Err(ApiError::Conflict(_))));
    migrate.allow_legacy_rejoin=true;
    submit_for_user(&pool,owner,legacy,migrate).await.unwrap();
    let preserved:Vec<u8>=sqlx::query_scalar("SELECT group_state FROM e2ee_groups WHERE channel_id=89022 AND user_id=89001").fetch_one(&pool).await.unwrap();
    assert_eq!(preserved,b"preserved-private-legacy-state");
    let old_content:String=sqlx::query_scalar("SELECT content FROM messages WHERE id=89032").fetch_one(&pool).await.unwrap();
    assert_eq!(old_content,"enc:legacy-ciphertext");
    let initial = op("init", 0, 0, "init");
    assert!(matches!(
        submit_for_user(&pool, outsider, ch, initial.clone()).await,
        Err(ApiError::Forbidden)
    ));
    let result = submit_for_user(&pool, owner, ch, initial.clone())
        .await
        .unwrap();
    assert_eq!(result["sequence"], "1");
    assert_eq!(
        submit_for_user(&pool, owner, ch, initial).await.unwrap(),
        result
    );
    let mut add = op("add", 1, 0, "commit");
    add.added.push(Added {
        user_id: peer.to_string(),
        welcome: URL_SAFE_NO_PAD.encode(b"encrypted Welcome"),
    });
    submit_for_user(&pool, owner, ch, add.clone())
        .await
        .unwrap();
    let first_welcome = welcome_for_user(&pool, peer, ch).await.unwrap();
    assert_eq!(
        welcome_for_user(&pool, peer, ch).await.unwrap(),
        first_welcome,
        "fetch is non-destructive"
    );
    assert!(welcome_for_user(&pool, outsider, ch).await.is_err());
    ack_for_user(&pool, peer, ch, &add.operation_id)
        .await
        .unwrap();
    ack_for_user(&pool, peer, ch, &add.operation_id)
        .await
        .unwrap();
    assert!(welcome_for_user(&pool, peer, ch).await.is_err());
    let mut duplicate = op("duplicate", 2, 1, "commit");
    duplicate.added = add.added.clone();
    assert!(matches!(
        submit_for_user(&pool, owner, ch, duplicate).await,
        Err(ApiError::Conflict(_))
    ));
    let mut missing = op("missing", 2, 1, "commit");
    missing.removed.push(outsider.to_string());
    assert!(matches!(
        submit_for_user(&pool, owner, ch, missing).await,
        Err(ApiError::Conflict(_))
    ));
    let pages = events_for_user(
        &pool,
        peer,
        ch,
        Page {
            after: Some("0".into()),
            limit: None,
        },
    )
    .await
    .unwrap();
    assert!(
        pages["events"].as_array().unwrap().is_empty(),
        "new members cannot fetch prior events"
    );
    let a = op("race-a", 2, 1, "application");
    let b = op("race-b", 2, 1, "application");
    let (a_result, b_result) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(
            submit_for_user(&pool, owner, ch, a.clone()),
            submit_for_user(&pool, peer, ch, b.clone())
        )
    })
    .await
    .expect("pool1 concurrent relay submissions must not deadlock");
    assert_eq!(
        usize::from(a_result.is_ok()) + usize::from(b_result.is_ok()),
        1
    );
    let (winner, who, event) = if let Ok(e) = a_result {
        (a, owner, e)
    } else {
        (b, peer, b_result.unwrap())
    };
    assert_eq!(
        submit_for_user(&pool, who, ch, winner.clone())
            .await
            .unwrap(),
        event,
        "retry before stale CAS"
    );
    let mut changed = winner;
    changed.ciphertext = URL_SAFE_NO_PAD.encode(b"changed");
    assert!(matches!(
        submit_for_user(&pool, who, ch, changed).await,
        Err(ApiError::Conflict(_))
    ));
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE channel_id=89020")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
    sqlx::query("UPDATE channels SET slowmode_delay=3600 WHERE id=89020")
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        submit_for_user(&pool, who, ch, op("slow", 3, 1, "application")).await,
        Err(ApiError::RateLimited { .. })
    ));
    sqlx::query("UPDATE channels SET slowmode_delay=0 WHERE id=89020")
        .execute(&pool)
        .await
        .unwrap();
    // Foreign-key reply cleanup is metadata-only and must remain possible.
    let mut reply = op("reply", 3, 1, "application");
    reply.reply_to_id = event["message_id"].as_str().map(str::to_string);
    let reply_event = submit_for_user(&pool, owner, ch, reply).await.unwrap();
    sqlx::query("DELETE FROM messages WHERE id=$1")
        .bind(
            event["message_id"]
                .as_str()
                .unwrap()
                .parse::<i64>()
                .unwrap(),
        )
        .execute(&pool)
        .await
        .unwrap();
    let reply_to: Option<i64> = sqlx::query_scalar("SELECT reply_to_id FROM messages WHERE id=$1")
        .bind(
            reply_event["message_id"]
                .as_str()
                .unwrap()
                .parse::<i64>()
                .unwrap(),
        )
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(reply_to.is_none());
    let mut remove = op("remove", 4, 1, "commit");
    remove.removed.push(peer.to_string());
    submit_for_user(&pool, owner, ch, remove).await.unwrap();
    assert!(require_active(&pool, peer, ch).await.is_err());
    assert!(matches!(
        events_for_user(
            &pool,
            peer,
            ch,
            Page {
                after: None,
                limit: None
            }
        )
        .await,
        Err(ApiError::Forbidden)
    ));
    assert!(matches!(
        submit_for_user(&pool, peer, ch, op("denied", 5, 2, "application")).await,
        Err(ApiError::Forbidden)
    ));
    assert!(matches!(
        submit_for_user(&pool, owner, ch, op("stale-epoch", 5, 1, "application")).await,
        Err(ApiError::Conflict(_))
    ));
    let pending: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT welcome FROM e2ee_stream_members WHERE channel_id=89020 AND user_id=89002",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(pending.is_none());
    let mut readd = op("readd", 5, 2, "commit");
    readd.added = add.added.clone();
    submit_for_user(&pool, owner, ch, readd).await.unwrap();
    assert!(
        ack_for_user(&pool, peer, ch, &add.operation_id)
            .await
            .is_err(),
        "old ACK cannot clear new Welcome"
    );
    assert!(welcome_for_user(&pool, peer, ch).await.is_ok());
    sqlx::query("DELETE FROM server_members WHERE server_id=89010 AND user_id=89002")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        events_for_user(
            &pool,
            peer,
            ch,
            Page {
                after: None,
                limit: None
            }
        )
        .await
        .is_err(),
        "channel rights revocation overrides MLS membership"
    );
    assert!(
        submit_for_user(&pool, peer, ch, op("revoked", 6, 3, "application"))
            .await
            .is_err()
    );
    sqlx::query("UPDATE channels SET e2ee_enabled=FALSE WHERE id=89020")
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        submit_for_user(&pool, owner, ch, op("disabled", 6, 3, "application")).await,
        Err(ApiError::BadRequest(_))
    ));
    sqlx::raw_sql(
        "DELETE FROM servers WHERE id=89010;DELETE FROM users WHERE id IN(89001,89002,89003);",
    )
    .execute(&pool)
    .await
    .unwrap();
}
