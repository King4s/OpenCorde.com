# OpenCorde Search — Module Index

Full-text search engine using Tantivy (Rust-native Lucene alternative).

## Files

### `lib.rs` (52 lines)
Public API and index lifecycle management.

**Exports:**
- `open_or_create(path: &Path) -> Result<(Index, SearchSchema)>` — Opens or creates an index
- `SearchIndexer` — Document indexing
- `SearchEngine` — Query execution
- `SearchSchema` — Index schema

**Tests:** 2
- `test_open_or_create_new` — Creates new index
- `test_open_or_create_existing` — Opens existing index

### `schema.rs` (99 lines)
Tantivy schema definition for message indexing.

**Exports:**
- `SearchSchema` — Compiled schema with field references
  - `message_id: Field` — u64, stored + indexed
  - `channel_id: Field` — u64, stored + indexed
  - `server_id: Field` — u64, stored + indexed
  - `author_id: Field` — u64, stored + indexed
  - `content: Field` — Text, full-text indexed + stored
  - `created_at: Field` — u64, stored + indexed

**Tests:** 3
- `test_schema_creation` — Schema builds with 6 fields
- `test_schema_field_names` — All expected fields present
- `test_schema_default` — Default impl works

### `indexer.rs` (175 lines)
Document indexing operations (add, delete, commit).

**Exports:**
- `SearchIndexer` — Manages IndexWriter
  - `new(index, heap_size) -> Result<Self>` — Creates with 50MB heap by default
  - `index_message(msg_id, ch_id, srv_id, auth_id, content, timestamp) -> Result<()>` — Add document
  - `delete_message(message_id) -> Result<()>` — Remove document
  - `commit() -> Result<()>` — Flush to disk

**Logging:**
- DEBUG: Index creation, message indexing/deletion, commit operations
- INFO: Successful commits

**Tests:** 6
- `test_indexer_creation` — Creates indexer
- `test_index_single_message` — Indexes one message
- `test_index_multiple_messages` — Indexes 5 messages
- `test_delete_message` — Removes message from index
- `test_commit` — Commits changes

### `searcher.rs` (165 lines)
Query execution and result retrieval.

**Exports:**
- `SearchResult` — Single search result
  - `message_id, channel_id, server_id, author_id, content: String, score: f32`
- `SearchEngine` — Query executor
  - `new(index, schema) -> Self` — Create engine
  - `search(query, server_id, channel_id, limit) -> Result<Vec<SearchResult>>` — Execute search
    - Query: Full-text search string
    - Filters: Optional server_id and channel_id
    - Limit: Clamped 1-1000
    - Returns: Results sorted by relevance score

**Logging:**
- INFO: Search requests (query, limit, filters)
- INFO: Result count after completion
- ERROR: Search execution failures

**Tests:** 8
- `test_search_engine_creation` — Creates engine
- `test_simple_search` — Query returns results
- `test_search_with_no_results` — Empty result set
- `test_search_with_server_filter` — Server filter works
- `test_search_with_channel_filter` — Channel filter works
- `test_search_limit` — Limit respected
- `test_search_result_has_score` — Score populated

## Statistics

- **Total Lines:** ~491 (all under 300 line limit)
- **Public Functions:** 4 (open_or_create, SearchIndexer::new/index_message/delete_message/commit, SearchEngine::new/search)
- **Tests:** 17 total (100% passing)
- **Dependencies:**
  - `tantivy` — Full-text search
  - `tracing` — Structured logging
  - `serde` — Result serialization
  - `anyhow` — Error handling
  - `tempfile` (dev) — Test fixtures

## Integration

The search crate is integrated into the API via:
- `opencorde-api::AppState::search: Option<Arc<SearchEngine>>`
- `GET /api/v1/search?q=query&server_id=X&channel_id=Y&limit=20` endpoint
- `routes::search` module provides REST handler with auth

See `crates/opencorde-api/src/routes/search.rs` for API integration.

## Next Steps

1. Initialize search engine in API startup (currently None)
2. Implement background indexing of messages (on create/edit/delete)
3. Add search result pagination support
4. Add advanced query syntax (AND, OR, phrase search)
5. Add index maintenance (compact, optimize)
