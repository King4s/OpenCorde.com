/**
 * @file Stage store — manages stage channel sessions (speakers + audience)
 * @purpose Start/end stage, join/leave, raise hand, promote/demote speakers
 * @depends api/client, stores/auth
 */
import { writable, derived, get } from "svelte/store";
import api from "$lib/api/client";
import type { StageSession, StageParticipant, StageDetail } from "$lib/api/types";

// ─── Stores ────────────────────────────────────────────────────────────────

export const stageSession = writable<StageSession | null>(null);
export const stageParticipants = writable<StageParticipant[]>([]);

/** Participants with role "speaker" */
export const speakers = derived(stageParticipants, ($p) =>
  $p.filter((p) => p.role === "speaker"),
);

/** Participants with role "audience" */
export const audience = derived(stageParticipants, ($p) =>
  $p.filter((p) => p.role === "audience"),
);

/** Audience members with hand raised */
export const handsRaised = derived(audience, ($a) =>
  $a.filter((p) => p.hand_raised),
);

/** Number of raised hands (for badge display) */
export const handCount = derived(handsRaised, ($h) => $h.length);

// ─── API Helpers ────────────────────────────────────────────────────────────

/** Fetch current stage state for a channel */
async function fetchStage(channelId: string): Promise<StageDetail | null> {
  try {
    const detail = await api.get<StageDetail>(`/channels/${channelId}/stage`);
    stageSession.set(detail.session);
    stageParticipants.set(detail.participants);
    return detail;
  } catch (e: any) {
    // 404 = no active stage — normal state
    if (e?.status !== 404) {
      console.warn("[Stage] fetch failed:", e);
    }
    stageSession.set(null);
    stageParticipants.set([]);
    return null;
  }
}

// ─── Public API ─────────────────────────────────────────────────────────────

/** Start a stage session (server owner / moderator) */
export async function startStage(
  channelId: string,
  topic?: string,
): Promise<void> {
  const res = await api.post<{ session: StageSession }>(
    `/channels/${channelId}/stage/start`,
    { topic: topic ?? null },
  );
  // Fetch full detail to get participant list
  await fetchStage(channelId);
}

/** End the active stage session */
export async function endStage(channelId: string): Promise<void> {
  await api.delete(`/channels/${channelId}/stage`);
  stageSession.set(null);
  stageParticipants.set([]);
}

/** Join stage as audience */
export async function joinStage(channelId: string): Promise<void> {
  await api.post(`/channels/${channelId}/stage/join`);
  await fetchStage(channelId);
}

/** Leave stage */
export async function leaveStage(channelId: string): Promise<void> {
  await api.delete(`/channels/${channelId}/stage/leave`);
  await fetchStage(channelId);
}

/** Raise hand to request speaking */
export async function raiseHand(channelId: string): Promise<void> {
  await api.post(`/channels/${channelId}/stage/hand`, { raised: true });
  await fetchStage(channelId);
}

/** Lower hand */
export async function lowerHand(channelId: string): Promise<void> {
  await api.post(`/channels/${channelId}/stage/hand`, { raised: false });
  await fetchStage(channelId);
}

/** Promote audience member to speaker */
export async function promoteSpeaker(
  channelId: string,
  userId: string,
): Promise<void> {
  await api.patch(`/channels/${channelId}/stage/speakers/${userId}`, {
    speaker: true,
  });
  await fetchStage(channelId);
}

/** Demote speaker to audience */
export async function demoteSpeaker(
  channelId: string,
  userId: string,
): Promise<void> {
  await api.patch(`/channels/${channelId}/stage/speakers/${userId}`, {
    speaker: false,
  });
  await fetchStage(channelId);
}

/** Refresh stage state (call on mount or WebSocket event) */
export async function refreshStage(channelId: string): Promise<void> {
  await fetchStage(channelId);
}

/** Clear all stage state (call when leaving channel/server) */
export function clearStage() {
  stageSession.set(null);
  stageParticipants.set([]);
}
