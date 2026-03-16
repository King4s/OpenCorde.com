# /crates/opencorde-crypto/

Purpose: E2EE layer using OpenMLS (Phase 2).

Entry: src/lib.rs
Status: Placeholder in Phase 1. Full implementation deferred to Phase 2.

Pattern: Separated by concern — identity management, MLS group ops, file encryption, key packages.

| File | Purpose |
|------|---------|
| identity.rs | User identity management, signature keys |
| mls.rs | MLS group creation, message encryption/decryption |
| file_crypto.rs | File-level encryption/decryption |
| key_package.rs | MLS KeyPackage generation and validation |
| mod.rs | Module exports |
