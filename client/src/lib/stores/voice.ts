/**
 * @file Voice store — manages voice channel state with LiveKit WebRTC
 * @purpose Join/leave voice channels, mute/deafen, track participants via LiveKit room.
 *          Enables E2EE for channels that have an active MLS group state.
 * @depends api/client, livekit-client, stores/e2ee, @tauri-apps/api/core
 */
import { writable, get } from "svelte/store";
import api from "$lib/api/client";
import type { VoiceState } from "$lib/api/types";
import {
  Room,
  RoomEvent,
  type RemoteParticipant,
  type Track,
  ExternalE2EEKeyProvider,
  isE2EESupported,
  type RoomOptions,
} from "livekit-client";
import { invoke } from "@tauri-apps/api/core";
import { getGroupState, hexToBytes } from "./e2ee";

export const inVoice = writable(false);
export const currentVoiceChannelId = writable<string | null>(null);
export const selfMute = writable(false);
export const selfDeaf = writable(false);
export const participants = writable<VoiceState[]>([]);
/** True when the current voice session has E2EE active. */
export const voiceE2EEActive = writable(false);

/** LiveKit participants (includes remote speakers) */
export const livekitParticipants = writable<
  Map<string, { identity: string; speaking: boolean; muted: boolean }>
>(new Map());

/** Video tracks: participant identity → Track (updated on TrackSubscribed/Unsubscribed). */
export const videoTracks = writable<Map<string, Track>>(new Map());

/** Reactive reference to the current LiveKit Room (null when not in voice). */
export const activeRoomStore = writable<Room | null>(null);

/** Selected microphone device ID (persisted to localStorage). */
export const selectedMicId = writable<string | null>(
  typeof localStorage !== "undefined" ? localStorage.getItem("oc_mic") : null,
);

/** Selected camera device ID (persisted to localStorage). */
export const selectedCamId = writable<string | null>(
  typeof localStorage !== "undefined" ? localStorage.getItem("oc_cam") : null,
);

// Persist device selections to localStorage
if (typeof localStorage !== "undefined") {
  selectedMicId.subscribe((v) =>
    v ? localStorage.setItem("oc_mic", v) : localStorage.removeItem("oc_mic"),
  );
  selectedCamId.subscribe((v) =>
    v ? localStorage.setItem("oc_cam", v) : localStorage.removeItem("oc_cam"),
  );
}

let activeRoom: Room | null = null;

interface JoinResponse {
  voice_state: VoiceState;
  livekit_token: string;
  livekit_url: string;
}

function updateParticipantMap(room: Room) {
  const map = new Map<
    string,
    { identity: string; speaking: boolean; muted: boolean }
  >();
  // Add local participant
  if (room.localParticipant) {
    const lp = room.localParticipant;
    map.set(lp.identity, {
      identity: lp.identity,
      speaking: lp.isSpeaking,
      muted: lp.isMicrophoneEnabled === false,
    });
  }
  // Add remote participants
  room.remoteParticipants.forEach((rp: RemoteParticipant) => {
    map.set(rp.identity, {
      identity: rp.identity,
      speaking: rp.isSpeaking,
      muted: rp.isMicrophoneEnabled === false,
    });
  });
  livekitParticipants.set(map);
}

export async function joinVoice(channelId: string): Promise<void> {
  // Leave current room if in one. If the LiveKit connection is stale, don't
  // block joining a new channel — just log and continue.
  if (activeRoom) {
    try {
      await activeRoom.disconnect();
    } catch (err) {
      console.warn("[Voice] stale room disconnect failed, continuing:", err);
    } finally {
      activeRoom = null;
      activeRoomStore.set(null);
    }
  }

  let joinedServerSide = false;
  try {
    const res = await api.post<JoinResponse>("/voice/join", {
      channel_id: channelId,
    });
    joinedServerSide = true;
    inVoice.set(true);
    currentVoiceChannelId.set(channelId);
    selfMute.set(res.voice_state.self_mute);
    selfDeaf.set(res.voice_state.self_deaf);
    voiceE2EEActive.set(false);

    // Build room options, enabling E2EE if an MLS group state exists for this channel
    const roomOptions: RoomOptions = {
      audioCaptureDefaults: {
        echoCancellation: true,
        noiseSuppression: true,
        autoGainControl: true,
      },
      adaptiveStream: true,
      dynacast: true,
    };

    const groupState = getGroupState(channelId);
    if (groupState && isE2EESupported()) {
      try {
        const keyHex = await invoke<string>("crypto_export_voice_key", {
          group_state_hex: groupState,
        });
        const keyBytes = hexToBytes(keyHex);
        // Uint8Array.buffer is ArrayBufferLike; slice to get a plain ArrayBuffer
        const keyBuffer: ArrayBuffer = keyBytes.buffer.slice(
          keyBytes.byteOffset,
          keyBytes.byteOffset + keyBytes.byteLength,
        ) as ArrayBuffer;
        const keyProvider = new ExternalE2EEKeyProvider();
        // Worker required by E2EEManagerOptions — use livekit's pre-built worker
        const worker = new Worker(
          new URL("livekit-client/e2ee-worker", import.meta.url),
          { type: "module" },
        );
        roomOptions.e2ee = { keyProvider, worker };
        await keyProvider.setKey(keyBuffer);
        voiceE2EEActive.set(true);
      } catch (err) {
        console.warn(
          "[E2EE] Voice key export failed, joining without E2EE:",
          err,
        );
      }
    }

    // Connect to LiveKit room
    const room = new Room(roomOptions);
    activeRoom = room;
    activeRoomStore.set(room);

    room
      .on(RoomEvent.ParticipantConnected, () => updateParticipantMap(room))
      .on(RoomEvent.ParticipantDisconnected, () => updateParticipantMap(room))
      .on(RoomEvent.ActiveSpeakersChanged, () => updateParticipantMap(room))
      .on(RoomEvent.TrackMuted, () => updateParticipantMap(room))
      .on(RoomEvent.TrackUnmuted, () => updateParticipantMap(room))
      .on(
        RoomEvent.TrackSubscribed,
        (track: Track, _pub: unknown, participant: RemoteParticipant) => {
          if (track.kind === "video") {
            videoTracks.update((m) => {
              const n = new Map(m);
              n.set(participant.identity, track);
              return n;
            });
          }
        },
      )
      .on(
        RoomEvent.TrackUnsubscribed,
        (track: Track, _pub: unknown, participant: RemoteParticipant) => {
          if (track.kind === "video") {
            videoTracks.update((m) => {
              const n = new Map(m);
              n.delete(participant.identity);
              return n;
            });
          }
        },
      )
      .on(RoomEvent.Disconnected, () => {
        inVoice.set(false);
        currentVoiceChannelId.set(null);
        activeRoom = null;
        activeRoomStore.set(null);
        videoTracks.set(new Map());
        livekitParticipants.set(new Map());
      });

    await room.connect(res.livekit_url, res.livekit_token);
    // Enable microphone by default (respecting self_mute). If the device is
    // unavailable or permission is denied, keep the room connected so the user
    // still joins the channel instead of bouncing back out.
    try {
      await room.localParticipant.setMicrophoneEnabled(!res.voice_state.self_mute);
    } catch (micErr) {
      console.warn("[Voice] microphone setup failed; staying connected:", micErr);
      selfMute.set(true);
      try {
        await room.localParticipant.setMicrophoneEnabled(false);
      } catch {
        // ignore secondary failures
      }
    }
    updateParticipantMap(room);
  } catch (err: any) {
    console.error("[Voice] failed to join voice channel:", err);

    // If the backend already registered the voice join, clean it up so the user
    // doesn't get stuck in a ghost voice state.
    if (joinedServerSide) {
      try {
        await api.post("/voice/leave");
      } catch {
        // ignore cleanup errors
      }
    }

    inVoice.set(false);
    currentVoiceChannelId.set(null);
    voiceE2EEActive.set(false);
    videoTracks.set(new Map());
    livekitParticipants.set(new Map());
    activeRoom = null;
    activeRoomStore.set(null);
    alert(err?.message ?? "Failed to join voice channel");
    throw err;
  }
}

export async function leaveVoice(): Promise<void> {
  if (activeRoom) {
    await activeRoom.disconnect();
    activeRoom = null;
  }
  await api.post("/voice/leave");
  inVoice.set(false);
  currentVoiceChannelId.set(null);
  voiceE2EEActive.set(false);
  participants.set([]);
  videoTracks.set(new Map());
  activeRoomStore.set(null);
  livekitParticipants.set(new Map());
}

export async function toggleMute(): Promise<void> {
  const muted = !get(selfMute);
  selfMute.set(muted);
  if (activeRoom) {
    await activeRoom.localParticipant.setMicrophoneEnabled(!muted);
  }
  await api.patch("/voice/state", {
    self_mute: muted,
    self_deaf: get(selfDeaf),
  });
}

export async function toggleDeaf(): Promise<void> {
  const deafened = !get(selfDeaf);
  selfDeaf.set(deafened);
  // Deafen: disable all remote audio tracks
  if (activeRoom) {
    activeRoom.remoteParticipants.forEach((rp: RemoteParticipant) => {
      rp.audioTrackPublications.forEach((pub) => {
        if (pub.track) pub.track.mediaStreamTrack.enabled = !deafened;
      });
    });
  }
  await api.patch("/voice/state", {
    self_mute: get(selfMute),
    self_deaf: deafened,
  });
}

export async function fetchParticipants(channelId: string): Promise<void> {
  const list = await api.get<VoiceState[]>(`/voice/participants/${channelId}`);
  participants.set(list);
}

export const screenSharing = writable(false);

export async function toggleScreenShare(): Promise<void> {
  if (!activeRoom) return;
  const sharing = !get(screenSharing);
  await activeRoom.localParticipant.setScreenShareEnabled(sharing);
  screenSharing.set(sharing);
}
