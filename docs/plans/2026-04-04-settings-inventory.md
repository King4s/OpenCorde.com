# OpenCorde Settings Surface Inventory

Date: 2026-04-04

## User settings

Route:
- `/settings`

Current file:
- `client/src/routes/settings/+page.svelte`

What it contains now:
- avatar upload
- username
- email
- status message
- bio
- theme toggle
- message density toggle
- push notifications toggle
- 2FA setup/disable
- data export
- account deletion

What’s missing:
- sidebar/tabbed settings shell
- session/device management
- password change UI
- privacy section
- clearer security section

## Server settings

Route:
- `/servers/[serverId]/settings`

Current panels:
- `OverviewPanel.svelte`
- `ModerationPanel.svelte`
- `InvitesPanel.svelte`
- `BansPanel.svelte`
- `MembersPanel.svelte`
- `IntegrationsPanel.svelte`
- `OnboardingPanel.svelte`

Entry points:
- gear icon in server sidebar
- owner-only access in the layout

What it contains now:
- server rename / description / delete
- verification level and content filter
- invite create/revoke
- ban list / unban
- member kick / ban / timeout
- bridge mappings and slash commands
- onboarding toggle / welcome message

What’s missing:
- server settings shell with left nav
- dedicated security section
- dedicated roles & permissions section
- discovery / visibility settings
- cleaner admin discoverability

## Channel settings

Current surface:
- `client/src/lib/components/modals/ChannelSettingsModal.svelte`

Entry points:
- channel gear/context menu from the channel UI

Current tabs/controls:
- overview
- permissions
- topic/name/nsfw/slowmode
- E2EE toggle

What’s missing:
- resizable modal or drawer behavior
- clearer tab structure for channel tools
- webhooks / pins / recordings in the same floating editor
- stronger permissions context

## Permissions

Current surface:
- `client/src/lib/components/modals/ChannelPermissionsTab.svelte`
- role APIs/stores in `client/src/lib/stores/roles.ts`

What exists now:
- channel-level override editor
- allow/deny/inherit toggles
- role lookup
- role assignment APIs in the store

What’s missing:
- server-wide roles & permissions page
- permission matrix / inheritance overview
- role management UI in the settings shell
- clearer separation between server permissions and channel overrides

## Security

Current surface:
- user 2FA in `/settings`
- moderation controls in server settings
- audit log page `/servers/[serverId]/audit-log`

What’s missing:
- explicit user security section
- explicit server security section
- session/device management UI
- better visibility for security policy controls

## App-shell / installability gaps

Current state:
- route-based SvelteKit app
- some modal/panel surfaces already exist

What’s missing:
- explicit PWA/installable shell behavior
- manifest + install metadata surfaced in the plan
- resizeable major windows/panels
- more desktop-like persistence of context

## Summary

OpenCorde already has the raw parts for settings, moderation, permissions, and security, but they are fragmented. The next implementation step is to turn them into coherent shells with discoverable navigation and resizable surfaces, while keeping the app installable and client-like rather than webpage-like.
