# Phase 1 source baseline

The adopted source snapshot was assembled from
`600a57cc46ea039af24c782313cdb7aca80aa1a7` on
`feat/phase1-operations-20261103`, including its reconciled Phase 1 working-tree
edits. That SHA remains the source-provenance baseline in the manifest.

The publication worktree on `feat/phase1-loki-20261104` starts from
`origin/main` at `9cddf891284c61497512c2d43166347b88f64df8`. It adopts the
final source snapshot without inheriting the source branch's unpublished commit
history. Historical QA scripts and notes outside the Phase 1 manifest and build
are excluded. Existing source branches, commits, and worktrees are preserved.
The Rust workspace (`Cargo.toml`,
`Cargo.lock`, and `crates/`) comes from the verified
`security-hotfix-20261103-01/sources.tar`, SHA-256
`977376f3527a301bafd86908b921961a5e14debbf8409dcaea4d0e71325a8ba1`.
Its lockfile SHA-256 is
`29d895a3de8393f6e28f2cd8638270b82554c5ecbb0eb51ddac06d44b09b2653`.
The highest database migration is `073_forwarded_messages.sql`. The security
release reported a successful locked offline API build and authenticated
persistent WebSocket/DM checks; it did not report a clean whole-workspace
dependency audit (24 advisory pairs remained).

The web client source and configuration are the 199 tracked, non-native files
listed by the `dm-hotfix-20261103-01/frontend-source-files.json` release
snapshot, SHA-256
`12b48337bdaace1e06e4a19c02356eff6c39fea934b850844d01514c873cd973`.
That snapshot combines login commit `62b25d374efd658b0aa56e71c6abfd5f7df321d5`
with the four released DM source edits. The client package manifests and root
package manifests accompany the source, as do the login/DM smoke tests. The
native `client/src-tauri` tree comes from the adopted source baseline. Generated
`.svelte-kit.bak`, Python bytecode, private logs, runtime output, and router
output are excluded from the publication snapshot; generated files already in
`origin/main` remain unchanged. All 567 manifest-covered adopted files are
retained, including the two source `.bak` files and the retired npm lockfile at
its archived path. No private environment files, dependencies, or build outputs
were adopted. QR was false in the release build; no production
environment file is included here.

The DM release had passing Svelte checks, build, 12 DM checks, and six login
smoke checks. Its release record explicitly leaves whole-client older-API
compatibility and a real authenticated browser session unverified. The source
assembly alone does not establish packaging, egress, or backup/restore
acceptance. The release build artifacts and deployed binary are provenance
references, not files in this worktree.

`reports/phase1-source-manifest.json` freezes SHA-256 hashes for 567 adopted
files. `scripts/verify-phase1.ps1` checks them and enforces migration ceiling
73. Future Phase 1 edits can be passed explicitly through `-AllowModified`
while the verifier reports each changed baseline path; migration files remain
immutable. This is source integrity evidence, not Phase 1 acceptance.

Raw frozen hashes remain recorded. Additional canonical LF hashes allow Git's
normal text-line-ending conversion across Windows and Linux without allowing
content changes. Binary hashes are unchanged. The released pnpm lockfile
(`4c728509b22c8356b063192d8ae3a4b09e7df20c090c6472013b97d5645bd865`)
is authoritative for the web client. The older competing npm lockfile is
preserved byte-for-byte at `reports/legacy-client-package-lock.json`; the
verifier checks that archived path and refuses a duplicate active npm lockfile.
The root npm lockfile is a separate browser-test dependency boundary.
