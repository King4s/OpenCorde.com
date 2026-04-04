<!--
	@component VideoGrid
	@purpose Multi-participant video grid for voice/video calls
	@version 1.0.0
	@uses stores/voice (livekitParticipants)
-->
<script lang="ts">
	import { livekitParticipants, videoTracks } from '$lib/stores/voice';

	interface Participant {
		identity: string;
		speaking: boolean;
		muted: boolean;
		videoTrack?: MediaStreamTrack | null;
	}

	// Determine grid columns based on participant count
	function gridCols(count: number): string {
		if (count <= 1) return 'grid-cols-1';
		if (count <= 4) return 'grid-cols-2';
		if (count <= 9) return 'grid-cols-3';
		return 'grid-cols-4';
	}

	// Volume per participant (0-200)
	let volumes = $state<Record<string, number>>({});

	function getVolume(identity: string): number {
		return volumes[identity] ?? 100;
	}

	function setVolume(identity: string, v: number) {
		volumes = { ...volumes, [identity]: v };
		// Wire to LiveKit participant volume if available
		try {
			(window as any).__lk_room?.participants.forEach((p: any) => {
				if (p.identity === identity) p.setVolume(v / 100);
			});
		} catch { /* room not available */ }
	}
</script>

{#if $livekitParticipants.size > 0}
	<div class="p-2 bg-gray-900 border-t border-gray-700">
		<p class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2">Video</p>
		<div class="grid gap-1.5 {gridCols($livekitParticipants.size)}">
			{#each [...$livekitParticipants.values()] as p (p.identity)}
				<div class="relative bg-gray-800 rounded-lg overflow-hidden aspect-video flex items-center justify-center
					{p.speaking ? 'ring-2 ring-gray-400' : ''}">

					<!-- Video element (populated by LiveKit SDK when track is available) -->
					<video
						class="w-full h-full object-cover"
						autoplay
						playsinline
						data-participant={p.identity}
					></video>

					<!-- Avatar placeholder when no video -->
					<div class="absolute inset-0 flex items-center justify-center {$videoTracks.has(p.identity) ? 'hidden' : ''}">
						<div class="w-10 h-10 rounded-full bg-gray-700 flex items-center justify-center text-white font-bold text-sm">
							{p.identity[0]?.toUpperCase() ?? '?'}
						</div>
					</div>

					<!-- Name + status overlay -->
					<div class="absolute bottom-0 left-0 right-0 flex items-center gap-1 bg-black/50 px-1.5 py-0.5">
						<span class="w-1.5 h-1.5 rounded-full flex-shrink-0 {p.speaking ? 'bg-gray-400' : 'bg-gray-500'}"></span>
						<span class="text-white text-xs truncate flex-1">{p.identity}</span>
						{#if p.muted}<span class="text-xs">🔇</span>{/if}
					</div>

					<!-- Volume slider on hover -->
					<div class="absolute top-1 right-1 opacity-0 hover:opacity-100 group-hover:opacity-100 transition-opacity">
						<input
							type="range" min="0" max="200"
							value={getVolume(p.identity)}
							oninput={(e) => setVolume(p.identity, parseInt((e.target as HTMLInputElement).value))}
							class="w-16 h-1 accent-gray-500"
							title="Volume: {getVolume(p.identity)}%"
						/>
					</div>
				</div>
			{/each}
		</div>
	</div>
{/if}
