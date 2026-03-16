//! Standalone JWT tests that don't require full app compilation

// Include the JWT module directly for testing
use opencorde_api::jwt::{
    create_access_token, create_refresh_token, validate_access_token, validate_refresh_token,
    validate_token,
};
use opencorde_core::Snowflake;

const TEST_SECRET: &str = "test-secret-key-min-32-chars-long!!!";
const USER_ID: i64 = 123456789;
const USERNAME: &str = "testuser";

#[test]
fn test_create_and_validate_access_token() {
    let user_id = Snowflake::new(USER_ID);
    let token = create_access_token(user_id, USERNAME, TEST_SECRET, 3600)
        .expect("token creation should succeed");

    let claims =
        validate_access_token(&token, TEST_SECRET).expect("token validation should succeed");

    assert_eq!(claims.sub, USER_ID.to_string());
    assert_eq!(claims.username, USERNAME);
    assert_eq!(claims.token_type, "access");
}

#[test]
fn test_create_and_validate_refresh_token() {
    let user_id = Snowflake::new(USER_ID);
    let token = create_refresh_token(user_id, USERNAME, TEST_SECRET, 604800)
        .expect("token creation should succeed");

    let claims =
        validate_refresh_token(&token, TEST_SECRET).expect("token validation should succeed");

    assert_eq!(claims.sub, USER_ID.to_string());
    assert_eq!(claims.username, USERNAME);
    assert_eq!(claims.token_type, "refresh");
}

#[test]
fn test_access_token_rejected_as_refresh() {
    let user_id = Snowflake::new(USER_ID);
    let token = create_access_token(user_id, USERNAME, TEST_SECRET, 3600)
        .expect("token creation should succeed");

    let result = validate_refresh_token(&token, TEST_SECRET);
    assert!(
        result.is_err(),
        "access token should not validate as refresh token"
    );
}

#[test]
fn test_refresh_token_rejected_as_access() {
    let user_id = Snowflake::new(USER_ID);
    let token = create_refresh_token(user_id, USERNAME, TEST_SECRET, 604800)
        .expect("token creation should succeed");

    let result = validate_access_token(&token, TEST_SECRET);
    assert!(
        result.is_err(),
        "refresh token should not validate as access token"
    );
}

#[test]
fn test_wrong_secret_fails() {
    let user_id = Snowflake::new(USER_ID);
    let token = create_access_token(user_id, USERNAME, TEST_SECRET, 3600)
        .expect("token creation should succeed");

    let wrong_secret = "wrong-secret-key-min-32-chars-long!!!";
    let result = validate_token(&token, wrong_secret);
    assert!(result.is_err(), "token should fail with wrong secret");
}

#[test]
fn test_malformed_token() {
    let result = validate_token("not.a.valid.token", TEST_SECRET);
    assert!(result.is_err(), "malformed token should fail");
}

#[test]
fn test_different_users_different_tokens() {
    let user1 = Snowflake::new(111);
    let user2 = Snowflake::new(222);

    let token1 = create_access_token(user1, USERNAME, TEST_SECRET, 3600)
        .expect("token1 creation should succeed");
    let token2 = create_access_token(user2, USERNAME, TEST_SECRET, 3600)
        .expect("token2 creation should succeed");

    assert_ne!(token1, token2);

    let claims1 = validate_token(&token1, TEST_SECRET).expect("token1 validation should succeed");
    let claims2 = validate_token(&token2, TEST_SECRET).expect("token2 validation should succeed");

    assert_eq!(claims1.sub, "111");
    assert_eq!(claims2.sub, "222");
}
