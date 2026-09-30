//! # Test Harness: Shared Postgres Container
//! Starts one ephemeral Postgres 16 container for the whole test binary,
//! applies the full migration chain, and hands out a shared connection pool
//! plus seed helpers for foreign-key parent rows.

use std::future::Future;
use std::sync::LazyLock;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt};
use tokio::runtime::Runtime;
use tokio::sync::OnceCell;

/// One runtime shared by every test in this binary.
///
/// `#[tokio::test]` would give each test its own runtime; the shared pool's
/// background tasks would then die with whichever test's runtime initialized
/// the pool, poisoning all remaining tests. Tests are plain `#[test]` fns
/// that drive their async body via `run` on this runtime instead.
static RT: LazyLock<Runtime> = LazyLock::new(|| Runtime::new().expect("test runtime"));

/// Run an async test body on the shared runtime.
pub fn run<F: Future>(future: F) -> F::Output {
    RT.block_on(future)
}

struct TestDb {
    /// Held so the container keeps running until the test binary exits.
    _container: ContainerAsync<Postgres>,
    pool: PgPool,
}

static DB: OnceCell<TestDb> = OnceCell::const_new();

/// Shared pool backed by a single ephemeral Postgres container.
///
/// The first caller pays the container start + migration cost (~seconds);
/// every other test reuses the same database.
pub async fn pool() -> &'static PgPool {
    let db = DB
        .get_or_init(|| async {
            let container = Postgres::default()
                // Match the production image in docker-compose.yml
                .with_tag("16-alpine")
                .start()
                .await
                .expect("start postgres testcontainer (is Docker running?)");
            let host = container.get_host().await.expect("container host");
            let port = container
                .get_host_port_ipv4(5432)
                .await
                .expect("mapped postgres port");
            let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
            let pool = PgPoolOptions::new()
                .max_connections(5)
                .connect(&url)
                .await
                .expect("connect to test postgres");
            sqlx::migrate!("./migrations")
                .run(&pool)
                .await
                .expect("apply migration chain");
            TestDb {
                _container: container,
                pool,
            }
        })
        .await;
    &db.pool
}

/// Insert a minimal user row; username and public_key are derived from the ID
/// so every seeded user is unique.
pub async fn seed_user(pool: &PgPool, id: i64) {
    sqlx::query("INSERT INTO users (id, username, public_key) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(format!("user{id}"))
        .bind(format!("{id:064x}"))
        .execute(pool)
        .await
        .expect("seed user");
}

/// Insert a minimal OAuth/bot application owned by `owner_user_id`.
pub async fn seed_application(pool: &PgPool, id: i64, owner_user_id: i64) {
    sqlx::query("INSERT INTO applications (id, name, owner_user_id) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(format!("app{id}"))
        .bind(owner_user_id)
        .execute(pool)
        .await
        .expect("seed application");
}

/// Insert a minimal server owned by `owner_id`.
pub async fn seed_server(pool: &PgPool, id: i64, owner_id: i64) {
    sqlx::query("INSERT INTO servers (id, name, owner_id) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(format!("server{id}"))
        .bind(owner_id)
        .execute(pool)
        .await
        .expect("seed server");
}

/// Insert a minimal text channel in `server_id`.
pub async fn seed_channel(pool: &PgPool, id: i64, server_id: i64) {
    sqlx::query("INSERT INTO channels (id, server_id, name) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(server_id)
        .bind(format!("channel{id}"))
        .execute(pool)
        .await
        .expect("seed channel");
}

/// Insert a minimal message in `channel_id` authored by `author_id`.
pub async fn seed_message(pool: &PgPool, id: i64, channel_id: i64, author_id: i64) {
    sqlx::query(
        "INSERT INTO messages (id, channel_id, author_id, content) VALUES ($1, $2, $3, 'seed')",
    )
    .bind(id)
    .bind(channel_id)
    .bind(author_id)
    .execute(pool)
    .await
    .expect("seed message");
}
