//! Bounded offline recovery helpers. No migrations, application workers, registry
//! discovery, or authentication service are started. Stop all writers first.
use std::{path::Path, time::Duration};

use anyhow::{Context, Result};
use aws_sdk_s3::config::{Credentials, Region};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};

const BATCH_SIZE: i64 = 500;

fn required(name: &str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} is required"))?;
    anyhow::ensure!(!value.is_empty(), "{name} must not be empty");
    Ok(value)
}

async fn pool() -> Result<PgPool> {
    let options: sqlx::postgres::PgConnectOptions = required("DATABASE_URL")?.parse()?;
    Ok(sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(15))
        .connect_with(options.options([("statement_timeout", "30000")]))
        .await?)
}

fn storage() -> Result<(aws_sdk_s3::Client, String)> {
    // Explicit credentials prevent SDK fallback to cloud metadata services.
    let credentials = Credentials::new(
        required("MINIO_ACCESS_KEY")?,
        required("MINIO_SECRET_KEY")?,
        None,
        None,
        "offline-maintenance",
    );
    let config = aws_sdk_s3::Config::builder()
        .credentials_provider(credentials)
        .region(Region::new("us-east-1"))
        .endpoint_url(required("MINIO_ENDPOINT")?)
        .force_path_style(true)
        .behavior_version_latest()
        .build();
    Ok((
        aws_sdk_s3::Client::from_conf(config),
        required("MINIO_BUCKET")?,
    ))
}

/// CLI: reindex | storage-init | verify-objects | search-verify <query>.
/// Standard output contains sanitized JSON, never object names or message text.
pub async fn run(args: &[String]) -> Result<Value> {
    match args {
        [command] if command == "reindex" => {
            let db = pool().await?;
            let count = reindex(&db, Path::new(&required("SEARCH_INDEX_PATH")?)).await?;
            Ok(
                json!({"ok":true,"operation":"reindex","indexed_messages":count,"batch_size":BATCH_SIZE}),
            )
        }
        [command] if command == "storage-init" => {
            let (client, bucket) = storage()?;
            // Head succeeds without mutations. Only an explicit 404 permits create;
            // authentication, network and other errors must fail closed.
            match client.head_bucket().bucket(&bucket).send().await {
                Ok(_) => Ok(json!({"ok":true,"operation":"storage-init","created":false})),
                Err(error) => {
                    let missing = error.raw_response().map(|r| r.status().as_u16()) == Some(404);
                    anyhow::ensure!(missing, "bucket could not be inspected");
                    client.create_bucket().bucket(&bucket).send().await?;
                    Ok(json!({"ok":true,"operation":"storage-init","created":true}))
                }
            }
        }
        [command] if command == "verify-objects" => {
            let db = pool().await?;
            let (client, bucket) = storage()?;
            verify_objects(&db, &client, &bucket).await
        }
        [command, query] if command == "search-verify" => {
            let path = required("SEARCH_INDEX_PATH")?;
            anyhow::ensure!(Path::new(&path).is_dir(), "index must already exist");
            let (index, schema) = opencorde_search::open_or_create(Path::new(&path))?;
            let count = opencorde_search::SearchEngine::new(index, schema)
                .search(query, None, None, 1000)?
                .len();
            Ok(
                json!({"ok":true,"operation":"search-verify","matched_messages":count,"result_limit":1000}),
            )
        }
        _ => anyhow::bail!(
            "usage: opencorde-maintenance reindex|storage-init|verify-objects|search-verify <query>"
        ),
    }
}

/// Build only from extant messages in plaintext channels. Messages are physically
/// deleted in the frozen 073 schema, so deleted rows cannot enter this query.
/// New destination is mandatory; failures retain the partial index for inspection.
pub async fn reindex(db: &PgPool, path: &Path) -> Result<u64> {
    let (index, _) = opencorde_search::create_fresh(path)?;
    let mut writer = opencorde_search::SearchIndexer::new(&index, 50_000_000)?;
    let mut last_id: Option<i64> = None;
    let mut indexed = 0u64;
    loop {
        let rows = sqlx::query(
            "SELECT m.id, m.channel_id, c.server_id, m.author_id, m.content, m.created_at \
             FROM messages m JOIN channels c ON c.id = m.channel_id \
             WHERE ($1::BIGINT IS NULL OR m.id > $1) AND c.e2ee_enabled = FALSE \
             ORDER BY m.id LIMIT $2",
        )
        .bind(last_id)
        .bind(BATCH_SIZE)
        .fetch_all(db)
        .await?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let id: i64 = row.try_get("id")?;
            let channel: i64 = row.try_get("channel_id")?;
            let server: i64 = row.try_get("server_id")?;
            let author: i64 = row.try_get("author_id")?;
            let content: String = row.try_get("content")?;
            let created: DateTime<Utc> = row.try_get("created_at")?;
            writer.index_message(
                id.try_into()?,
                channel.try_into()?,
                server.try_into()?,
                author.try_into()?,
                &content,
                created.timestamp().try_into()?,
            )?;
            last_id = Some(id);
            indexed += 1;
        }
        writer.commit()?;
    }
    writer.commit()?;
    Ok(indexed)
}

/// Reference rows have a stable (source, id) cursor. A shared object key may be
/// referenced by several rows: each reference is verified and hashed separately.
/// This avoids an unbounded in-memory deduplication set during offline recovery.
#[derive(sqlx::FromRow, Debug)]
struct StoredObjectReference {
    source: i16, // 0 = files, 1 = users.avatar_url
    id: i64,
    reference: String,
    expected_size: Option<i64>,
}

async fn reference_batch(
    db: &PgPool,
    after: Option<(i16, i64)>,
) -> Result<Vec<StoredObjectReference>> {
    Ok(sqlx::query_as(
        "WITH refs AS ( \
           SELECT 0::SMALLINT AS source, id, bucket_key AS reference, size AS expected_size FROM files \
           UNION ALL \
           SELECT 1::SMALLINT, id, avatar_url, NULL::BIGINT FROM users \
           WHERE avatar_url IS NOT NULL AND avatar_url <> '' \
         ) SELECT source, id, reference, expected_size FROM refs \
         WHERE ($1::SMALLINT IS NULL OR (source, id) > ($1, $2::BIGINT)) \
         ORDER BY source, id LIMIT $3",
    )
    .bind(after.map(|cursor| cursor.0))
    .bind(after.map(|cursor| cursor.1))
    .bind(BATCH_SIZE)
    .fetch_all(db)
    .await?)
}

fn canonical_avatar_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value)
        .map(|id| id.get_version_num() == 4 && id.to_string() == value)
        .unwrap_or(false)
}

/// The frozen uploader stores an unescaped string:
/// <public base>/<bucket>/avatars/<canonical UUIDv4>/<raw filename>.
/// Inspect its raw path, never fetch its host, decode percent escapes, or apply
/// URL dot-segment normalization: doing so would change the actual S3 key.
/// Historical host/base prefixes may differ after restore. Consequently a URL
/// on any host with this precise local namespace is a local-object candidate;
/// if that object is missing, verification fails rather than fetching that host.
/// Query/fragment-bearing candidates and conflicting bucket/namespace matches
/// cannot be proven from the schema and fail closed for operator reconciliation.
fn local_avatar_key(reference: &str, bucket: &str) -> Result<Option<String>> {
    let raw_path = if let Some(rest) = reference
        .strip_prefix("https://")
        .or_else(|| reference.strip_prefix("http://"))
    {
        let parsed = url::Url::parse(reference).context("invalid avatar reference")?;
        anyhow::ensure!(parsed.host_str().is_some(), "invalid avatar reference");
        match rest.find(['/', '?', '#', '\\']) {
            Some(position) if rest.as_bytes()[position] == b'/' => &rest[position..],
            Some(position) if rest.as_bytes()[position] == b'\\' => {
                anyhow::bail!("ambiguous avatar authority")
            }
            _ => "", // A slash inside a query/fragment is not a URL path.
        }
    } else if reference.starts_with('/') && !reference.starts_with("//") {
        reference
    } else {
        anyhow::bail!("unsupported avatar reference; reconcile before backup");
    };
    let path_end = raw_path.find(['?', '#']).unwrap_or(raw_path.len());
    let path = &raw_path[..path_end];
    let marker = format!("/{bucket}/avatars/");
    let mut matches = path.match_indices(&marker);
    let Some((offset, _)) = matches.next() else {
        // A canonical uploader UUID under another bucket may be a local avatar
        // from a differently configured installation. Do not silently omit it.
        let looks_like_other_bucket = path.match_indices("/avatars/").any(|(offset, _)| {
            path[offset + "/avatars/".len()..]
                .split_once('/')
                .is_some_and(|(id, _)| canonical_avatar_uuid(id))
        });
        anyhow::ensure!(
            !looks_like_other_bucket,
            "ambiguous avatar bucket reference"
        );
        return Ok(None);
    };
    anyhow::ensure!(
        matches.next().is_none(),
        "ambiguous avatar namespace reference"
    );
    anyhow::ensure!(
        path_end == raw_path.len(),
        "ambiguous avatar query or fragment"
    );
    let suffix = &path[offset + marker.len()..];
    let (id, filename) = suffix
        .split_once('/')
        .context("invalid local avatar reference")?;
    anyhow::ensure!(
        canonical_avatar_uuid(id) && !filename.is_empty(),
        "invalid local avatar reference"
    );
    Ok(Some(format!("avatars/{id}/{filename}")))
}

/// Read only from the configured bucket. Avatar sizes are not persisted in the
/// frozen schema, so HEAD supplies a required size before the streamed GET.
async fn verify_reference_content(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    expected_size: Option<i64>,
) -> Result<(u64, [u8; 32])> {
    tokio::time::timeout(Duration::from_secs(120), async {
        let expected = match expected_size {
            Some(size) => size,
            None => client
                .head_object()
                .bucket(bucket)
                .key(key)
                .send()
                .await?
                .content_length()
                .context("avatar object size missing")?,
        };
        anyhow::ensure!(expected >= 0, "invalid object size");
        let mut object = client.get_object().bucket(bucket).key(key).send().await?;
        anyhow::ensure!(
            object.content_length() == Some(expected),
            "object size differs from reference"
        );
        let mut hasher = Sha256::new();
        let mut bytes = 0u64;
        while let Some(chunk) = object.body.try_next().await? {
            bytes = bytes
                .checked_add(chunk.len() as u64)
                .context("size overflow")?;
            anyhow::ensure!(bytes <= expected as u64, "object exceeds reference size");
            hasher.update(&chunk);
        }
        anyhow::ensure!(
            bytes == expected as u64,
            "object size differs from reference"
        );
        Ok((bytes, hasher.finalize().into()))
    })
    .await?
}

fn hash_reference(manifest: &mut Sha256, source: i16, id: i64, reference: &str) {
    manifest.update(source.to_be_bytes());
    manifest.update(id.to_be_bytes());
    manifest.update((reference.len() as u64).to_be_bytes());
    manifest.update(reference.as_bytes());
}

async fn verify_objects(db: &PgPool, client: &aws_sdk_s3::Client, bucket: &str) -> Result<Value> {
    let mut cursor = None;
    let mut checked_files = 0u64;
    let mut checked_avatars = 0u64;
    let mut excluded_external_avatars = 0u64;
    let mut total_bytes = 0u64;
    let mut manifest = Sha256::new();
    manifest.update(b"opencorde-object-reference-manifest-v2\0");
    loop {
        let rows = reference_batch(db, cursor).await?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            cursor = Some((row.source, row.id));
            let (key, expected_size) = match row.source {
                0 => {
                    let expected = row.expected_size.context("file size missing")?;
                    anyhow::ensure!(expected >= 0, "invalid file size");
                    (row.reference, Some(expected))
                }
                1 => match local_avatar_key(&row.reference, bucket)? {
                    Some(key) => (key, None),
                    None => {
                        // Include the excluded reference in the inventory digest,
                        // but never send a network request to its URL.
                        hash_reference(&mut manifest, 2, row.id, &row.reference);
                        excluded_external_avatars += 1;
                        continue;
                    }
                },
                _ => anyhow::bail!("invalid object reference source"),
            };
            let (bytes, hash) =
                verify_reference_content(client, bucket, &key, expected_size).await?;
            hash_reference(&mut manifest, row.source, row.id, &key);
            manifest.update(bytes.to_be_bytes());
            manifest.update(hash);
            if row.source == 0 {
                checked_files += 1;
            } else {
                checked_avatars += 1;
            }
            total_bytes = total_bytes.checked_add(bytes).context("size overflow")?;
        }
    }
    Ok(json!({"ok":true,"operation":"verify-objects",
        "checked_objects":checked_files + checked_avatars,
        "checked_file_references":checked_files,"checked_avatar_references":checked_avatars,
        "external_avatar_references_excluded":excluded_external_avatars,
        "counting":"per-reference","verified_bytes":total_bytes,
        "manifest_sha256":hex::encode(manifest.finalize()),
        "manifest_format":"opencorde-object-reference-manifest-v2"}))
}

#[cfg(test)]
mod tests;
