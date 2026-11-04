# E2EE validation and current boundaries

This change repairs the two-client desktop handshake and prevents the known
plaintext fallback and private-state uploads. It does not constitute a complete
multi-device or arbitrary-size group implementation.

## Verified by regression tests

- Actual production Tauri IPC commands accept snake_case arguments and preserve
  user IDs above JavaScript's exact integer range by receiving strings.
- Two isolated native client states exchange a KeyPackage and Welcome, derive the
  same exporter secret, and encrypt/decrypt messages, replies, and file bytes.
- Frontend wire codecs convert native hex to the API's base64url and back.
- PostgreSQL Welcome delivery returns the original payload exactly once, including
  simultaneous callers; outsiders cannot consume another member's invitation.
- Nonempty private MLS state uploads are rejected before writes. New relay rows
  contain empty state markers. Existing database blobs are left unchanged.
- Browser sends, missing-key sends, encryption failures, and unsupported encrypted
  edits, scheduling, threads, forums, and slash commands fail before plaintext
  transport. The composer keeps its draft when delivery fails.

CI runs the frontend security suite, PostgreSQL regression, and headless native
IPC test in addition to the normal workspace, lint, and desktop checks.

## Boundaries that remain

Browser MLS cryptography is not implemented. Encrypted-channel sending requires
the desktop client; the browser displays a clear error without sending plaintext.

Native signers, key-package private material, and group-state maps remain
session-local. The serialized MLS format is unchanged, and the historical fixture
is tested, but there is no new user-facing restart or multi-device recovery system.

Multi-peer initialization adds members across separate epochs; complete commit
distribution and synchronization for arbitrary membership changes remains
unfinished. The validation target is two independent native clients, not a claim
that arbitrary-size groups are ready.

Stopping new private-state uploads does not erase historical copies. This change
does not delete or rewrite existing user state, nor claim to retroactively restore
the secrecy of previously uploaded key material.

Live validation uses disposable accounts and a private test server. Its native
probe invokes these production IPC handlers and extracts the production frontend
wire codecs rather than implementing independent encryption or encoding logic.
The live result is recorded separately after deployment; passing local tests alone
is not a claim of production validation.
