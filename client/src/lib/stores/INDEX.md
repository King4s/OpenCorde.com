# /client/src/lib/stores/

Purpose: Svelte stores for client-side state management.

Pattern: One store per domain. camelCase naming. Each exports get(), subscribe(), update() via Svelte stores.

| File | Purpose | State |
|------|---------|-------|
| auth.ts | Current user, auth token | user, token, isAuthenticated |
| servers.ts | User's servers list | servers, currentServer |
| channels.ts | Channels in current server | channels, currentChannel |
| messages.ts | Messages in current channel | messages, loading, error |
| dms.ts | DM channels and messages | dmChannels, activeDmMessages, dmLoading |
| voice.ts | Voice session state | activeVoiceChannel, participants, audioDevices |
| presence.ts | User online status, presence | onlineUsers, afkUsers |
| moderation.ts | Server moderation (ban, kick, timeout) | banList, fetchBans, banUser, unbanUser, kickUser, timeoutUser |
| e2ee.ts | MLS group state management for E2EE channels | e2eeGroupStates, joinE2EEGroup, initE2EEGroup, addE2EEMember |
| pushNotifications.ts | Web Push subscription lifecycle | notificationsEnabled, registerPushToken, unregisterPushToken |
