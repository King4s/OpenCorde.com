//! Integration tests for JWT and authentication

use opencorde_api::jwt::{create_access_token, validate_access_token};
use opencorde_core::Snowflake;

#[test]
fn test_jwt_create_and_validate() {
    const SECRET: &str = "test-secret-key-min-32-chars-long!!!";
    const USER_ID: i64 = 123456789;
    const USERNAME: &str = "testuser";

    let user_id = Snowflake::new(USER_ID);
    let token = create_access_token(user_id, USERNAME, SECRET, 3600)
        .expect("token creation should succeed");

    let claims = validate_access_token(&token, SECRET).expect("validation should succeed");

    assert_eq!(claims.sub, USER_ID.to_string());
    assert_eq!(claims.username, USERNAME);
    assert_eq!(claims.token_type, "access");
}

#[test]
fn test_jwt_wrong_secret() {
    const SECRET: &str = "test-secret-key-min-32-chars-long!!!";
    const WRONG_SECRET: &str = "wrong-secret-key-min-32-chars-long!";
    const USER_ID: i64 = 123456789;
    const USERNAME: &str = "testuser";

    let user_id = Snowflake::new(USER_ID);
    let token = create_access_token(user_id, USERNAME, SECRET, 3600)
        .expect("token creation should succeed");

    let result = validate_access_token(&token, WRONG_SECRET);
    assert!(result.is_err(), "validation with wrong secret should fail");
}
