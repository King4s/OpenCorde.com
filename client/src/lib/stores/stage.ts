/**
 * @file Stage store — manages stage channel sessions and participants
 * @purpose Fetch/manage stage sessions, control speaker roles, raise hands
 * @depends api/client, api/types
 */
import { writable, derived } from "svelte/store";
import api from "$lib/api/client";
import type {
  StageSession,
  StageDetail,
  StageParticipant,
} from "$lib/api/types";

export const stageSession = writable<StageSession | null>(null);
export const stageParticipants = writable<StageParticipant[]>([]);
export const stageLoading = writable(false);
export const stageError = writable<string | null>(null);

// Derived stores for UI convenience
export const speakers = derived(stageParticipants, ($p) =>
  $p
    .filter((p) => p.role === "speaker")
    .sort(
      (a, b) =>
        new Date(a.joined_at).getTime() - new Date(b.joined_at).getTime(),
    ),
);

export const audience = derived(stageParticipants, ($p) =>
  $p
    .filter((p) => p.role === "audience")
    .sort(
      (a, b) =>
        new Date(a.joined_at).getTime() - new Date(b.joined_at).getTime(),
    ),
);

export const handsRaised = derived(audience, ($a) =>
  $a.filter((p) => p.hand_raised),
);

/// Fetch current stage session and participants.
export async function fetchStage(channelId: string): Promise<void> {
  stageLoading.set(true);
  stageError.set(null);
  try {
    const detail = await api.get<StageDetail>(`/channels/${channelId}/stage`);
    stageSession.set(detail.session);
    stageParticipants.set(detail.participants);
  } catch (e: any) {
    stageError.set(e.message || "Failed to fetch stage");
  } finally {
    stageLoading.set(false);
  }
}

/// Start a new stage session on a channel.
export async function startStage(
  channelId: string,
  topic?: string,
): Promise<void> {
  stageLoading.set(true);
  stageError.set(null);
  try {
    const session = await api.post<StageSession>(
      `/channels/${channelId}/stage/start`,
      {
        topic: topic || null,
      },
    );
    stageSession.set(session);
    // Fetch to populate participants
    await fetchStage(channelId);
  } catch (e: any) {
    stageError.set(e.message || "Failed to start stage");
    throw e;
  } finally {
    stageLoading.set(false);
  }
}

/// End the current stage session.
export async function endStage(channelId: string): Promise<void> {
  stageLoading.set(true);
  stageError.set(null);
  try {
    await api.delete(`/channels/${channelId}/stage`);
    stageSession.set(null);
    stageParticipants.set([]);
  } catch (e: any) {
    stageError.set(e.message || "Failed to end stage");
    throw e;
  } finally {
    stageLoading.set(false);
  }
}

/// Join the stage as an audience member.
export async function joinStage(channelId: string): Promise<void> {
  stageLoading.set(true);
  stageError.set(null);
  try {
    const participant = await api.post<StageParticipant>(
      `/channels/${channelId}/stage/join`,
      {},
    );
    // Add or update participant
    stageParticipants.update((list) => {
      const existing = list.find((p) => p.user_id === participant.user_id);
      if (existing) {
        return list.map((p) =>
          p.user_id === participant.user_id ? participant : p,
        );
      }
      return [...list, participant];
    });
  } catch (e: any) {
    stageError.set(e.message || "Failed to join stage");
    throw e;
  } finally {
    stageLoading.set(false);
  }
}

/// Leave the stage.
export async function leaveStage(channelId: string): Promise<void> {
  stageLoading.set(true);
  stageError.set(null);
  try {
    await api.delete(`/channels/${channelId}/stage/leave`);
    // Remove self from participants
    stageParticipants.update((list) =>
      list.filter((p) => p.role !== "audience" || p.hand_raised),
    );
  } catch (e: any) {
    stageError.set(e.message || "Failed to leave stage");
    throw e;
  } finally {
    stageLoading.set(false);
  }
}

/// Raise hand to request speaking privileges.
export async function raiseHand(channelId: string): Promise<void> {
  stageError.set(null);
  try {
    await api.post(`/channels/${channelId}/stage/hand`, { raised: true });
    stageParticipants.update((list) =>
      list.map((p) =>
        p.role === "audience" ? { ...p, hand_raised: true } : p,
      ),
    );
  } catch (e: any) {
    stageError.set(e.message || "Failed to raise hand");
  }
}

/// Lower hand (cancel speaking request).
export async function lowerHand(channelId: string): Promise<void> {
  stageError.set(null);
  try {
    await api.post(`/channels/${channelId}/stage/hand`, { raised: false });
    stageParticipants.update((list) =>
      list.map((p) =>
        p.role === "audience" ? { ...p, hand_raised: false } : p,
      ),
    );
  } catch (e: any) {
    stageError.set(e.message || "Failed to lower hand");
  }
}

/// Promote a participant to speaker.
export async function promoteSpeaker(
  channelId: string,
  userId: string,
): Promise<void> {
  stageError.set(null);
  try {
    await api.patch(`/channels/${channelId}/stage/speakers/${userId}`, {
      speaker: true,
    });
    stageParticipants.update((list) =>
      list.map((p) =>
        p.user_id === userId
          ? { ...p, role: "speaker" as const, hand_raised: false }
          : p,
      ),
    );
  } catch (e: any) {
    stageError.set(e.message || "Failed to promote speaker");
  }
}

/// Demote a speaker to audience.
export async function demoteSpeaker(
  channelId: string,
  userId: string,
): Promise<void> {
  stageError.set(null);
  try {
    await api.patch(`/channels/${channelId}/stage/speakers/${userId}`, {
      speaker: false,
    });
    stageParticipants.update((list) =>
      list.map((p) =>
        p.user_id === userId ? { ...p, role: "audience" as const } : p,
      ),
    );
  } catch (e: any) {
    stageError.set(e.message || "Failed to demote speaker");
  }
}

/// Clear stage state (when channel changes or session ends).
export function clearStage(): void {
  stageSession.set(null);
  stageParticipants.set([]);
  stageError.set(null);
}
