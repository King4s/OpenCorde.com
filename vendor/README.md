# OpenMLS 0.5.0 compatibility patch

OpenCorde retains OpenMLS 0.5.0, its 0.2 crypto providers, and all existing
persistence and wire formats. Rust 1.99.1 rejects two ambiguous imports in the
published crate. Both now explicitly select the outgoing transcript hash input
types from `framing::public_message`, which implement the operations these
call sites already use. No cryptographic operations, structures, serialization,
or public APIs are changed.

`openmls-0.5.0/` contains the **entire** published crate, including upstream
tests, vectors, benches, and publication metadata. Only these source files differ:

- `src/group/group_context.rs`: qualify `ConfirmedTranscriptHashInput`.
- `src/group/public_group/mod.rs`: qualify `InterimTranscriptHashInput`.

The upstream MIT license is also included; the published archive omitted it.
Both Cargo workspaces patch crates.io to this same directory. The API Docker
build copies it explicitly. It is excluded from workspace membership.

## Provenance and verification

- Archive: https://static.crates.io/crates/openmls/openmls-0.5.0.crate
- SHA-256: `95d4a03899704c17e7b280676a3301216faccf9b19eb860968f84a492aef944b`
- Publication commit: `837c7a99b46b1d8090551da195c4f56c183af238`
- License: https://github.com/openmls/openmls/blob/837c7a99b46b1d8090551da195c4f56c183af238/LICENSE

Run `python3 scripts/verify-openmls-vendor.py`. It verifies the archive digest
and byte-compares every file after applying exactly the two documented import
edits, checks the license digest, and rejects missing or additional files.
Use `--archive /path/to/openmls-0.5.0.crate` for an offline check.

## Compatibility regression

`crates/opencorde-crypto/tests/fixtures/openmls-0.5.0.json` was generated with
the unpatched crates.io dependency and Rust 1.97.1. It contains disposable test
identities, private keys, serialized two-member group states, one encrypted
message, and expected file/voice exporter keys. Never use these keys outside tests.

The integration test loads these historical states into fresh providers,
decrypts the old message, exchanges replies, persists advanced ratchets and
reloads them, and checks the exporter keys. A truncated wire message is rejected.
The ignored generator is only for intentional baseline regeneration; do not
replace the committed fixture with patched output.

```sh
cargo test -p opencorde-crypto --test openmls_compatibility --locked
cargo test --workspace --locked
cargo clippy --workspace --locked -- -D warnings
cargo check --manifest-path client/src-tauri/Cargo.toml --locked
```

Retire this vendor patch only with explicit state and interoperability migration
evidence for its replacement, or an upstream 0.5-compatible import-only release.
