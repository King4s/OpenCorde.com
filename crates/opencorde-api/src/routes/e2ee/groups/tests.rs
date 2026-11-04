use super::*;

#[test]
fn base64_roundtrip() {
    let data = b"MLS welcome message bytes";
    assert_eq!(base64_decode(&base64_encode(data)).unwrap(), data);
}

#[test]
fn private_state_is_rejected_and_omitted_state_is_supported() {
    let request: InitGroupRequest = serde_json::from_str(r#"{"member_welcomes":[]}"#).unwrap();
    assert!(request.group_state.is_empty());
    assert!(reject_private_state("").is_ok());
    for value in ["private-state", "eyJwcml2YXRlIjoic2VjcmV0In0", " "] {
        assert!(matches!(
            reject_private_state(value),
            Err(ApiError::BadRequest(_))
        ));
    }
}

fn init_payload(state: &str, recipient: i64, welcome: &[u8]) -> InitGroupRequest {
    InitGroupRequest {
        group_state: state.into(),
        member_welcomes: vec![MemberWelcome {
            user_id: recipient.to_string(),
            welcome_message: base64_encode(welcome),
        }],
    }
}

async fn stored_state(pool: &sqlx::PgPool, user: i64) -> Vec<u8> {
    sqlx::query_scalar(
        "SELECT group_state FROM e2ee_groups WHERE channel_id = 88020 AND user_id = $1",
    )
    .bind(user)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL via OPENCORDE_TEST_DATABASE_URL"]
async fn welcome_is_delivered_once_and_private_state_never_changes() {
    let url = std::env::var("OPENCORDE_TEST_DATABASE_URL")
        .expect("set OPENCORDE_TEST_DATABASE_URL to a disposable database");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .unwrap();
    sqlx::migrate!("../opencorde-db/migrations")
        .run(&pool)
        .await
        .unwrap();

    // IDs are reserved for this fixture; the database must be disposable.
    sqlx::raw_sql(
        "DELETE FROM servers WHERE id = 88010;
         DELETE FROM users WHERE id IN (88001,88002,88003);
         INSERT INTO users(id,username,public_key) VALUES
            (88001,'e2ee_owner','e2ee_owner'),(88002,'e2ee_member','e2ee_member'),
            (88003,'e2ee_outsider','e2ee_outsider');
         INSERT INTO servers(id,name,owner_id) VALUES (88010,'e2ee_regression',88001);
         INSERT INTO server_members(user_id,server_id) VALUES (88001,88010),(88002,88010);
         INSERT INTO channels(id,server_id,name) VALUES (88020,88010,'encrypted');
         INSERT INTO e2ee_groups(channel_id,user_id,group_state) VALUES
            (88020,88001,convert_to('legacy-owner-state','UTF8')),
            (88020,88002,convert_to('legacy-member-state','UTF8'));",
    )
    .execute(&pool)
    .await
    .unwrap();
    let channel = Snowflake::new(88020);
    let owner = Snowflake::new(88001);
    let member = Snowflake::new(88002);
    let outsider = Snowflake::new(88003);
    let welcome = b"\0binary welcome\xff";
    let raw_state = base64_encode(b"private new group state");

    assert!(matches!(
        init_group_for_user(
            &pool,
            owner,
            channel,
            init_payload(&raw_state, 88002, welcome)
        )
        .await,
        Err(ApiError::BadRequest(_))
    ));
    assert!(matches!(
        update_state_for_user(
            &pool,
            owner,
            channel,
            UpdateStateRequest {
                group_state: raw_state
            }
        )
        .await,
        Err(ApiError::BadRequest(_))
    ));
    assert_eq!(stored_state(&pool, 88001).await, b"legacy-owner-state");
    assert_eq!(stored_state(&pool, 88002).await, b"legacy-member-state");
    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM e2ee_groups WHERE channel_id=88020 AND welcome_message IS NOT NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(pending, 0, "a rejected state upload cannot store a Welcome");

    init_group_for_user(&pool, owner, channel, init_payload("", 88002, welcome))
        .await
        .unwrap();
    assert!(matches!(
        get_welcome_for_user(&pool, outsider, channel).await,
        Err(ApiError::Forbidden)
    ));
    let received = get_welcome_for_user(&pool, member, channel)
        .await
        .unwrap()
        .0;
    assert_eq!(base64_decode(&received.welcome_message).unwrap(), welcome);
    assert!(matches!(
        get_welcome_for_user(&pool, member, channel).await,
        Err(ApiError::NotFound(_))
    ));

    // Two simultaneous consumers must never both receive the one-time payload.
    init_group_for_user(&pool, owner, channel, init_payload("", 88002, welcome))
        .await
        .unwrap();
    let (first, second) = tokio::join!(
        get_welcome_for_user(&pool, member, channel),
        get_welcome_for_user(&pool, member, channel)
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let (ok, error) = if first.is_ok() {
        (first, second)
    } else {
        (second, first)
    };
    assert_eq!(
        base64_decode(&ok.unwrap().0.welcome_message).unwrap(),
        welcome
    );
    assert!(matches!(error, Err(ApiError::NotFound(_))));

    assert_eq!(
        update_state_for_user(
            &pool,
            owner,
            channel,
            UpdateStateRequest {
                group_state: String::new()
            }
        )
        .await
        .unwrap(),
        StatusCode::NO_CONTENT
    );
    assert!(matches!(
        init_group_for_user(&pool, owner, channel, init_payload("", 88003, welcome)).await,
        Err(ApiError::Forbidden)
    ));
    assert_eq!(stored_state(&pool, 88001).await, b"legacy-owner-state");
    assert_eq!(stored_state(&pool, 88002).await, b"legacy-member-state");

    sqlx::query("DELETE FROM servers WHERE id=88010")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE id IN (88001,88002,88003)")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}
