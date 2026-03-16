# /crates/opencorde-core/src/models/

**Domain model types. One model per file.**

Pattern: Each file defines one primary type with #[derive(Serialize, Deserialize)] and helper methods.

## Files

| File | Lines | Type | Primary Fields |
|------|-------|------|-----------------|
| mod.rs | 21 | Module | Re-exports all model types |
| user.rs | 157 | User, UserProfile, UserStatus | id, username, email, password_hash, avatar_url, status, created_at, updated_at |
| server.rs | 83 | Server | id, name, owner_id, icon_url, description, created_at, updated_at |
| channel.rs | 156 | Channel, ChannelType | id, server_id, name, channel_type (Text/Voice/Category), topic, position, parent_id, created_at, updated_at |
| message.rs | 188 | Message, Attachment | id, channel_id, author_id, content, attachments[], edited_at, created_at |
| member.rs | 110 | Member | user_id, server_id, nickname, role_ids[], joined_at |
| role.rs | 162 | Role | id, server_id, name, permissions, color (RGB u32), position, mentionable, created_at |
| invite.rs | 200 | Invite | code, server_id, creator_id, uses, max_uses (Option), expires_at (Option), created_at |
| voice_state.rs | 134 | VoiceState | user_id, channel_id, session_id, self_mute, self_deaf, joined_at |

## Key Patterns

### Serialization
- All types: `#[derive(Debug, Clone, Serialize, Deserialize)]`
- All timestamps: `chrono::DateTime<Utc>` (with serde support)
- All IDs: `Snowflake` (i64 newtype, serializes as string)
- Optional fields: `Option<T>` with None handling

### Helper Methods
- `User::to_profile()` — Create public profile without credentials
- `Invite::is_expired()` — Check expiration
- `Invite::is_exhausted()` — Check usage limit
- `Invite::is_valid()` — Check both conditions

### Enumerations
- `UserStatus`: Online, Idle, DoNotDisturb, Offline
- `ChannelType`: Text, Voice, Category

## Testing
Each file includes comprehensive unit tests covering:
- Creation and basic field access
- Serialization/deserialization round-trips
- Optional fields (None/Some)
- Edge cases (empty attachments, no nickname, unlimited invites, etc.)
- Multiple instances (different states, variations)

Total: 41 model tests (all passing)

## Constraints
- All files: < 300 lines (max 200 lines for models)
- All types: Public, documented
- All tests: In `#[cfg(test)] mod tests` at file bottom
- All derives: At minimum Debug, Clone, Serialize, Deserialize
