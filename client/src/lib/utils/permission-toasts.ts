/**
 * @file permission-toasts.ts — Map 403 API errors to contextual toast messages
 * @purpose Translate endpoint + error code into human-readable permission denial reasons
 * @used-by api/client.ts
 */

import { toastError } from '$lib/stores/toasts.svelte';

/** Permission name lookup for common endpoints */
const ENDPOINT_PERMISSIONS: Record<string, string> = {
  '/messages': 'Manage Messages',
  '/pins': 'Pin Messages',
  '/reactions': 'Add Reactions',
  '/members': 'Kick / Ban / Manage Members',
  '/roles': 'Manage Roles',
  '/channels': 'Manage Channels',
  '/webhooks': 'Manage Webhooks',
  '/emojis': 'Manage Emojis & Stickers',
  '/bans': 'Ban Members',
  '/invites': 'Create Instant Invite',
  '/moderation': 'Moderate Members',
  '/threads': 'Manage Threads',
  '/events': 'Manage Events',
  '/settings': 'Manage Server',
  '/nicknames': 'Manage Nicknames',
  '/slowmode': 'Manage Channels',
  '/e2ee': 'Administrator',
};

function guessPermission(path: string): string | undefined {
  for (const [key, perm] of Object.entries(ENDPOINT_PERMISSIONS)) {
    if (path.includes(key)) return perm;
  }
  return undefined;
}

/** Show a contextual permission-denied toast for a 403 response. */
export function showPermissionDenied(path: string, serverMessage?: string) {
  const perm = guessPermission(path);
  let message: string;

  if (serverMessage && serverMessage !== 'forbidden' && serverMessage !== 'FORBIDDEN') {
    // Backend returned a custom reason
    message = serverMessage;
  } else if (perm) {
    message = `You need **${perm}** permission to perform this action.`;
  } else {
    message = 'You do not have permission to perform this action.';
  }

  toastError(message, 'Permission Denied');
}
