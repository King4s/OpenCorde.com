<script lang="ts">
	/**
	 * @file Voice controls panel
	 * @purpose Mute/deafen/leave controls, participant list, video grid, device settings
	 * @depends stores/voice, api/client
	 * @version 3.0.0 — VideoGrid + VoiceSettings
	 */
	import {
		inVoice,
		selfMute,
		selfDeaf,
		screenSharing,
		leaveVoice,
		toggleMute,
		toggleDeaf,
		toggleScreenShare,
		livekitParticipants,
		currentVoiceChannelId
	} from '$lib/stores/voice';
	import api from '$lib/api/client';
	import { members } from '$lib/stores/members';
	import VideoGrid from './VideoGrid.svelte';
	import VoiceSettings from './VoiceSettings.svelte';

	interface Props {
		canRecord?: boolean;
	}

	let { canRecord = false }: Props = $props();

	let recording = $state(false);
	let showSettings = $state(false);

	function displayName(userId: string): string {
		const member = $members.find((m) => m.user_id === userId);
		return member?.nickname ?? member?.username ?? userId;
	}

	async function toggleRecording() {
		const channelId = $currentVoiceChannelId;
		if (!channelId) return;
		try {
			if (recording) {
				await api.post(`/channels/${channelId}/recording/stop`);
				recording = false;
			} else {
				await api.post(`/channels/${channelId}/recording/start`);
				recording = true;
			}
		} catch (e) {
			console.error('[Recording] toggle failed:', e);
		}
	}
</script>

{#if $inVoice}
	<div class="bg-gray-900 border-t border-gray-700">
		<div class="p-3">
			<!-- Voice status + controls -->
			<div class="flex items-center justify-between gap-2 mb-2">
				<span class="text-gray-400 text-xs font-medium truncate flex items-center gap-1">
					<span class="w-1.5 h-1.5 rounded-full bg-gray-500 animate-pulse inline-block"></span>
					Voice Connected
				</span>
				<div class="flex gap-1">
					<button
						onclick={toggleMute}
						class="p-1.5 rounded transition-colors {$selfMute ? 'bg-gray-600 hover:bg-gray-700' : 'bg-gray-700 hover:bg-gray-600'} text-white text-xs"
						title={$selfMute ? 'Unmute' : 'Mute'}
						aria-label={$selfMute ? 'Unmute microphone' : 'Mute microphone'}
					>{$selfMute ? '🔇' : '🎤'}</button>
					<button
						onclick={toggleDeaf}
						class="p-1.5 rounded transition-colors {$selfDeaf ? 'bg-gray-600 hover:bg-gray-700' : 'bg-gray-700 hover:bg-gray-600'} text-white text-xs"
						title={$selfDeaf ? 'Undeafen' : 'Deafen'}
						aria-label={$selfDeaf ? 'Undeafen' : 'Deafen'}
					>{$selfDeaf ? '🔕' : '🔊'}</button>
					<button
						onclick={toggleScreenShare}
						class="p-1.5 rounded transition-colors {$screenSharing ? 'bg-gray-600 hover:bg-gray-700' : 'bg-gray-700 hover:bg-gray-600'} text-white text-xs"
						title={$screenSharing ? 'Stop sharing screen' : 'Share screen'}
						aria-label={$screenSharing ? 'Stop sharing screen' : 'Share screen'}
					>🖥</button>
					{#if canRecord}
						<button
							onclick={toggleRecording}
							class="p-1.5 rounded transition-colors {recording ? 'bg-gray-600 hover:bg-gray-700 animate-pulse' : 'bg-gray-700 hover:bg-gray-600'} text-white text-xs"
							title={recording ? 'Stop recording' : 'Start recording'}
							aria-label={recording ? 'Stop recording' : 'Start recording'}
						>⏺</button>
					{/if}
					<button
						onclick={() => showSettings = true}
						class="p-1.5 rounded bg-gray-700 hover:bg-gray-600 text-white text-xs transition-colors"
						title="Voice & Video Settings"
						aria-label="Open voice settings"
					>⚙</button>
					<button
						onclick={leaveVoice}
						class="p-1.5 rounded bg-gray-700 hover:bg-gray-600 text-white text-xs transition-colors"
						title="Disconnect"
						aria-label="Disconnect from voice"
					>✕</button>
				</div>
			</div>

			<!-- Participant list -->
				{#if $livekitParticipants.size > 0}
					<div class="space-y-1">
						{#each [...$livekitParticipants.values()] as p (p.identity)}
							<div class="flex items-center gap-1.5 text-xs">
								<span class="w-1.5 h-1.5 rounded-full flex-shrink-0 {p.speaking ? 'bg-gray-400' : 'bg-gray-600'}"></span>
								<span class="text-gray-300 truncate flex-1">{displayName(p.identity)}</span>
								{#if p.muted}<span class="text-gray-500 text-xs">🔇</span>{/if}
							</div>
						{/each}
					</div>
				{/if}
</div>

		<!-- Video grid -->
		<VideoGrid />
	</div>

	{#if showSettings}
		<VoiceSettings onClose={() => showSettings = false} />
	{/if}
{/if}
