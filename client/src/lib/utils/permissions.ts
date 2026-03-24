/**
 * @file permissions.ts — Permission bit flag definitions
 * @purpose Define all role permission flags with labels, descriptions, and categories
 * @used-by RolePermissionsTab.svelte, permission checks throughout the client
 */

// Bit values match Discord's permission flag spec exactly.
// Reference: https://docs.discord.com/developers/topics/permissions
export const PERMISSIONS = {
  // General server permissions
  CREATE_INSTANT_INVITE:      { bit: 1n << 0n,  label: 'Create Invites',        desc: 'Create invite links',                               category: 'General' },
  KICK_MEMBERS:               { bit: 1n << 1n,  label: 'Kick Members',          desc: 'Remove members from the server',                    category: 'Moderation' },
  BAN_MEMBERS:                { bit: 1n << 2n,  label: 'Ban Members',            desc: 'Permanently ban members',                           category: 'Moderation' },
  ADMINISTRATOR:              { bit: 1n << 3n,  label: 'Administrator',          desc: 'All permissions, bypasses channel overrides',       category: 'General' },
  MANAGE_CHANNELS:            { bit: 1n << 4n,  label: 'Manage Channels',        desc: 'Create, edit, and delete channels',                 category: 'General' },
  MANAGE_GUILD:               { bit: 1n << 5n,  label: 'Manage Server',          desc: 'Change server name, region, icon',                  category: 'General' },
  ADD_REACTIONS:              { bit: 1n << 6n,  label: 'Add Reactions',          desc: 'Add emoji reactions to messages',                   category: 'Text' },
  VIEW_AUDIT_LOG:             { bit: 1n << 7n,  label: 'View Audit Log',         desc: 'View the server audit log',                         category: 'General' },
  PRIORITY_SPEAKER:           { bit: 1n << 8n,  label: 'Priority Speaker',       desc: 'Others\' volume reduced when you speak',            category: 'Voice' },
  STREAM:                     { bit: 1n << 9n,  label: 'Video / Stream',         desc: 'Share camera and screen in voice channels',         category: 'Voice' },
  VIEW_CHANNEL:               { bit: 1n << 10n, label: 'View Channels',          desc: 'View channels and read messages',                   category: 'General' },
  SEND_MESSAGES:              { bit: 1n << 11n, label: 'Send Messages',          desc: 'Send messages in text channels',                    category: 'Text' },
  SEND_TTS_MESSAGES:          { bit: 1n << 12n, label: 'Send TTS Messages',      desc: 'Send text-to-speech messages',                      category: 'Text' },
  MANAGE_MESSAGES:            { bit: 1n << 13n, label: 'Manage Messages',        desc: 'Delete and pin any message',                        category: 'Moderation' },
  EMBED_LINKS:                { bit: 1n << 14n, label: 'Embed Links',            desc: 'Links auto-embed with previews',                    category: 'Text' },
  ATTACH_FILES:               { bit: 1n << 15n, label: 'Attach Files',           desc: 'Upload files and images',                           category: 'Text' },
  READ_MESSAGE_HISTORY:       { bit: 1n << 16n, label: 'Read Message History',   desc: 'Read past messages in channels',                    category: 'Text' },
  MENTION_EVERYONE:           { bit: 1n << 17n, label: 'Mention @everyone',      desc: 'Ping @everyone, @here, and all roles',              category: 'Text' },
  USE_EXTERNAL_EMOJIS:        { bit: 1n << 18n, label: 'Use External Emojis',    desc: 'Use emojis from other servers',                     category: 'Text' },
  VIEW_GUILD_INSIGHTS:        { bit: 1n << 19n, label: 'View Server Insights',   desc: 'View server analytics and insights',                category: 'General' },
  CONNECT:                    { bit: 1n << 20n, label: 'Connect',                desc: 'Join voice and stage channels',                     category: 'Voice' },
  SPEAK:                      { bit: 1n << 21n, label: 'Speak',                  desc: 'Speak in voice channels',                           category: 'Voice' },
  MUTE_MEMBERS:               { bit: 1n << 22n, label: 'Mute Members',           desc: 'Mute others in voice channels',                     category: 'Voice' },
  DEAFEN_MEMBERS:             { bit: 1n << 23n, label: 'Deafen Members',         desc: 'Deafen others in voice channels',                   category: 'Voice' },
  MOVE_MEMBERS:               { bit: 1n << 24n, label: 'Move Members',           desc: 'Move members between voice channels',               category: 'Voice' },
  USE_VAD:                    { bit: 1n << 25n, label: 'Use Voice Activity',      desc: 'Use voice activity detection (not push-to-talk)',   category: 'Voice' },
  CHANGE_NICKNAME:            { bit: 1n << 26n, label: 'Change Nickname',        desc: 'Change own nickname',                               category: 'General' },
  MANAGE_NICKNAMES:           { bit: 1n << 27n, label: 'Manage Nicknames',       desc: "Change other members' nicknames",                   category: 'General' },
  MANAGE_ROLES:               { bit: 1n << 28n, label: 'Manage Roles',           desc: 'Create and edit roles below your highest role',     category: 'General' },
  MANAGE_WEBHOOKS:            { bit: 1n << 29n, label: 'Manage Webhooks',        desc: 'Create and manage webhooks',                        category: 'General' },
  MANAGE_GUILD_EXPRESSIONS:   { bit: 1n << 30n, label: 'Manage Emojis & Stickers', desc: 'Add, edit, and remove custom emojis/stickers',   category: 'General' },
  USE_APPLICATION_COMMANDS:   { bit: 1n << 31n, label: 'Use Slash Commands',     desc: 'Use application slash commands and context menus', category: 'Text' },
  REQUEST_TO_SPEAK:           { bit: 1n << 32n, label: 'Request to Speak',       desc: 'Request to speak in stage channels',                category: 'Stage' },
  MANAGE_EVENTS:              { bit: 1n << 33n, label: 'Manage Events',          desc: 'Edit and delete scheduled events',                  category: 'Events' },
  MANAGE_THREADS:             { bit: 1n << 34n, label: 'Manage Threads',         desc: 'Archive and delete threads',                        category: 'Text' },
  CREATE_PUBLIC_THREADS:      { bit: 1n << 35n, label: 'Create Public Threads',  desc: 'Create public threads on messages',                 category: 'Text' },
  CREATE_PRIVATE_THREADS:     { bit: 1n << 36n, label: 'Create Private Threads', desc: 'Create invite-only private threads',                category: 'Text' },
  USE_EXTERNAL_STICKERS:      { bit: 1n << 37n, label: 'Use External Stickers',  desc: 'Use stickers from other servers',                   category: 'Text' },
  SEND_MESSAGES_IN_THREADS:   { bit: 1n << 38n, label: 'Send in Threads',        desc: 'Send messages in threads',                          category: 'Text' },
  MODERATE_MEMBERS:           { bit: 1n << 40n, label: 'Timeout Members',        desc: 'Temporarily mute members (timeout)',                category: 'Moderation' },
  CREATE_EVENTS:              { bit: 1n << 44n, label: 'Create Events',          desc: 'Create scheduled events',                           category: 'Events' },
  SEND_VOICE_MESSAGES:        { bit: 1n << 46n, label: 'Send Voice Messages',    desc: 'Send voice message recordings',                     category: 'Text' },
  SEND_POLLS:                 { bit: 1n << 49n, label: 'Send Polls',             desc: 'Create poll messages',                              category: 'Text' },
  PIN_MESSAGES:               { bit: 1n << 51n, label: 'Pin Messages',           desc: 'Pin and unpin messages in channels',                category: 'Moderation' },
  BYPASS_SLOWMODE:            { bit: 1n << 52n, label: 'Bypass Slowmode',        desc: 'Send messages ignoring slowmode restrictions',      category: 'Moderation' },
} as const;

export type PermissionKey = keyof typeof PERMISSIONS;

export const PERMISSION_CATEGORIES = ['General', 'Moderation', 'Text', 'Voice', 'Stage', 'Events'] as const;
export type PermissionCategory = (typeof PERMISSION_CATEGORIES)[number];

/** Check whether a given flag is set in a permissions bigint. */
export function hasPermission(permissions: bigint, flag: bigint): boolean {
  return (permissions & flag) === flag;
}

/** Return a new permissions bigint with the flag toggled on or off. */
export function togglePermission(permissions: bigint, flag: bigint, enabled: boolean): bigint {
  return enabled ? (permissions | flag) : (permissions & ~flag);
}

/** Convert bigint permissions to a number for JSON serialisation (API uses i64/u64). */
export function permissionsToNumber(perms: bigint): number {
  return Number(perms & 0xFFFFFFFFFFFFFFFFn);
}

/** Convert a number from the API into a client-side bigint. */
export function numberToPermissions(n: number): bigint {
  return BigInt(n);
}
