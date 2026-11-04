/**
 * @file E2EE store — manages MLS group states for E2EE channels
 * @purpose Persist MLS group state blobs, join groups via welcome messages,
 *          expose group state for voice key export.
 * @depends api/client, @tauri-apps/api/core (Tauri crypto commands)
 */
import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import api from "$lib/api/client";
import { channels } from "./channels";

// Map<channelId, groupStateHex> — group states live in-memory for the session.
// Each channel has its own MLS group (text and voice channels are separate groups).
export const e2eeGroupStates = writable<Map<string, string>>(new Map());

// ─── Helpers ─────────────────────────────────────────────────────────────────

/** Convert base64url (no-pad) string to hex string for Tauri commands. */
export function base64urlToHex(b64: string): string {
  const padded = b64.replace(/-/g, "+").replace(/_/g, "/");
  const fullPad = padded.padEnd(
    padded.length + ((4 - (padded.length % 4)) % 4),
    "=",
  );
  const binary = atob(fullPad);
  return Array.from(binary)
    .map((c) => c.charCodeAt(0).toString(16).padStart(2, "0"))
    .join("");
}

/** Convert hex string to base64url (no-pad) for server API. */
export function hexToBase64url(hex: string): string {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.slice(i, i + 2), 16);
  }
  const binary = String.fromCharCode(...bytes);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
}

/** Convert hex to Uint8Array (for passing to LiveKit key provider). */
export function hexToBytes(hex: string): Uint8Array {
  const arr = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    arr[i / 2] = parseInt(hex.slice(i, i + 2), 16);
  }
  return arr;
}

// ─── State Accessors ─────────────────────────────────────────────────────────

export function getGroupState(channelId: string): string | undefined {
  return get(e2eeGroupStates).get(channelId);
}

export function setGroupState(channelId: string, groupStateHex: string): void {
  e2eeGroupStates.update((map) => {
    map.set(channelId, groupStateHex);
    return map;
  });
}

export function clearGroupState(channelId: string): void {
  e2eeGroupStates.update((map) => {
    map.delete(channelId);
    return map;
  });
}

/** Require known channel metadata before choosing an encryption policy. */
export function isEncryptedChannel(channelId: string): boolean {
  const channel = get(channels).find((entry) => entry.id === channelId);
  if (!channel) {
    throw new Error(
      "Channel security information is unavailable. Reopen the channel.",
    );
  }
  return channel.e2ee_enabled || Boolean(getGroupState(channelId));
}

/** Block features whose wire format does not support encrypted content. */
export function requirePlaintextChannel(
  channelId: string,
  operation: string,
): void {
  if (isEncryptedChannel(channelId)) {
    throw new Error(`${operation} is unavailable in encrypted channels.`);
  }
}

function requireNativeCrypto(): void {
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) {
    throw new Error("Encrypted channels require the desktop app.");
  }
}

/** Validate the crypto boundary before any message or attachment is uploaded. */
export function requireChannelGroupState(
  channelId: string,
): string | undefined {
  if (!isEncryptedChannel(channelId)) return undefined;
  requireNativeCrypto();
  const state = getGroupState(channelId);
  if (!state) {
    throw new Error(
      "Encryption keys are unavailable. Join the encrypted channel before sending.",
    );
  }
  return state;
}

// ─── Group Lifecycle ─────────────────────────────────────────────────────────

/**
 * Join an E2EE group for a channel by fetching and processing the welcome message.
 *
 * Call this when opening a channel that has a pending welcome (i.e., you were
 * added to the group but haven't joined yet). After this call, `getGroupState`
 * returns the group state and voice key export becomes available.
 *
 * Throws if no welcome message is pending (404) or if crypto fails.
 */
export async function joinE2EEGroup(channelId: string): Promise<void> {
  if (getGroupState(channelId)) return;
  requireNativeCrypto();
  const { welcome_message } = await api.get<{ welcome_message: string }>(
    `/channels/${channelId}/e2ee/welcome`,
  );

  const welcomeHex = base64urlToHex(welcome_message);
  const groupStateHex = await invoke<string>("crypto_process_welcome", {
    welcome_hex: welcomeHex,
  });

  setGroupState(channelId, groupStateHex);
}

/**
 * Initialize an E2EE group as the creator.
 *
 * Creates a new MLS group, optionally adds initial members, then POSTs to
 * /e2ee/init so the server can distribute welcome messages to each member.
 *
 * @param channelId - Channel to enable E2EE for
 * @param memberKeyPackages - Map<userId, keyPackageHex> of initial members to add
 */
export async function initE2EEGroup(
  channelId: string,
  memberKeyPackages: Map<string, string>,
): Promise<void> {
  let groupStateHex: string = await invoke<string>("crypto_create_group");
  const memberWelcomes: { user_id: string; welcome_message: string }[] = [];

  for (const [userId, kpHex] of memberKeyPackages) {
    const result = await invoke<{
      commit_hex: string;
      welcome_hex: string;
      group_state_hex: string;
    }>("crypto_add_member", {
      group_state_hex: groupStateHex,
      member_key_package_hex: kpHex,
    });
    groupStateHex = result.group_state_hex;
    memberWelcomes.push({
      user_id: userId,
      welcome_message: hexToBase64url(result.welcome_hex),
    });
  }

  setGroupState(channelId, groupStateHex);

  await api.post(`/channels/${channelId}/e2ee/init`, {
    group_state: "",
    member_welcomes: memberWelcomes,
  });
}

/**
 * Add a new member to an existing E2EE group.
 *
 * Fetches the member's key package from the server, runs crypto_add_member,
 * and relays the public Welcome to the API.
 */
export async function addE2EEMember(
  channelId: string,
  memberId: string,
  memberKeyPackageHex: string,
): Promise<void> {
  const groupStateHex = getGroupState(channelId);
  if (!groupStateHex)
    throw new Error(`no E2EE group state for channel ${channelId}`);

  const result = await invoke<{
    commit_hex: string;
    welcome_hex: string;
    group_state_hex: string;
  }>("crypto_add_member", {
    group_state_hex: groupStateHex,
    member_key_package_hex: memberKeyPackageHex,
  });

  setGroupState(channelId, result.group_state_hex);

  // Only the public Welcome leaves the device. MLS state contains private keys.
  await api.post(`/channels/${channelId}/e2ee/init`, {
    group_state: "",
    member_welcomes: [
      {
        user_id: memberId,
        welcome_message: hexToBase64url(result.welcome_hex),
      },
    ],
  });
}
