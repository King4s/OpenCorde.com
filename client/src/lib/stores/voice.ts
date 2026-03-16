/**
 * @file Voice store — manages voice channel state
 * @purpose Join/leave voice, mute/deafen, track participants
 * @depends api/client, api/types
 */
import { writable } from 'svelte/store';
import api from '$lib/api/client';
import type { VoiceState } from '$lib/api/types';

export const inVoice = writable(false);
export const currentVoiceChannelId = writable<string | null>(null);
export const selfMute = writable(false);
export const selfDeaf = writable(false);
export const participants = writable<VoiceState[]>([]);
export const livekitToken = writable<string | null>(null);

interface JoinResponse { voice_state: VoiceState; livekit_token: string; }

export async function joinVoice(channelId: string): Promise<void> {
  const res = await api.post<JoinResponse>('/voice/join', { channel_id: channelId });
  inVoice.set(true);
  currentVoiceChannelId.set(channelId);
  livekitToken.set(res.livekit_token);
  selfMute.set(res.voice_state.self_mute);
  selfDeaf.set(res.voice_state.self_deaf);
  await fetchParticipants(channelId);
}

export async function leaveVoice(): Promise<void> {
  await api.post('/voice/leave');
  inVoice.set(false);
  currentVoiceChannelId.set(null);
  livekitToken.set(null);
  participants.set([]);
}

export async function toggleMute(): Promise<void> {
  selfMute.update(m => !m);
  let mute = false;
  selfMute.subscribe(v => mute = v)();
  let deaf = false;
  selfDeaf.subscribe(v => deaf = v)();
  await api.patch('/voice/state', { self_mute: mute, self_deaf: deaf });
}

export async function toggleDeaf(): Promise<void> {
  selfDeaf.update(d => !d);
  let mute = false;
  selfMute.subscribe(v => mute = v)();
  let deaf = false;
  selfDeaf.subscribe(v => deaf = v)();
  await api.patch('/voice/state', { self_mute: mute, self_deaf: deaf });
}

export async function fetchParticipants(channelId: string): Promise<void> {
  const list = await api.get<VoiceState[]>(`/voice/participants/${channelId}`);
  participants.set(list);
}
