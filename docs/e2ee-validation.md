# E2EE validation and current boundaries

The updated desktop stores its signer, OpenMLS provider keys, groups, receive cursors,
pending operations and recent decrypted history in an authenticated encrypted local
vault. An operating-system credential store protects the wrapping key. Logout locks
memory without deleting the vault; signing back into the same account/server restores it.
A missing credential-store key or corrupt vault fails closed and preserves the existing
data.

## Verified before deployment

- Full Rust workspace and historical OpenMLS 0.5 serialized-state/message fixtures.
- Actual Tauri IPC, including user IDs above JavaScript's exact integer range,
  account/server isolation, process restart and exact pending-operation recovery.
- Real Linux Secret Service credential storage and independent native process vaults.
- Five-member and twenty-member groups through the actual API, including ciphertext
  projections, lost-response retry without duplication, concurrent sends, staged commit
  conflicts, offline epoch catchup, removal and explicit device rejoin.
- PostgreSQL authorization, single-connection-pool concurrency, atomic legacy-to-v2
  transition, one-time package publication, repeatable Welcome delivery and exact ACK.
- Rendered frontend login, group setup, send, full browser/native restart, recovered
  history and lost-response outbox recovery with exactly one displayed message.
- Private state upload rejection, fail-closed unsupported sends and retained drafts.

CI runs frontend security regressions, PostgreSQL E2EE regressions, native restart tests
with both test keys and the real Linux credential backend, plus workspace and lint gates.
See [the synchronization contract](e2ee-recovery-sync.md) for ordering and recovery rules.
Production validation is recorded separately after deployment.

## Boundaries

Browser MLS cryptography remains unsupported: encrypted sending requires the updated
desktop client. Encrypted edits, scheduling, threads, forums and slash commands remain
unavailable and fail closed.

This supports one active MLS device identity per account in a channel. It does not
synchronize private keys across devices. A lost device must restore its local vault
and OS key or explicitly rejoin after an administrator removes and re-adds it.
Legacy channels require an explicit administrator migration and participant rejoin;
old clients cannot write into the new ordered stream.

Decrypted display history is bounded to 1000 messages per channel and 16 MiB across
the vault; file keys retain 32 epochs. Evicted history is explicitly unavailable
locally rather than replaying consumed ratchets. The encrypted vault has a 64 MiB
limit and fails safely before releasing an operation that cannot be persisted.
Encrypted voice disconnects on observed membership/epoch or account changes and
requires rejoining with current keys.

Stopping private-state uploads cannot retroactively restore secrecy of historical
uploaded material. Existing database blobs and messages are preserved; this change
does not silently erase or replace user state.


## Reproduce the combined local acceptance test

Build the API and the native crypto_ipc integration test binary. Start the API
against a disposable PostgreSQL database, then run scripts/test-e2ee-recovery.py
inside an isolated Secret Service session. Set OPENCORDE_QA_API to the loopback
API URL including /api/v1, OPENCORDE_QA_IPC to the compiled integration-test binary,
and OPENCORDE_TEST_OS_KEYS=1. OPENCORDE_QA_OUTPUT optionally selects a private
evidence directory; otherwise a temporary directory is created.

The harness uses separate loopback source addresses for independent clients,
creates only disposable accounts, and writes protected fixtures for follow-up
rendered tests. Remove the disposable database, keyring and private fixtures
after validation. It deliberately refuses non-loopback API addresses.
