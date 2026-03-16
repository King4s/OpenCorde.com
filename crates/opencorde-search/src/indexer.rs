//! # Search Indexer
//! Adds, updates, and removes messages from the search index.
//!
//! ## Depends On
//! - tantivy (full-text search library)
//! - crate::schema::SearchSchema

use tantivy::{Index, IndexWriter, Term, doc};
use tracing::instrument;

use crate::schema::SearchSchema;

/// Manages indexing operations for the search engine.
pub struct SearchIndexer {
    schema: SearchSchema,
    writer: IndexWriter,
}

impl std::fmt::Debug for SearchIndexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchIndexer")
            .field("schema", &self.schema)
            .finish_non_exhaustive()
    }
}

impl SearchIndexer {
    /// Creates a new SearchIndexer from an Index and heap size.
    ///
    /// # Arguments
    /// * `index` - The Tantivy Index to write to
    /// * `heap_size` - Memory buffer size in bytes (typically 50MB = 50_000_000)
    ///
    /// # Errors
    /// Returns Tantivy error if writer creation fails.
    #[instrument(skip(index))]
    pub fn new(index: &Index, heap_size: usize) -> tantivy::Result<Self> {
        tracing::debug!(heap_size, "creating search indexer");
        let schema = SearchSchema::new();
        let writer = index.writer(heap_size)?;
        Ok(SearchIndexer { schema, writer })
    }

    /// Indexes a message for full-text search.
    ///
    /// # Arguments
    /// * `message_id` - Unique message identifier
    /// * `channel_id` - Channel containing this message
    /// * `server_id` - Server containing the channel
    /// * `author_id` - User who authored the message
    /// * `content` - Message text content
    /// * `created_at_unix` - Unix timestamp in seconds
    ///
    /// # Errors
    /// Returns Tantivy error if document addition fails.
    #[instrument(skip(content))]
    pub fn index_message(
        &mut self,
        message_id: u64,
        channel_id: u64,
        server_id: u64,
        author_id: u64,
        content: &str,
        created_at_unix: u64,
    ) -> tantivy::Result<()> {
        tracing::debug!(
            message_id,
            channel_id,
            server_id,
            author_id,
            content_len = content.len(),
            "indexing message"
        );

        self.writer.add_document(doc!(
            self.schema.message_id => message_id,
            self.schema.channel_id => channel_id,
            self.schema.server_id => server_id,
            self.schema.author_id => author_id,
            self.schema.content => content,
            self.schema.created_at => created_at_unix,
        ))?;

        Ok(())
    }

    /// Removes a message from the index by message_id.
    ///
    /// # Arguments
    /// * `message_id` - The message to delete
    ///
    /// # Errors
    /// Returns Tantivy error if deletion fails.
    #[instrument]
    pub fn delete_message(&mut self, message_id: u64) -> tantivy::Result<()> {
        tracing::debug!(message_id, "deleting message from index");
        let term = Term::from_field_u64(self.schema.message_id, message_id);
        self.writer.delete_term(term);
        Ok(())
    }

    /// Commits pending index changes to disk.
    ///
    /// All pending documents and deletions are written atomically.
    ///
    /// # Errors
    /// Returns Tantivy error if commit fails.
    #[instrument]
    pub fn commit(&mut self) -> tantivy::Result<()> {
        tracing::debug!("committing index changes");
        self.writer.commit()?;
        tracing::info!("index committed successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_index() -> (TempDir, Index) {
        let dir = TempDir::new().unwrap();
        let schema = SearchSchema::new();
        let index = Index::create_in_dir(dir.path(), schema.schema).unwrap();
        (dir, index)
    }

    #[test]
    fn test_indexer_creation() {
        let (_dir, index) = create_test_index();
        let indexer = SearchIndexer::new(&index, 50_000_000);
        assert!(indexer.is_ok());
    }

    #[test]
    fn test_index_single_message() {
        let (_dir, index) = create_test_index();
        let mut indexer = SearchIndexer::new(&index, 50_000_000).unwrap();

        let result = indexer.index_message(1001, 2001, 3001, 4001, "Hello world", 1704067200);

        assert!(result.is_ok());
    }

    #[test]
    fn test_index_multiple_messages() {
        let (_dir, index) = create_test_index();
        let mut indexer = SearchIndexer::new(&index, 50_000_000).unwrap();

        for i in 0..5 {
            let result = indexer.index_message(
                1000 + i,
                2001,
                3001,
                4001,
                &format!("Message {}", i),
                1704067200 + i,
            );
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_delete_message() {
        let (_dir, index) = create_test_index();
        let mut indexer = SearchIndexer::new(&index, 50_000_000).unwrap();

        indexer
            .index_message(1001, 2001, 3001, 4001, "Test", 1704067200)
            .unwrap();

        let result = indexer.delete_message(1001);
        assert!(result.is_ok());
    }

    #[test]
    fn test_commit() {
        let (_dir, index) = create_test_index();
        let mut indexer = SearchIndexer::new(&index, 50_000_000).unwrap();

        indexer
            .index_message(1001, 2001, 3001, 4001, "Test", 1704067200)
            .unwrap();

        let result = indexer.commit();
        assert!(result.is_ok());
    }
}
