//! # Search Query Execution
//! Executes full-text search queries against the index.
//!
//! ## Depends On
//! - tantivy (full-text search library)
//! - crate::schema::SearchSchema
//! - serde (result serialization)

use serde::Serialize;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::OwnedValue;
use tantivy::{Index, TantivyDocument};
use tracing::instrument;

use crate::schema::SearchSchema;

/// A single search result with relevance score.
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    /// Unique message identifier
    pub message_id: u64,
    /// Channel containing this message
    pub channel_id: u64,
    /// Server containing the channel
    pub server_id: u64,
    /// Author of the message
    pub author_id: u64,
    /// Message text content
    pub content: String,
    /// Relevance score (higher = better match)
    pub score: f32,
}

/// Extracts a u64 field value from a Tantivy document.
fn extract_u64(doc: &TantivyDocument, field: tantivy::schema::Field) -> anyhow::Result<u64> {
    match doc.get_first(field) {
        Some(OwnedValue::U64(n)) => Ok(*n),
        Some(_) => Err(anyhow::anyhow!("field is not u64")),
        None => Err(anyhow::anyhow!("field not found in document")),
    }
}

/// Extracts a text field value from a Tantivy document.
fn extract_text(doc: &TantivyDocument, field: tantivy::schema::Field) -> anyhow::Result<String> {
    match doc.get_first(field) {
        Some(OwnedValue::Str(s)) => Ok(s.clone()),
        Some(_) => Err(anyhow::anyhow!("field is not text")),
        None => Err(anyhow::anyhow!("field not found in document")),
    }
}

/// Executes search queries against the Tantivy index.
#[derive(Debug)]
pub struct SearchEngine {
    index: Index,
    schema: SearchSchema,
}

impl SearchEngine {
    /// Creates a new SearchEngine from an Index.
    ///
    /// # Arguments
    /// * `index` - The Tantivy Index to search
    /// * `schema` - The schema used to create the index
    #[instrument(skip(index))]
    pub fn new(index: Index, schema: SearchSchema) -> Self {
        tracing::debug!("creating search engine");
        SearchEngine { index, schema }
    }

    /// Searches messages by content query with optional filtering.
    ///
    /// # Arguments
    /// * `query` - Full-text query string (e.g., "hello world")
    /// * `server_id` - Optional server filter (returns only matching messages)
    /// * `channel_id` - Optional channel filter (returns only matching messages)
    /// * `limit` - Maximum results to return (capped at 1000)
    ///
    /// # Returns
    /// Vector of SearchResult sorted by relevance (highest score first)
    ///
    /// # Errors
    /// Returns error if query parsing or search execution fails.
    #[instrument(skip(query))]
    pub fn search(
        &self,
        query: &str,
        server_id: Option<u64>,
        channel_id: Option<u64>,
        limit: usize,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let limit = limit.min(1000);
        tracing::info!(query = %query, limit, ?server_id, ?channel_id, "executing search");

        let reader = self.index.reader()?;
        let searcher = reader.searcher();

        let parser = QueryParser::for_index(&self.index, vec![self.schema.content]);
        let parsed = parser.parse_query(query)?;

        let top_docs = searcher.search(&parsed, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (score, addr) in top_docs {
            let doc = searcher.doc::<TantivyDocument>(addr)?;

            let message_id = extract_u64(&doc, self.schema.message_id)?;
            let channel_id_val = extract_u64(&doc, self.schema.channel_id)?;
            let server_id_val = extract_u64(&doc, self.schema.server_id)?;
            let author_id = extract_u64(&doc, self.schema.author_id)?;
            let content = extract_text(&doc, self.schema.content)?;

            results.push(SearchResult {
                message_id,
                channel_id: channel_id_val,
                server_id: server_id_val,
                author_id,
                content,
                score,
            });
        }

        if let Some(sid) = server_id {
            results.retain(|r| r.server_id == sid);
        }
        if let Some(cid) = channel_id {
            results.retain(|r| r.channel_id == cid);
        }

        tracing::info!(results = results.len(), "search complete");
        Ok(results)
    }

    /// Create an indexer that writes to this engine's index.
    ///
    /// Caller is responsible for calling commit() after indexing.
    ///
    /// # Errors
    /// Returns tantivy error if writer creation fails.
    pub fn make_indexer(&self, heap_size: usize) -> tantivy::Result<crate::SearchIndexer> {
        crate::SearchIndexer::new(&self.index, heap_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::doc;
    use tempfile::TempDir;

    fn create_test_engine() -> (TempDir, SearchEngine) {
        let dir = TempDir::new().unwrap();
        let schema = SearchSchema::new();
        let index = Index::create_in_dir(dir.path(), schema.schema.clone()).unwrap();

        let mut writer = index.writer(50_000_000).unwrap();
        writer
            .add_document(doc!(
                schema.message_id => 1001u64,
                schema.channel_id => 2001u64,
                schema.server_id => 3001u64,
                schema.author_id => 4001u64,
                schema.content => "hello world",
                schema.created_at => 1704067200u64,
            ))
            .unwrap();

        writer
            .add_document(doc!(
                schema.message_id => 1002u64,
                schema.channel_id => 2001u64,
                schema.server_id => 3001u64,
                schema.author_id => 4002u64,
                schema.content => "rust programming",
                schema.created_at => 1704067201u64,
            ))
            .unwrap();

        writer.commit().unwrap();

        let engine = SearchEngine::new(index, schema);
        (dir, engine)
    }

    #[test]
    fn test_search_engine_creation() {
        let (_dir, _engine) = create_test_engine();
    }

    #[test]
    fn test_simple_search() {
        let (_dir, engine) = create_test_engine();

        let results = engine.search("hello", None, None, 10).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.message_id == 1001));
    }

    #[test]
    fn test_search_with_no_results() {
        let (_dir, engine) = create_test_engine();

        let results = engine.search("nonexistent", None, None, 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_with_server_filter() {
        let (_dir, engine) = create_test_engine();

        let results = engine.search("hello", Some(3001), None, 10).unwrap();
        assert!(results.iter().all(|r| r.server_id == 3001));
    }

    #[test]
    fn test_search_with_channel_filter() {
        let (_dir, engine) = create_test_engine();

        let results = engine.search("hello", None, Some(2001), 10).unwrap();
        assert!(results.iter().all(|r| r.channel_id == 2001));
    }

    #[test]
    fn test_search_limit() {
        let (_dir, engine) = create_test_engine();

        let results = engine.search("world rust hello", None, None, 1).unwrap();
        assert!(results.len() <= 1);
    }

    #[test]
    fn test_search_result_has_score() {
        let (_dir, engine) = create_test_engine();

        let results = engine.search("hello", None, None, 10).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].score > 0.0);
    }
}
