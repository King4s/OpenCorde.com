use super::*;

const ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const BUCKET: &str = "opencorde";

#[test]
fn avatar_inventory_preserves_keys_across_historical_hosts_and_prefixes() {
    let key = format!("avatars/{ID}/profile photo%20été.png");
    for base in [
        "https://old.example/files",
        "https://new.example",
        "http://127.0.0.1:9000",
        "/files",
    ] {
        assert_eq!(
            local_avatar_key(&format!("{base}/{BUCKET}/{key}"), BUCKET).unwrap(),
            Some(key.clone())
        );
    }
}

#[test]
fn avatar_inventory_excludes_external_urls_without_fetching_them() {
    for reference in [
        "https://avatars.steamstatic.com/example_full.jpg".to_string(),
        "https://cdn.discordapp.com/avatars/123456/hash.png".to_string(),
        format!("https://external.example?next=/{BUCKET}/avatars/{ID}/image.png"),
        format!("https://external.example#/{BUCKET}/avatars/{ID}/image.png"),
        format!("https://external.example/picture?next=/{BUCKET}/avatars/{ID}/image.png"),
    ] {
        assert_eq!(local_avatar_key(&reference, BUCKET).unwrap(), None);
    }
}

#[test]
fn local_avatar_shape_on_unknown_host_is_checked_only_in_configured_storage() {
    // The schema cannot prove historical host ownership. A matching path must
    // become a configured-bucket candidate, never an outbound URL fetch.
    assert_eq!(
        local_avatar_key(
            &format!("https://untrusted.invalid/{BUCKET}/avatars/{ID}/file.png"),
            BUCKET
        )
        .unwrap(),
        Some(format!("avatars/{ID}/file.png"))
    );
}

#[test]
fn ambiguous_or_malformed_local_avatar_references_fail_closed() {
    for reference in [
        format!("https://old.example/other-bucket/avatars/{ID}/image.png"),
        format!("https://old.example/{BUCKET}/avatars/{ID}/image.png?size=100"),
        format!("https://old.example/{BUCKET}/avatars/{ID}/image#fragment.png"),
        format!("https://old.example/{BUCKET}/avatars/{ID}/"),
        format!("https://old.example/{BUCKET}/avatars/not-a-uuid/image.png"),
        format!("https://old.example/{BUCKET}/avatars/{ID}/nested/{BUCKET}/avatars/{ID}/image.png"),
        format!(
            "https://old.example/{BUCKET}/avatars/550e8400-e29b-11d4-a716-446655440000/image.png"
        ),
        "data:image/png;base64,fixture".to_string(),
    ] {
        assert!(local_avatar_key(&reference, BUCKET).is_err());
    }
}

#[test]
fn avatar_inventory_uses_literal_uploaded_filename_without_url_normalization() {
    for filename in [
        "percent%2Fname.png",
        "nested/../name.png",
        "folder/name.png",
    ] {
        let key = format!("avatars/{ID}/{filename}");
        assert_eq!(
            local_avatar_key(&format!("https://old.example/{BUCKET}/{key}"), BUCKET).unwrap(),
            Some(key)
        );
    }
}

#[test]
fn reference_manifest_separates_files_avatars_and_external_inventory() {
    let digest = |source, id, reference: &str| {
        let mut hash = Sha256::new();
        hash_reference(&mut hash, source, id, reference);
        hex::encode(hash.finalize())
    };
    assert_eq!(digest(1, 42, "avatars/key"), digest(1, 42, "avatars/key"));
    assert_ne!(digest(0, 42, "avatars/key"), digest(1, 42, "avatars/key"));
    assert_ne!(digest(1, 42, "avatars/key"), digest(2, 42, "avatars/key"));
    assert_ne!(digest(1, 42, "avatars/key"), digest(1, 43, "avatars/key"));
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL via OPENCORDE_TEST_DATABASE_URL"]
async fn object_inventory_includes_avatars_and_files_across_keyset_batches() {
    let url = std::env::var("OPENCORDE_TEST_DATABASE_URL")
        .expect("set OPENCORDE_TEST_DATABASE_URL to a disposable database");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    sqlx::raw_sql(
        "CREATE TEMP TABLE files(id BIGINT PRIMARY KEY, bucket_key TEXT NOT NULL, size BIGINT NOT NULL); \
         CREATE TEMP TABLE users(id BIGINT PRIMARY KEY, avatar_url TEXT); \
         INSERT INTO files SELECT id, 'attachments/' || id, id FROM generate_series(1,501) id; \
         INSERT INTO users VALUES (1,NULL),(2,''),(3,'https://avatars.steamstatic.com/external.jpg');"
    ).execute(&pool).await.unwrap();
    let key = format!("avatars/{ID}/image.png");
    // Multiple users may share a key: preserve both references in the manifest.
    sqlx::query("INSERT INTO users VALUES (4,$1),(5,$1)")
        .bind(format!("https://historical.example/files/{BUCKET}/{key}"))
        .execute(&pool)
        .await
        .unwrap();
    let first = reference_batch(&pool, None).await.unwrap();
    assert_eq!(first.len(), 500);
    assert!(
        first
            .iter()
            .all(|row| row.source == 0 && row.expected_size == Some(row.id))
    );
    let last = first.last().unwrap();
    let second = reference_batch(&pool, Some((last.source, last.id)))
        .await
        .unwrap();
    assert_eq!(
        second
            .iter()
            .map(|row| (row.source, row.id))
            .collect::<Vec<_>>(),
        vec![(0, 501), (1, 3), (1, 4), (1, 5)]
    );
    assert_eq!(
        local_avatar_key(&second[1].reference, BUCKET).unwrap(),
        None
    );
    for row in &second[2..] {
        assert_eq!(
            local_avatar_key(&row.reference, BUCKET).unwrap(),
            Some(key.clone())
        );
        assert!(
            row.expected_size.is_none(),
            "avatar size must come from S3 HEAD"
        );
    }
    assert!(
        reference_batch(&pool, Some((1, 5)))
            .await
            .unwrap()
            .is_empty()
    );
    pool.close().await;
}
