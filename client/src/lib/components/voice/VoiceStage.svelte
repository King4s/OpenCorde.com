<script lang="ts">
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import { inVoice, videoTracks } from '$lib/stores/voice';
	import VideoGrid from './VideoGrid.svelte';

	let container = $state<HTMLDivElement | null>(null);
	let popoutMode = $state(false);
	let fullscreenActive = $state(false);

	async function toggleFullscreen() {
		if (!browser || !container) return;

		try {
			if (document.fullscreenElement) {
				await document.exitFullscreen();
			} else {
				await container.requestFullscreen();
			}
		} catch (err) {
			console.warn('[VoiceStage] fullscreen toggle failed:', err);
		}
	}

	function openPopout() {
		if (!browser) return;
		const url = new URL(window.location.href);
		url.searchParams.set('voicePopout', '1');
		window.open(url.toString(), '_blank', 'popup,width=1280,height=760');
	}

	onMount(() => {
		if (!browser) return;
		popoutMode = new URLSearchParams(window.location.search).get('voicePopout') === '1';
		const onFsChange = () => {
			fullscreenActive = document.fullscreenElement === container;
		};
		document.addEventListener('fullscreenchange', onFsChange);
		onFsChange();
		return () => document.removeEventListener('fullscreenchange', onFsChange);
	});
</script>

{#if $inVoice && $videoTracks.size > 0}
	<div
		bind:this={container}
		class="{popoutMode ? 'fixed inset-0 z-50 bg-black' : 'mx-4 my-3'} {fullscreenActive ? 'bg-black' : ''}"
	>
		<div class="{popoutMode ? 'h-full w-full' : 'rounded-xl border border-gray-700 bg-gray-900'} overflow-hidden shadow-xl">
			<div class="flex items-center justify-between gap-2 border-b border-gray-700 bg-gray-950/70 px-3 py-2">
				<div class="min-w-0">
					<p class="text-xs font-semibold uppercase tracking-wider text-gray-400">Voice video</p>
					<p class="truncate text-sm text-gray-200">Live participants</p>
				</div>
				<div class="flex items-center gap-1">
					<button
						onclick={toggleFullscreen}
						class="rounded bg-gray-800 px-3 py-1.5 text-xs text-white transition-colors hover:bg-gray-700"
						title={fullscreenActive ? 'Exit full screen' : 'Full screen'}
						aria-label={fullscreenActive ? 'Exit full screen' : 'Full screen'}
					>
						{fullscreenActive ? '⤢' : '⛶'}
					</button>
					<button
						onclick={openPopout}
						class="rounded bg-gray-800 px-3 py-1.5 text-xs text-white transition-colors hover:bg-gray-700"
						title="Pop out video"
						aria-label="Pop out video"
					>
						⇱
					</button>
				</div>
			</div>

			<div class="{popoutMode ? 'h-[calc(100%-2.75rem)]' : 'min-h-[18rem]'} flex items-center justify-center p-2">
				<div class="w-full max-w-5xl">
					<VideoGrid />
				</div>
			</div>
		</div>
	</div>
{/if}