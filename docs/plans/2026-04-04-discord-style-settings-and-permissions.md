# Discord-Style Settings and Permissions Implementation Plan

> **For Hermes:** Follow this plan step by step, keeping each task small and verifiable before moving on.

**Goal:** Turn the current fragmented settings surfaces into a coherent Discord-like admin experience for user settings, server settings, security, roles, permissions, and channel configuration.

**Architecture:** Keep route-based shells for major settings areas, but use in-place panels/modals for the most common tasks. User settings stay under `/settings`, server settings stay under `/servers/[serverId]/settings`, and channel settings remain a modal/side panel. The biggest change is adding a real Roles & Permissions surface and making security explicit instead of scattered. The whole app should feel installable and native-like: PWA metadata, service worker, offline-safe shell, and a layout that behaves like a desktop app instead of a sequence of web pages. Every major module/window/frame should be resizable so the UI can adapt like a desktop client.

**Tech Stack:** SvelteKit, Svelte 5 runes, existing `api/client`, current stores/components, Tailwind CSS, plus PWA/service-worker support for installability.

---

## Task 1: Map the current settings surfaces and entry points

**Objective:** Write down every existing settings/admin entry point so we can see what is route-based, what is modal-based, and what is missing.

**Files:**
- Create: `docs/plans/2026-04-04-discord-style-settings-and-permissions.md`
- Review: `client/src/routes/settings/+page.svelte`
- Review: `client/src/routes/servers/[serverId]/settings/panels/*.svelte`
- Review: `client/src/lib/components/modals/ChannelSettingsModal.svelte`
- Review: `client/src/lib/components/layout/ChannelSidebar.svelte`

**Steps:**
1. List every user, server, and channel settings surface.
2. Note which ones are routes and which ones are modals/panels.
3. Mark the missing surfaces: security, roles, permissions, and discoverability.
4. Verify the list against the codebase before changing anything.

**Verification:**
- We can point to the exact file for each current settings surface.
- We have a clear list of what is missing.

---

## Task 2: Add a proper settings shell for user settings

**Objective:** Make `/settings` feel like a real settings hub instead of a single long page.

**Files:**
- Modify: `client/src/routes/settings/+page.svelte`
- Create: `client/src/lib/components/settings/SettingsSidebar.svelte`
- Create: `client/src/lib/components/settings/SettingsSection.svelte`

**Steps:**
1. Split the current user settings page into left-nav sections.
2. Keep profile, appearance, notifications, security, and privacy as distinct sections.
3. Keep the existing controls, but group them visually and logically.
4. Add a visible “Security” section that contains 2FA and future session controls.

**Verification:**
- `/settings` opens with a settings sidebar.
- Security is no longer buried in the page.
- Existing controls still work.

---

## Task 3: Add a server settings shell with clear navigation

**Objective:** Make server settings look like a real admin console.

**Files:**
- Modify: `client/src/routes/servers/[serverId]/settings/+page.svelte`
- Modify/Create: `client/src/routes/servers/[serverId]/settings/panels/*.svelte`
- Create: `client/src/lib/components/settings/ServerSettingsSidebar.svelte`

**Steps:**
1. Add a left-side nav for server settings sections.
2. Group sections into Overview, Security, Roles & Permissions, Moderation, Members, Invites, Onboarding, Integrations, and Automod.
3. Keep each panel focused on one job.
4. Make the gear icon entry point obvious from the server UI.

**Verification:**
- Server settings no longer feels like a pile of unrelated panels.
- The “Security” and “Roles & Permissions” entries are visible.

---

## Task 4: Build a real Roles & Permissions page

**Objective:** Expose role editing and permission management in one place.

**Files:**
- Create: `client/src/routes/servers/[serverId]/settings/panels/RolesPermissionsPanel.svelte`
- Modify: `client/src/lib/stores/roles.ts`
- Modify: `client/src/lib/components/modals/ChannelPermissionsTab.svelte`

**Steps:**
1. Add a roles list with create/edit/delete actions.
2. Add a server-wide permissions matrix or at minimum a role editor with clear permission toggles.
3. Reuse the existing permission bit logic instead of inventing a second model.
4. Keep the channel override editor, but make it obvious that it is channel-level, not server-wide.

**Verification:**
- There is a visible place to manage roles.
- There is a visible place to manage permissions.
- Channel overrides and server roles are no longer confused.

---

## Task 5: Make server security explicit

**Objective:** Put moderation and access policy into a single security area.

**Files:**
- Create: `client/src/routes/servers/[serverId]/settings/panels/SecurityPanel.svelte`
- Modify: `client/src/routes/servers/[serverId]/settings/panels/ModerationPanel.svelte`
- Modify: `client/src/routes/servers/[serverId]/settings/panels/MembersPanel.svelte`

**Steps:**
1. Move verification level and content filter into a dedicated Security panel.
2. Keep moderation actions like timeout, kick, ban, and audit log linked nearby.
3. Add any missing server access controls that already exist in the backend.
4. Make this section the obvious place for “who can join / speak / send / see”.

**Verification:**
- A server owner can find security settings without guessing.
- Moderation and security are grouped logically.

---

## Task 6: Improve the channel settings modal into a true floating editor

**Objective:** Make channel settings feel more like Discord and less like a page edit form.

**Files:**
- Modify: `client/src/lib/components/modals/ChannelSettingsModal.svelte`
- Modify: `client/src/lib/components/modals/ChannelPermissionsTab.svelte`
- Modify: `client/src/lib/components/layout/ChannelSidebar.svelte`

**Steps:**
1. Keep the modal behavior for channel settings.
2. Add clearer tabs: Overview, Permissions, Webhooks, Pins, Recordings, E2EE if applicable.
3. Keep the permission editor in the modal, but improve the labels and context.
4. Make the channel gear/context menu the primary entry point.

**Verification:**
- Channel settings can be opened without leaving the current channel.
- Permissions are easier to find and understand.

---

## Task 7: Make user settings discoverable from the main UI

**Objective:** Reduce the feeling that user settings are hidden behind a route jump.

**Files:**
- Modify: `client/src/lib/components/layout/UserPanel.svelte`
- Modify: `client/src/lib/components/layout/ChannelSidebar.svelte`
- Modify: `client/src/routes/@me/+layout.svelte` if needed

**Steps:**
1. Add a clearly labeled path to user settings.
2. Ensure the current user panel can open the relevant settings section directly.
3. Make the entry point obvious from the sidebar and/or avatar menu.

**Verification:**
- A user can find settings in one obvious click path.

---

## Task 8: Verify the experience end-to-end

**Objective:** Confirm the new settings structure actually feels coherent.

**Files:**
- Review: all files touched above

**Steps:**
1. Navigate to user settings and confirm the section layout works.
2. Navigate to server settings and confirm security/permissions are visible.
3. Open channel settings and confirm the modal approach still works.
4. Confirm no entry points are broken.
5. Check that nothing important is still hidden behind obscure navigation.

**Verification:**
- Settings are discoverable.
- Permissions are understandable.
- Security is explicit.
- The UI feels closer to Discord and less like a collection of pages.

---

## Execution order

1. Task 1 — inventory
2. Task 2 — user settings shell
3. Task 3 — server settings shell
4. Task 4 — roles & permissions
5. Task 5 — server security
6. Task 6 — channel modal polish
7. Task 7 — discoverability polish
8. Task 8 — end-to-end verification

## Navigation map

**Major routes**
- `/@me` — DM home, friends, personal entry hub
- `/settings` — user settings shell
- `/servers` — server list / join / create hub
- `/servers/[serverId]` — server home / overview
- `/servers/[serverId]/channels/[channelId]` — channel content (chat / voice / forum / stage)
- `/servers/[serverId]/settings` — server settings shell
- `/admin` — instance admin console

**Drawer / modal surfaces**
- Channel settings modal from channel gear/context menu
- Pins panel from channel header
- Thread panel from message actions
- Search modal from channel header
- Webhook manager from channel settings
- Quick switcher from keyboard shortcut / sidebar
- User profile popover from avatars/usernames
- Confirmation dialogs for destructive actions

**Resizable surfaces**
- User settings sidebar and content pane
- Server settings sidebar and content pane
- Permissions editor / roles panel
- Channel settings modal or drawer
- Admin dashboard side panes
- Member list / moderation pane where used

**Behavior rules**
- Big context changes use routes.
- Local tasks use drawers/modals.
- Serious admin tools use resizable panels.
- The shell stays persistent so the app feels like a client.
- Installable PWA behavior is part of the base experience, not a later polish item.

If anything turns up that changes the order, stop and update the plan before coding.
