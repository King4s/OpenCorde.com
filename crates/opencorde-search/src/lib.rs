//! # OpenCorde Search
//! Full-text search using Tantivy (Rust-native Lucene alternative).
//!
//! ## Usage
//! ```ignore
//! let (index, schema) = open_or_create(Path::new("/path/to/index"))?;
//! let mut indexer = SearchIndexer::new(&index, 50_000_000)?;
//! indexer.index_message(msg_id, ch_id, srv_id, auth_id, "hello", timestamp)?;
//! indexer.commit()?;
//!
//! let engine = SearchEngine::new(index, schema);
//! let results = engine.search("hello", None, None, 20)?;
//! ```
//!
//! ## Modules
//! - `schema` — Tantivy field definitions and index structure
//! - `indexer` — Document indexing operations (add, delete, commit)
//! - `searcher` — Query execution and result retrieval
//!
//! ## Depends On
//! - tantivy — Full-text search engine
//! - tracing — Structured logging
//! - serde — Result serialization

pub mod indexer;
pub mod schema;
pub mod searcher;

use anyhow::Result;
use std::path::Path;
use tantivy::Index;

pub use indexer::SearchIndexer;
pub use schema::SearchSchema;
pub use searcher::{SearchEngine, SearchResult};

/// Opens or creates a search index at the given path.
///
/// If the directory exists, opens the existing index.
/// If not, creates a new index with the default schema.
///
/// # Arguments
/// * `path` - Directory path for the index
///
/// # Returns
/// Tuple of (Index, SearchSchema) for use with SearchIndexer and SearchEngine
///
/// # Errors
/// Returns error if index creation/opening fails.
pub fn open_or_create(path: &Path) -> Result<(Index, SearchSchema)> {
    tracing::debug!(path = ?path, "opening or creating search index");

    let schema = SearchSchema::new();

    let index = if path.exists() {
        tracing::info!("opening existing index");
        Index::open_in_dir(path)?
    } else {
        tracing::info!("creating new index");
        std::fs::create_dir_all(path)?;
        Index::create_in_dir(path, schema.schema.clone())?
    };

    tracing::info!("index ready");
    Ok((index, schema))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_open_or_create_new() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("index");
        let result = open_or_create(&path);
        assert!(result.is_ok());
        assert!(path.exists());
    }

    #[test]
    fn test_open_or_create_existing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("index");

        // Create index
        let (index1, schema1) = open_or_create(&path).unwrap();
        drop(index1);

        // Open existing index
        let (index2, schema2) = open_or_create(&path).unwrap();
        drop(index2);

        // Verify schemas match
        assert_eq!(
            schema1.schema.fields().count(),
            schema2.schema.fields().count()
        );
    }
}
