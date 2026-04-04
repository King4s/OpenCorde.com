# OpenCorde Navigation Map

This document maps the app shell, route hierarchy, and the major drawers/panels that make OpenCorde feel like a desktop client.

## 1) Global shell

- `src/routes/+layout.svelte`
  - Global auth gate for private routes.
  - Tauri server bootstrap (`ServerSetup`) when needed.
  - PWA install prompt banner.
  - Registers the service worker on mount.

## 2) Public routes

- `/`
  - Marketing / landing page.
- `/login`
- `/register`
- `/reset-password`
- `/invite/[code]`

## 3) Authenticated app shell

### `/servers`
`src/routes/servers/+layout.svelte`

Primary chrome for authenticated users:

- Left rail: server list
- Main area: current server / DM destination
- Global helpers: unread indicators, presence, DM entry points

### `/servers/[serverId]`
`src/routes/servers/[serverId]/+layout.svelte`

Server workspace shell:

- Left sidebar: channel list / channel groups
- Top / contextual controls: invite, settings, quick switcher
- Right side overlays / drawers:
  - member list
  - onboarding modal
  - webhook manager
  - quick switcher

### `/servers/[serverId]/channels/[channelId]`
`src/routes/servers/[serverId]/channels/[channelId]/+page.svelte`

Channel conversation workspace:

- Main center: message timeline + composer
- Secondary panes:
  - thread panel
  - pins / info surfaces
  - channel settings modal

### `/servers/[serverId]/forum/[channelId]`
Forum channel list view.

### `/servers/[serverId]/forum/[channelId]/[postId]`
Forum post detail view.

### `/servers/[serverId]/settings`
`src/routes/servers/[serverId]/settings/+page.svelte`

Server administration shell:

- Left nav: stacked settings sections
- Right content panel: one section at a time
- Sections:
  - Overview
  - Roles & Permissions
  - Members
  - Bans
  - Invites
  - Moderation
  - AutoMod
  - Audit Log
  - Integrations
  - Emojis
  - Onboarding

## 4) DM shell

### `/@me`
`src/routes/@me/+layout.svelte`

Direct-message workspace shell:

- Server rail
- DM-specific content area
- Shared presence / unread / navigation behavior

### `/@me/dms/[dmId]`
DM conversation view.

## 5) Major drawers, modals, and floating panels

These are the places where the app behaves like a desktop client instead of a flat webpage:

- `ChannelSettingsModal`
  - floating editor for channel settings + permissions
- `ChannelPermissionsTab`
  - channel override editor
- `QuickSwitcher`
  - command-palette style navigation surface
- `WebhookManager`
  - server integration overlay
- `OnboardingModal`
  - setup flow for server onboarding
- `UserPanel`
  - account / profile / quick actions
- `ThreadPanel`
  - side conversation panel in channels
- `MemberList`
  - server member drawer

## 6) Resizable surface targets

These are the main surfaces that should eventually support drag-resize like a native client:

- Server list rail
- Channel sidebar
- Member list drawer
- Thread panel
- Server settings left nav
- Channel settings modal width/height
- Quick switcher width
- Any future inspector / drawer components

## 7) Current routing pattern

OpenCorde uses file-based SvelteKit routing with nested layouts to keep shell state persistent while the active view changes.

Rule of thumb:

- `+layout.svelte` = shell / persistent chrome
- `+page.svelte` = current content view
- modal components = floating editors and inspectors

## 8) Practical mental model

Think of the app as:

- one global shell
- one workspace shell per area (servers, DMs, settings)
- many floating inspectors on top
- resize-friendly side panels where possible

That structure is what keeps the product feeling like a desktop client instead of a single-page website.
