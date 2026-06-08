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

// ─── Adaptive Bitrate Controller ───────────────────────────────────────────

/** Bitrate quality tiers — always prefer highest possible */
const BITRATE_TIERS = {
  excellent: { maxBitrate: 192_000, label: '192kbps', maxLossPct: 1, maxRtt: 50 },
  good:      { maxBitrate: 128_000, label: '128kbps', maxLossPct: 3, maxRtt: 100 },
  fair:      { maxBitrate: 64_000,  label: '64kbps',  maxLossPct: 8, maxRtt: 200 },
  poor:      { maxBitrate: 32_000,  label: '32kbps',  maxLossPct: Infinity, maxRtt: Infinity },
} as const;

type TierName = keyof typeof BITRATE_TIERS;

/** Current bitrate state exposed for debugging */
export const bitrateTier = writable<TierName>('excellent');
export const bitrateStats = writable<{
  packetsLost: number; packetsSent: number; rtt: number; lossPct: number;
  availableBitrate: number; currentBitrate: number;
} | null>(null);

let bitrateInterval: ReturnType<typeof setInterval> | null = null;
let tierSamples: TierName[] = [];       // hysteresis: 3 consecutive samples required
let currentTier: TierName = 'excellent';
let audioSender: RTCRtpSender | null = null;

function pickTier(lossPct: number, rtt: number): TierName {
  const tiers: TierName[] = ['excellent', 'good', 'fair', 'poor'];
  for (const tier of tiers) {
    const cfg = BITRATE_TIERS[tier];
    if (lossPct <= cfg.maxLossPct && rtt <= cfg.maxRtt) return tier;
  }
  return 'poor';
}

async function applyBitrate(sender: RTCRtpSender, bitrate: number) {
  try {
    const params = sender.getParameters();
    if (!params.encodings) params.encodings = [{}];
    params.encodings[0].maxBitrate = bitrate;
    await sender.setParameters(params);
  } catch (e) {
    console.warn('[BitrateCtrl] setParameters failed:', e);
  }
}

async function tickBitrate(room: Room) {
  if (!audioSender) return;
  try {
    const pc = (room as any).engine?.publisher?.pc as RTCPeerConnection | undefined;
    if (!pc) { findSenderFromTrack(); return; }

    const report = await pc.getStats(null);
    let packetsLost = 0, packetsSent = 0, rtt = 0, availableBitrate = 0;

    report.forEach(stat => {
      if (stat.type === 'outbound-rtp' && stat.kind === 'audio') {
        packetsLost = stat.packetsLost ?? 0;
        packetsSent = stat.packetsSent ?? 0;
      }
      if (stat.type === 'candidate-pair' && stat.state === 'succeeded') {
        rtt = Math.round(stat.currentRoundTripTime * 1000) || 0;
        availableBitrate = stat.availableOutgoingBitrate ?? 0;
      }
    });

    const lossPct = packetsSent > 0 ? (packetsLost / (packetsSent + packetsLost)) * 100 : 0;
    const tier = pickTier(lossPct, rtt);
    const cfg = BITRATE_TIERS[tier];

    // Update stats store for debugging
    bitrateStats.set({ packetsLost, packetsSent, rtt, lossPct, availableBitrate, currentBitrate: cfg.maxBitrate });

    // Hysteresis: require 3 consecutive samples in the same tier
    tierSamples.push(tier);
    if (tierSamples.length > 5) tierSamples.shift();

    const last3 = tierSamples.slice(-3);
    const stableTier = last3.every(t => t === last3[0]) ? last3[0] : currentTier;

    if (stableTier !== currentTier) {
      console.log(`[BitrateCtrl] Switching: ${currentTier}(${BITRATE_TIERS[currentTier].label}) → ${stableTier}(${cfg.label}) loss=${lossPct.toFixed(1)}% rtt=${rtt}ms`);
      currentTier = stableTier;
      bitrateTier.set(stableTier);
      await applyBitrate(audioSender, BITRATE_TIERS[stableTier].maxBitrate);
    }
  } catch (e) {
    // Silently ignore transient stats errors
  }
}

function findSenderFromTrack() {
  if (audioSender) return;
  // Walk all senders from all RTCPeerConnections
  try {
    // LiveKit stores the sender — try to find it via the audio track
    const senders: RTCRtpSender[] = [];
    // @ts-ignore — access internal RTCPeerConnection
    const pc = (window as any).__lk_pc as RTCPeerConnection | undefined;
    if (pc) {
      senders.push(...pc.getSenders());
    } else {
      // Fallback: scan for any RTCPeerConnection on the page
      // (limited — can't easily enumerate PCs from window scope)
    }
    for (const s of senders) {
      if (s.track?.kind === 'audio') {
        audioSender = s;
        break;
      }
    }
  } catch { /* noop */ }
}

function startBitrateController(room: Room) {
  // Find the audio sender — wait briefly for tracks to be published
  setTimeout(() => {
    if (!audioSender) {
      try {
        const pub = (room.localParticipant as any).getTrackPublication?.('microphone');
        const sender = pub?.audioTrack?.sender;
        if (sender) audioSender = sender;
      } catch { /* internal API, may fail */ }
    }
    if (!audioSender) findSenderFromTrack();

    if (audioSender) {
      console.log('[BitrateCtrl] Controller active, starting at 128kbps');
      // Start at 128kbps (good tier) and let monitoring adjust
      applyBitrate(audioSender, 128_000);
      tierSamples = ['good', 'good', 'good', 'good', 'good'];
      currentTier = 'good';
      bitrateTier.set('good');
    } else {
      console.warn('[BitrateCtrl] No audio sender found, controller idle');
    }
  }, 2000);

  // Poll every 3 seconds
  bitrateInterval = setInterval(() => tickBitrate(room), 3000);
}

function stopBitrateController() {
  if (bitrateInterval) {
    clearInterval(bitrateInterval);
    bitrateInterval = null;
  }
  audioSender = null;
  tierSamples = [];
  currentTier = 'excellent';
  bitrateTier.set('excellent');
  bitrateStats.set(null);
}

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
let joiningInProgress = false;

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

async function prewarmMicrophonePermission(): Promise<void> {
  if (
    typeof navigator === "undefined" ||
    !navigator.mediaDevices?.getUserMedia
  ) {
    return;
  }

  try {
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    stream.getTracks().forEach((track) => track.stop());
  } catch (err) {
    console.warn("[Voice] microphone permission preflight failed:", err);
  }
}

export async function joinVoice(channelId: string): Promise<void> {
  if (joiningInProgress) return;
  if (get(currentVoiceChannelId) === channelId) return;
  joiningInProgress = true;
  // On mobile browsers, microphone permission prompts are more reliable when
  // requested immediately from the user gesture, before any awaited network
  // calls can break activation context.
  if (!get(selfMute)) {
    await prewarmMicrophonePermission();
  }

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
        if (activeRoom !== room) return;
        stopBitrateController();
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
      await room.localParticipant.setMicrophoneEnabled(
        !res.voice_state.self_mute,
      );
    } catch (micErr) {
      console.warn(
        "[Voice] microphone setup failed; staying connected:",
        micErr,
      );
      selfMute.set(true);
      try {
        await room.localParticipant.setMicrophoneEnabled(false);
      } catch {
        // ignore secondary failures
      }
    }
    updateParticipantMap(room);

    // Start adaptive bitrate controller after mic is set up
    startBitrateController(room);
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
  } finally {
    joiningInProgress = false;
  }
}

export async function leaveVoice(): Promise<void> {
  stopBitrateController();
  try {
    if (activeRoom) {
      await activeRoom.disconnect();
    }

    try {
      await api.post("/voice/leave");
    } catch (err: any) {
      // Leaving voice should never log the user out. If the backend rejects the
      // cleanup request (expired token, already cleaned up, temporary auth
      // hiccup), keep the client session intact and just clear local voice state.
      console.warn("[Voice] backend leave cleanup failed:", err);
    }
  } finally {
    activeRoom = null;
    inVoice.set(false);
    currentVoiceChannelId.set(null);
    voiceE2EEActive.set(false);
    participants.set([]);
    videoTracks.set(new Map());
    activeRoomStore.set(null);
    livekitParticipants.set(new Map());
  }
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
