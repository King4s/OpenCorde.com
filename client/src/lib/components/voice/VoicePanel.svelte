<script lang="ts">
	/**
	 * @file Voice controls panel
	 * @purpose Mute/deafen/leave controls and participant list when in voice
	 * @depends stores/voice, api/client
	 * @version 2.1.0
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

	interface Props {
		/** When true the record button is shown (server owner / manage_channels). */
		canRecord?: boolean;
	}

	let { canRecord = false }: Props = $props();

	let recording = $state(false);

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
	<div class="bg-gray-900 p-3 border-t border-gray-700">
		<!-- Voice status + controls -->
		<div class="flex items-center justify-between gap-2 mb-2">
			<span class="text-green-400 text-xs font-medium truncate flex items-center gap-1">
				<span class="w-1.5 h-1.5 rounded-full bg-green-500 animate-pulse inline-block"></span>
				Voice Connected
			</span>
			<div class="flex gap-1">
				<button
					onclick={toggleMute}
					class="p-1.5 rounded transition-colors {$selfMute
						? 'bg-red-600 hover:bg-red-700'
						: 'bg-gray-700 hover:bg-gray-600'} text-white text-xs"
					title={$selfMute ? 'Unmute' : 'Mute'}
					aria-label={$selfMute ? 'Unmute microphone' : 'Mute microphone'}
				>
					{$selfMute ? '🔇' : '🎤'}
				</button>
				<button
					onclick={toggleDeaf}
					class="p-1.5 rounded transition-colors {$selfDeaf
						? 'bg-red-600 hover:bg-red-700'
						: 'bg-gray-700 hover:bg-gray-600'} text-white text-xs"
					title={$selfDeaf ? 'Undeafen' : 'Deafen'}
					aria-label={$selfDeaf ? 'Undeafen' : 'Deafen'}
				>
					{$selfDeaf ? '🔕' : '🔊'}
				</button>
				<button
					onclick={toggleScreenShare}
					class="p-1.5 rounded transition-colors {$screenSharing
						? 'bg-blue-600 hover:bg-blue-700'
						: 'bg-gray-700 hover:bg-gray-600'} text-white text-xs"
					title={$screenSharing ? 'Stop sharing screen' : 'Share screen'}
					aria-label={$screenSharing ? 'Stop sharing screen' : 'Share screen'}
				>
					🖥
				</button>
				{#if canRecord}
					<button
						onclick={toggleRecording}
						class="p-1.5 rounded transition-colors {recording
							? 'bg-red-600 hover:bg-red-700 animate-pulse'
							: 'bg-gray-700 hover:bg-gray-600'} text-white text-xs"
						title={recording ? 'Stop recording' : 'Start recording'}
						aria-label={recording ? 'Stop recording' : 'Start recording'}
					>
						⏺
					</button>
				{/if}
				<button
					onclick={leaveVoice}
					class="p-1.5 rounded bg-red-700 hover:bg-red-600 text-white text-xs transition-colors"
					title="Disconnect"
					aria-label="Disconnect from voice"
				>
					✕
				</button>
			</div>
		</div>

		<!-- Participant list from LiveKit -->
		{#if $livekitParticipants.size > 0}
			<div class="space-y-1">
				{#each [...$livekitParticipants.values()] as p (p.identity)}
					<div class="flex items-center gap-1.5 text-xs">
						<span class="w-1.5 h-1.5 rounded-full flex-shrink-0 {p.speaking ? 'bg-green-400' : 'bg-gray-600'}"></span>
						<span class="text-gray-300 truncate flex-1">{p.identity}</span>
						{#if p.muted}
							<span class="text-gray-500 text-xs" title="Muted">🔇</span>
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</div>
{/if}
