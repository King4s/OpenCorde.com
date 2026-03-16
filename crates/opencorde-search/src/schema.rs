//! # Search Schema
//! Defines the Tantivy schema for message indexing.
//!
//! ## Fields
//! - message_id: stored, indexed (u64)
//! - channel_id: stored, indexed (u64)
//! - server_id: stored, indexed (u64)
//! - author_id: stored, indexed (u64)
//! - content: full-text indexed, stored
//! - created_at: stored, indexed (u64, unix timestamp)

use tantivy::schema::*;

/// Compiled Tantivy schema for message search.
#[derive(Clone, Debug)]
pub struct SearchSchema {
    /// Raw Tantivy schema
    pub schema: Schema,
    /// Field reference for message_id
    pub message_id: Field,
    /// Field reference for channel_id
    pub channel_id: Field,
    /// Field reference for server_id
    pub server_id: Field,
    /// Field reference for author_id
    pub author_id: Field,
    /// Field reference for message content (full-text searchable)
    pub content: Field,
    /// Field reference for creation timestamp
    pub created_at: Field,
}

impl SearchSchema {
    /// Creates a new SearchSchema with default field configuration.
    pub fn new() -> Self {
        tracing::debug!("creating new search schema");
        let mut builder = Schema::builder();

        let message_id = builder.add_u64_field("message_id", STORED | INDEXED);
        let channel_id = builder.add_u64_field("channel_id", STORED | INDEXED);
        let server_id = builder.add_u64_field("server_id", STORED | INDEXED);
        let author_id = builder.add_u64_field("author_id", STORED | INDEXED);
        let content = builder.add_text_field("content", TEXT | STORED);
        let created_at = builder.add_u64_field("created_at", STORED | INDEXED);

        SearchSchema {
            schema: builder.build(),
            message_id,
            channel_id,
            server_id,
            author_id,
            content,
            created_at,
        }
    }
}

impl Default for SearchSchema {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let schema = SearchSchema::new();
        assert_eq!(schema.schema.fields().count(), 6);
    }

    #[test]
    fn test_schema_field_names() {
        let schema = SearchSchema::new();
        let field_names: Vec<_> = schema
            .schema
            .fields()
            .map(|(_, entry)| entry.name())
            .collect();

        assert!(field_names.contains(&"message_id"));
        assert!(field_names.contains(&"channel_id"));
        assert!(field_names.contains(&"server_id"));
        assert!(field_names.contains(&"author_id"));
        assert!(field_names.contains(&"content"));
        assert!(field_names.contains(&"created_at"));
    }

    #[test]
    fn test_schema_default() {
        let schema = SearchSchema::default();
        assert_eq!(schema.schema.fields().count(), 6);
    }
}
