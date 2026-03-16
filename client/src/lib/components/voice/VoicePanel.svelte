<script lang="ts">
	/**
	 * @file Voice controls panel
	 * @purpose Mute/deafen/leave controls when in voice
	 * @depends stores/voice
	 * @version 1.0.0
	 */
	import { inVoice, selfMute, selfDeaf, leaveVoice, toggleMute, toggleDeaf } from '$lib/stores/voice';
</script>

{#if $inVoice}
	<div class="bg-gray-900 p-3 border-t border-gray-700">
		<div class="flex items-center justify-between gap-2">
			<span class="text-green-400 text-xs font-medium truncate">Voice Connected</span>
			<div class="flex gap-1">
				<button
					onclick={toggleMute}
					class="p-1.5 rounded transition-colors {$selfMute
						? 'bg-red-600 hover:bg-red-700'
						: 'bg-gray-700 hover:bg-gray-600'} text-white text-xs"
					title={$selfMute ? 'Unmute' : 'Mute'}
				>
					{$selfMute ? '🔇' : '🎤'}
				</button>
				<button
					onclick={toggleDeaf}
					class="p-1.5 rounded transition-colors {$selfDeaf
						? 'bg-red-600 hover:bg-red-700'
						: 'bg-gray-700 hover:bg-gray-600'} text-white text-xs"
					title={$selfDeaf ? 'Undeafen' : 'Deafen'}
				>
					{$selfDeaf ? '🔇' : '🔊'}
				</button>
				<button
					onclick={leaveVoice}
					class="p-1.5 rounded bg-red-700 hover:bg-red-600 text-white text-xs transition-colors"
					title="Disconnect"
				>
					✕
				</button>
			</div>
		</div>
	</div>
{/if}
