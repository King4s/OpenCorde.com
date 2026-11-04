//! Offline reindex regression against the disposable PostgreSQL QA service.
use opencorde_api::maintenance::reindex;

#[tokio::test]
#[ignore = "requires disposable PostgreSQL via OPENCORDE_TEST_DATABASE_URL"]
async fn reindex_excludes_encrypted_and_deleted_messages_across_batches() {
    let url = std::env::var("OPENCORDE_TEST_DATABASE_URL")
        .expect("set OPENCORDE_TEST_DATABASE_URL to a disposable database");
    // Session-local tables shadow the application schema and disappear on close.
    // This fixture cannot mutate real messages even if the URL is misconfigured.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    sqlx::raw_sql(
        "CREATE TEMP TABLE channels(id BIGINT PRIMARY KEY, server_id BIGINT NOT NULL, e2ee_enabled BOOLEAN NOT NULL); \
         CREATE TEMP TABLE messages(id BIGINT PRIMARY KEY, channel_id BIGINT NOT NULL, author_id BIGINT NOT NULL, content TEXT NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()); \
         INSERT INTO channels VALUES (1,1,false),(2,1,true); \
         INSERT INTO messages(id,channel_id,author_id,content) SELECT id,1,1,'plaintextfixture' FROM generate_series(1,503) id; \
         INSERT INTO messages(id,channel_id,author_id,content) VALUES (504,2,1,'encryptedfixture'),(505,1,1,'deletedfixture'); \
         DELETE FROM messages WHERE id = 505;",
    ).execute(&pool).await.unwrap();
    let parent =
        std::env::temp_dir().join(format!("opencorde-reindex-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&parent).unwrap();
    let path = parent.join("index");
    assert_eq!(reindex(&pool, &path).await.unwrap(), 503);
    assert!(
        reindex(&pool, &path).await.is_err(),
        "existing index must be preserved"
    );
    let (index, schema) = opencorde_search::open_or_create(&path).unwrap();
    let engine = opencorde_search::SearchEngine::new(index, schema);
    assert_eq!(
        engine
            .search("plaintextfixture", None, None, 1000)
            .unwrap()
            .len(),
        503
    );
    assert!(
        engine
            .search("encryptedfixture", None, None, 1000)
            .unwrap()
            .is_empty()
    );
    assert!(
        engine
            .search("deletedfixture", None, None, 1000)
            .unwrap()
            .is_empty()
    );
    drop(engine);
    pool.close().await;
    std::fs::remove_dir_all(&parent).unwrap();
}
