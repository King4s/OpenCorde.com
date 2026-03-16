//! # OpenMesh Core
//! Shared domain types, models, and permission system.
//!
//! This is the foundation crate with NO external service dependencies.
//! Contains type definitions, ID generation, permissions, Ed25519 keypairs, and events.
//!
//! ## Modules
//! - `models` — Domain model types (user, server, channel, message, etc.)
//! - `snowflake` — 64-bit time-ordered ID generator with custom epoch
//! - `permissions` — Bitfield permission system with role-based overrides
//! - `events` — Shared event type definitions for WebSocket gateway
//! - `keypair` — Ed25519 keypair generation for mesh identity
//! - `password` — Argon2 password hashing and verification
//!
//! ## Features
//! - No external service dependencies (except crypto libraries)
//! - Fully serializable types (Serialize/Deserialize)
//! - Thread-safe ID generation
//! - Ed25519 keypair generation for non-custodial user identity
//! - Comprehensive unit tests
//!
//! ## Depends On
//! Only standard library + workspace dependencies:
//! - `serde` — Serialization
//! - `chrono` — DateTime handling
//! - `bitflags` — Efficient permission bitmasks
//! - `rand` — For seed generation and keypair generation
//! - `ed25519-dalek` — Ed25519 keypair generation
//! - `hex` — Hex encoding for public key serialization

pub mod events;
pub mod gateway;
pub mod keypair;
pub mod mesh;
pub mod models;
pub mod password;
pub mod permission_compute;
pub mod permissions;
pub mod snowflake;

// Re-export common types at crate root for convenience
pub use events::GatewayEvent;
pub use keypair::generate_keypair;
pub use models::{
    Attachment, Channel, ChannelType, Invite, Member, Message, Role, Server, User, UserProfile,
    UserStatus, VoiceState,
};
pub use password::{hash_password, verify_password};
pub use permissions::{OverwriteType, PermissionOverwrite, Permissions};
pub use snowflake::{Snowflake, SnowflakeGenerator};
