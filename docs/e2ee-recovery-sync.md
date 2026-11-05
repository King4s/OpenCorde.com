# Durable MLS recovery and synchronization contract

Status: implementation contract; deployment requires regression tests and independent review.

## Native ownership

Native desktop owns signer, serializable provider store, group ratchets, cursors and decrypted cache. Account + normalized server URL including path isolated AES-GCM snapshot, wrapping key in OS keyring, exclusive process lock, atomic fsync commit. Missing key/corruption/persistence error fail closed without replacing good bytes. Logout locks memory; same-account login recovers. Private state never leaves native storage.

## Relay API

All IDs, sequence and epoch numbers are decimal strings.

POST /channels/{id}/e2ee/v2/events accepts:

    {operation_id, expected_sequence, epoch, kind, ciphertext, added:[{user_id,welcome}], removed:[user_id]}

kind is init, commit, application. ciphertext and welcome are base64url TLS MLS bytes. Init has empty ciphertext, epoch0, no membership changes; registers creator. A commit is signed MLS bytes at submitted old epoch, resulting epoch increments one. Applications retain epoch and cannot change membership. Commit membership changes require MANAGE_CHANNELS. Targets require VIEW_CHANNEL. Only active MLS members read/write. Removed members lose API access immediately.

Server locks stream row; resolves identical idempotent retry before sequence/epoch CAS, rejects ID reuse with changed bytes. Commit log, membership, pending welcomes, epoch and ordinary message projection commit atomically. Conflict409 never mutates state. Unknown stream404. Legacy mutators fail safely for v2 channels. Existing legacy groups require explicit migration/recreation; never overwrite historical data automatically.

GET /channels/{id}/e2ee/v2/events?after=0&limit=100 returns {events,sequence,epoch,has_more}. Events include operation_id,sequence,epoch (RESULTING),sender_id,kind,ciphertext,added:[user_id],removed:[user_id],message_id (application only). Welcome bytes never appear in event pages. Newly joined members see only events after their join cursor, not historical ciphertext.

GET /channels/{id}/e2ee/v2/welcome is non-consuming and returns {operation_id,sequence,epoch,welcome}. POST same path /ack body {operation_id} acknowledges only the exact pending Welcome after durable join. Repeat ACK is harmless.

## Native operation transaction

Before returning outbound bytes, persist before-state + candidate-state + exact public operation and ID + expected sequence. The entire account vault permits only one pending operation, so candidate provider snapshots cannot rewind unrelated channels. Restart resends exact pending bytes. Acceptance installs candidate and accepted cursor atomically. A definite409 retains the consumed sender ratchet, then catches up and prepares a fresh operation; ambiguous failure never discards pending operation. Incoming MLS events are authenticated; group/provider/cursor and decrypted cache commit atomically. Native verifies event sequence, resulting epoch and membership against authenticated MLS result. Welcome joins and persists before ACK. Catchup runs on entry/reconnect and before outgoing operations.

## Acceptance

Real native IPC with separate process restarts/crashes at key publication, Welcome receipt, encryption, transmission, commit publication and decrypt boundaries. Account/origin isolation, corrupt/missing-key failure. PostgreSQL ordering/races/dedup/atomicity/removed-member denial. Five- and twenty-member groups, offline catchup, add/remove transitions, concurrent proposals. Full workspace, strict lint, frontend and desktop checks plus independent review before deployment.

## Integration details

Application operations may include reply_to_id and attachments; non-application operations reject these fields. Ordinary message projections retain the existing enc:hex format and return message_id. Acknowledged event epoch is the resulting epoch, while submissions carry the old/current epoch.

Native prepared candidates do not mutate the committed snapshot. Accepted operations promote the candidate atomically. Rejected application operations retain the consumed send ratchet at the old cursor. Commits retain a separate post-staging, unmerged rejection snapshot: clearing the pending commit preserves the old-epoch send ratchet. No ciphertext that crossed the network permits its sender ratchet to rewind. Local decrypted history is bounded to 1000 messages per channel within a 64 MiB encrypted vault cap. Display uses only message IDs returned by current ordinary message listings; replaying retained encrypted log entries must not resurrect deleted messages.

Normal channel permissions are checked in addition to MLS membership. Transaction-local authorization uses the transaction connection, avoiding pool exhaustion under concurrent writers. Database guards serialize legacy content writes and group-state mutations with v2 initialization. Only the authenticated relay transaction sets the transaction-local internal message-write authorization. Metadata-only foreign-key reply cleanup remains allowed.

A legacy channel requires an explicit administrator confirmation, then init with allow_legacy_rejoin true. Historical private blobs and messages remain untouched. All members must rejoin the new group. An existing v2 group cannot silently reset; a lost device must restore its local vault/key or be explicitly removed and re-added by an administrator.
