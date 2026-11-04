# Unwired draft components

Parked here on 2026-09-30 while getting CI green again. These components were
added in commit 0790f50 (2026-06-09) but were never rendered anywhere (ProfileTab
only had an unused import of StatusPicker, since removed), and they reference API types, store exports and backend endpoints
that do not exist yet (`Poll`, `MeshPeer`, `InstanceHealth`, presence custom
status, etc.). Living under `src/` they broke `pnpm check` with 39 type errors.

They are outside `src/` so SvelteKit, Vite and svelte-check ignore them.
Prettier still formats them.

To revive one: add the missing types to `src/lib/api/types.ts` (mirror the Rust
response structs), add any missing store exports, `git mv` the file back into
its original folder, and wire it into a route.

| File                                              | Original location        | Blocking gap                                                               |
| ------------------------------------------------- | ------------------------ | -------------------------------------------------------------------------- |
| chat/PollDisplay.svelte, chat/PollComposer.svelte | src/lib/components/chat/ | `Poll`/`PollAnswer` types; poll API contract                               |
| chat/VoiceRecorder.svelte                         | src/lib/components/chat/ | variable named `state` shadows the `$state` rune                           |
| user/StatusPicker.svelte                          | src/lib/components/user/ | presence store exports + custom status fields on `UserProfile`             |
| admin/BrandingPanel.svelte                        | src/routes/admin/        | `InstanceBranding` types (backend exists: routes/admin/branding.rs)        |
| admin/JobsPanel.svelte                            | src/routes/admin/        | `JobInfo`/`JobDetail`/`JobsResponse` types (backend: routes/admin/jobs.rs) |
| admin/UpgradePanel.svelte                         | src/routes/admin/        | `UpgradeStatus` etc. types (backend: routes/admin/upgrade.rs)              |
| admin/HealthPanel.svelte                          | src/routes/admin/        | `InstanceHealth`/`ServiceHealth` types                                     |
| admin/FederationPanel.svelte                      | src/routes/admin/        | `MeshPeer`/`FederationSettings` types                                      |
