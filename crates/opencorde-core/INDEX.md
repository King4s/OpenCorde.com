# /crates/opencorde-core/

**Foundation crate with no external service dependencies.**

Contains all core types, models, permission system, and event definitions for the OpenCorde platform.

## Entry Point
- `src/lib.rs` (40 lines) — Crate root with module declarations and re-exports

## Core Modules

| Module | Lines | Purpose |
|--------|-------|---------|
| snowflake.rs | 276 | Snowflake ID generator (64-bit, time-ordered, custom epoch) |
| permissions.rs | 185 | Bitfield permission system with role overwrites |
| permission_compute.rs | 145 | Permission computation logic (base + overwrites) |
| gateway.rs | 89 | WebSocket gateway event type definitions |
| events.rs | 234 | Event serialization tests and re-exports |

## Domain Models (src/models/)

| File | Lines | Type | Purpose |
|------|-------|------|---------|
| mod.rs | 21 | Module | Re-export all model types |
| user.rs | 157 | User, UserProfile, UserStatus | User accounts and public views |
| server.rs | 83 | Server | Community/guild representation |
| channel.rs | 156 | Channel, ChannelType | Text/voice/category channels |
| message.rs | 188 | Message, Attachment | Chat messages with files |
| member.rs | 110 | Member | Server membership with roles |
| role.rs | 162 | Role | Permissions, colors, hierarchy |
| invite.rs | 200 | Invite | Server invite links (expiring/limited) |
| voice_state.rs | 134 | VoiceState | Voice connection metadata |

## Key Features

### Snowflake ID Generator
- Format: [42 bits: timestamp][5 bits: worker][5 bits: process][12 bits: sequence]
- Custom epoch: 2024-01-01 00:00:00 UTC
- Lifetime: ~139 years of valid IDs
- Monotonic: Strictly increasing generation
- Serialization: String format (preserves i64 precision in JSON)

### Permission System
- Bitflags (u64) for efficient storage
- Role + member overwrites (member takes precedence)
- Administrator bypass (all checks)
- Default @everyone permissions included
- Fully serializable

### Domain Types
- All derive Serialize/Deserialize (serde)
- IDs use Snowflake type (i64 internally)
- Timestamps use chrono::DateTime<Utc>
- Type-safe enums (UserStatus, ChannelType, etc.)

### Gateway Events
- Serde tagged enum: `{ "type": "Name", "data": {...} }`
- 19 event variants: lifecycle, messages, typing, presence, voice, servers, channels, members
- Full JSON serialization support

## Dependencies (Workspace)
- `serde` / `serde_json` — Serialization
- `chrono` — DateTime (with serde feature)
- `bitflags` — Permission bitmasks
- `rand` — RNG (for ID generation)

## Testing
- **76 tests total** (all passing)
- Snowflake: generation, uniqueness, monotonicity, timestamp extraction
- Permissions: admin override, role/member overwrites, serialization
- Models: creation, serialization, relationships, edge cases
- Events: all variants, JSON round-trip

## Constraints
- **No file over 300 lines** (max: events.rs at 234 lines)
- **All public types**: Serialize, Deserialize, Debug, Clone
- **All modules**: Documented with doc comments
- **All tests**: Unit tests in #[cfg(test)] mod tests at file bottom

## Build Status
✓ Compiles cleanly (cargo check)
✓ All 76 tests pass (cargo test)
✓ No clippy warnings (cargo clippy)
✓ Edition 2024 compliant
