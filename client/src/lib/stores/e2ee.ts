/**
 * Native owns private keys, ratchets and recovery. This store contains only
 * opaque channel handles and public relay metadata, never MLS snapshots.
 */
import { writable, get } from "svelte/store";
import { invoke } from "@tauri-apps/api/core";
import api from "$lib/api/client";
import { channels } from "./channels";
import type { Attachment } from "$lib/api/types";

interface ChannelInfo {
  channel_id: string;
  sequence: string;
  epoch: string;
  active: boolean;
  welcome_ack: string | null;
}
interface Prepared {
  channel_id: string;
  operation: { operation_id: string; [key: string]: unknown };
}
interface RelayEvent {
  operation_id: string;
  sequence: string;
  epoch: string;
  message_id?: string;
  [key: string]: unknown;
}
interface SessionInfo {
  session_id: string;
  device_id: string;
  key_package_hex: string | null;
  channels: ChannelInfo[];
  pending: Prepared | null;
  retry_channel: string | null;
  recovered_drafts: Record<string, string>;
}
interface Context {
  id: string;
  server: string;
  generation: number;
}
let context: Context | null = null;
let generation = 0;
let queue: Promise<unknown> = Promise.resolve();
export const e2eeGroupStates = writable<Map<string, string>>(new Map());
export const e2eeRetryChannel = writable<string | null>(null);
export const recoveredDrafts = writable<Map<string, string>>(new Map());
export const e2eeErrors = writable<Map<string, string>>(new Map());

export function base64urlToHex(value: string): string {
  const binary = atob(value.replace(/-/g, "+").replace(/_/g, "/"));
  return Array.from(binary, (char) =>
    char.charCodeAt(0).toString(16).padStart(2, "0"),
  ).join("");
}
export function hexToBase64url(value: string): string {
  if (!/^(?:[0-9a-fA-F]{2})*$/.test(value))
    throw new Error("Invalid native encoding");
  let binary = "";
  for (let i = 0; i < value.length; i += 2)
    binary += String.fromCharCode(parseInt(value.slice(i, i + 2), 16));
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=/g, "");
}
export function hexToBytes(value: string): Uint8Array {
  return Uint8Array.from(value.match(/../g) ?? [], (byte) =>
    parseInt(byte, 16),
  );
}
export function isNativeCrypto(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
function serverAddress(): string {
  return (
    localStorage.getItem("opencorde_server")?.replace(/\/$/, "") ||
    window.location.origin
  );
}
function requireContext(): Context {
  if (!isNativeCrypto())
    throw new Error("Encrypted channels require the desktop app.");
  if (!context || context.server !== serverAddress())
    throw new Error(
      "Encryption is locked. Sign in to unlock this server's local vault.",
    );
  return context;
}
function check(ctx: Context): void {
  if (
    context !== ctx ||
    ctx.generation !== generation ||
    ctx.server !== serverAddress()
  ) {
    throw new Error("Encryption session changed. Reopen the channel.");
  }
}
async function native<T>(
  ctx: Context,
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  check(ctx);
  const result = await invoke<T>(command, { ...args, session_id: ctx.id });
  check(ctx);
  return result;
}
function serialized<T>(work: (ctx: Context) => Promise<T>): Promise<T> {
  const ctx = requireContext();
  const pending = queue
    .catch(() => {})
    .then(() => {
      check(ctx);
      return work(ctx);
    });
  queue = pending.then(
    () => {},
    () => {},
  );
  return pending;
}
export const e2eeEpochs = writable<Map<string, string>>(new Map());
function publish(info: SessionInfo): void {
  e2eeEpochs.set(
    new Map(
      info.channels
        .filter((channel) => channel.active)
        .map((channel) => [channel.channel_id, channel.epoch]),
    ),
  );
  e2eeRetryChannel.set(info.retry_channel);
  recoveredDrafts.set(new Map(Object.entries(info.recovered_drafts ?? {})));
  e2eeGroupStates.set(
    new Map(
      info.channels
        .filter((channel) => channel.active)
        .map((channel) => [channel.channel_id, channel.channel_id]),
    ),
  );
}
async function status(ctx: Context): Promise<SessionInfo> {
  const info = await native<SessionInfo>(ctx, "crypto_status");
  publish(info);
  return info;
}
export async function lockE2EE(): Promise<void> {
  const old = context;
  generation++;
  context = null;
  e2eeGroupStates.set(new Map());
  e2eeEpochs.set(new Map());
  e2eeErrors.set(new Map());
  e2eeRetryChannel.set(null);
  recoveredDrafts.set(new Map());
  if (old) await invoke("crypto_lock", { session_id: old.id });
}
export async function openE2EE(userId: string): Promise<void> {
  await lockE2EE();
  if (!isNativeCrypto()) return;
  const version = generation;
  const server = serverAddress();
  const info = await invoke<SessionInfo>("crypto_init", {
    server_url: server,
    user_id: userId,
  });
  if (version !== generation || server !== serverAddress()) {
    await invoke("crypto_lock", { session_id: info.session_id });
    throw new Error("Account changed while unlocking encryption.");
  }
  context = { id: info.session_id, server, generation: version };
  publish(info);
  await serialized(async (ctx) => {
    await drain(ctx);
    let ready = await native<SessionInfo>(ctx, "crypto_key_package", {
      confirmed_hex: null,
    });
    if (ready.key_package_hex) {
      const hex = ready.key_package_hex;
      await api.post("/users/me/key-packages", {
        key_package: hexToBase64url(hex),
      });
      check(ctx);
      ready = await native<SessionInfo>(ctx, "crypto_key_package", {
        confirmed_hex: hex,
      });
    }
    publish(ready);
  });
}
export function getGroupState(channelId: string): string | undefined {
  return get(e2eeGroupStates).get(channelId);
}
export function isEncryptedChannel(channelId: string): boolean {
  const channel = get(channels).find((item) => item.id === channelId);
  if (!channel)
    throw new Error(
      "Channel security information is unavailable. Reopen the channel.",
    );
  return channel.e2ee_enabled || Boolean(getGroupState(channelId));
}
export function requirePlaintextChannel(
  channelId: string,
  operation: string,
): void {
  if (isEncryptedChannel(channelId))
    throw new Error(`${operation} is unavailable in encrypted channels.`);
}
export function requireChannelGroupState(
  channelId: string,
): string | undefined {
  if (!isEncryptedChannel(channelId)) return undefined;
  requireContext();
  if (!getGroupState(channelId))
    throw new Error(
      "Encryption keys are unavailable. Open the channel to recover or request an invitation.",
    );
  return channelId;
}
export function cryptoArguments(channelId: string): {
  session_id: string;
  channel_id: string;
} {
  const ctx = requireContext();
  return { session_id: ctx.id, channel_id: channelId };
}
function isConflict(error: unknown): boolean {
  return (
    (error as { status?: number; code?: string }).status === 409 ||
    (error as { code?: string }).code === "CONFLICT"
  );
}
async function drain(ctx: Context): Promise<void> {
  for (let attempt = 0; attempt < 4; attempt++) {
    const info = await status(ctx);
    if (info.retry_channel) {
      await catchup(ctx, info.retry_channel);
      await native(ctx, "crypto_retry_operation");
      continue;
    }
    if (!info.pending) return;
    const pending = info.pending;
    try {
      const event = await api.post<RelayEvent>(
        `/channels/${pending.channel_id}/e2ee/v2/events`,
        pending.operation,
      );
      check(ctx);
      await native(ctx, "crypto_accept_event", { event });
      await status(ctx);
      return;
    } catch (error) {
      if (!isConflict(error)) throw error;
      await native(ctx, "crypto_reject_conflict", {
        operation_id: pending.operation.operation_id,
      });
    }
  }
  throw new Error(
    "Concurrent channel changes paused delivery. The pending operation is stored safely; synchronize again.",
  );
}

async function sync(ctx: Context, channelId: string): Promise<ChannelInfo> {
  await drain(ctx);
  return catchup(ctx, channelId);
}
async function catchup(ctx: Context, channelId: string): Promise<ChannelInfo> {
  let info = await status(ctx);
  let channel = info.channels.find((item) => item.channel_id === channelId);
  if (!channel || !channel.active) {
    let welcome;
    try {
      welcome = await api.get(`/channels/${channelId}/e2ee/v2/welcome`);
    } catch (error) {
      check(ctx);
      throw new Error(
        "No recoverable invitation is available. Ask an administrator to remove and re-invite this device, or explicitly recreate a legacy encrypted channel.",
      );
    }
    check(ctx);
    await native(ctx, "crypto_join", { channel_id: channelId, welcome });
    info = await status(ctx);
    channel = info.channels.find((item) => item.channel_id === channelId);
  }
  if (!channel) throw new Error("Encrypted channel recovery failed.");
  if (channel.welcome_ack) {
    await api.post(`/channels/${channelId}/e2ee/v2/welcome/ack`, {
      operation_id: channel.welcome_ack,
    });
    check(ctx);
    await native(ctx, "crypto_ack_welcome", {
      channel_id: channelId,
      operation_id: channel.welcome_ack,
    });
  }
  let more = true;
  while (more) {
    const page = await api.get<{
      events: RelayEvent[];
      has_more: boolean;
      sequence: string;
      epoch: string;
    }>(
      `/channels/${channelId}/e2ee/v2/events?after=${channel.sequence}&limit=100`,
    );
    check(ctx);
    if (page.events.length) {
      await native(ctx, "crypto_apply_events", {
        channel_id: channelId,
        events: page.events,
      });
      channel = (await status(ctx)).channels.find(
        (item) => item.channel_id === channelId,
      )!;
    }
    if (page.has_more && !page.events.length)
      throw new Error(
        "Encrypted synchronization returned an empty continuation.",
      );
    if (
      !page.has_more &&
      (page.sequence !== channel.sequence || page.epoch !== channel.epoch)
    ) {
      throw new Error(
        "Encrypted synchronization is incomplete. Reconnect and try again.",
      );
    }
    more = page.has_more;
  }
  e2eeErrors.update((errors) => {
    const next = new Map(errors);
    next.delete(channelId);
    return next;
  });
  return channel;
}
export async function joinE2EEGroup(channelId: string): Promise<void> {
  try {
    await serialized(async (ctx) => {
      await sync(ctx, channelId);
    });
  } catch (error) {
    e2eeErrors.update((errors) =>
      new Map(errors).set(channelId, (error as Error).message),
    );
    throw error;
  }
}
export async function initE2EEGroup(
  channelId: string,
  members: Map<string, string>,
  allowLegacyRejoin = false,
): Promise<void> {
  await serialized(async (ctx) => {
    await drain(ctx);
    await native(ctx, "crypto_prepare_init", {
      channel_id: channelId,
      allow_legacy_rejoin: allowLegacyRejoin,
    });
    await drain(ctx);
    if (members.size) {
      await native(ctx, "crypto_prepare_members", {
        channel_id: channelId,
        added: [...members].map(([user_id, key_package_hex]) => ({
          user_id,
          key_package_hex,
        })),
        removed: null,
      });
      await drain(ctx);
    }
  });
}
export async function addE2EEMember(
  channelId: string,
  memberId: string,
  packageHex: string,
): Promise<void> {
  await serialized(async (ctx) => {
    await sync(ctx, channelId);
    await native(ctx, "crypto_prepare_members", {
      channel_id: channelId,
      added: [{ user_id: memberId, key_package_hex: packageHex }],
      removed: null,
    });
    await drain(ctx);
  });
}
export async function removeE2EEMember(
  channelId: string,
  memberId: string,
): Promise<void> {
  await serialized(async (ctx) => {
    await sync(ctx, channelId);
    await native(ctx, "crypto_prepare_members", {
      channel_id: channelId,
      added: [],
      removed: memberId,
    });
    await drain(ctx);
  });
}
export async function sendEncryptedMessage(
  channelId: string,
  content: string,
  replyToId?: string,
  attachments: Attachment[] = [],
): Promise<void> {
  await serialized(async (ctx) => {
    const channel = await sync(ctx, channelId);
    for (const attachment of attachments) {
      if (attachment.encryption_epoch !== channel.epoch)
        throw new Error(
          "Channel membership changed after attachment encryption. Remove and attach the files again.",
        );
    }
    await native(ctx, "crypto_prepare_message", {
      channel_id: channelId,
      plaintext: content,
      reply_to_id: replyToId ?? null,
      attachments,
    });
    await drain(ctx);
  });
}
export async function cachedMessage(
  channelId: string,
  messageId: string,
  content: string,
): Promise<string> {
  if (!content.startsWith("enc:")) return content;
  if (!isNativeCrypto()) return "[Encrypted message — desktop app required]";
  const ctx = requireContext();
  return (
    (await native<string | null>(ctx, "crypto_cached_message", {
      channel_id: channelId,
      message_id: messageId,
      ciphertext_hex: content.slice(4),
    })) ??
    "[Encrypted history unavailable on this device or outside local retention]"
  );
}

/** Explicit user action preserves the old sealed group until a new Welcome joins. */
export async function requestE2EERejoin(channelId: string): Promise<void> {
  await serialized(async (ctx) => {
    const info = await native<SessionInfo>(ctx, "crypto_request_rejoin", {
      channel_id: channelId,
    });
    publish(info);
    if (info.key_package_hex) {
      await api.post("/users/me/key-packages", {
        key_package: hexToBase64url(info.key_package_hex),
      });
      check(ctx);
      await native(ctx, "crypto_key_package", {
        confirmed_hex: info.key_package_hex,
      });
    }
    await catchup(ctx, channelId);
  });
}

export async function cancelFailedE2EEOperation(
  channelId: string,
): Promise<void> {
  await serialized(async (ctx) => {
    publish(
      await native<SessionInfo>(ctx, "crypto_cancel_retry", {
        channel_id: channelId,
      }),
    );
    e2eeErrors.update((errors) => {
      const next = new Map(errors);
      next.delete(channelId);
      return next;
    });
  });
}
