<!--
  @component VideoGrid
  @purpose Multi-participant video grid with per-participant volume control
  @version 1.0.0
  @depends stores/voice
-->
<script lang="ts">
	import { videoTracks, livekitParticipants, activeRoomStore } from '$lib/stores/voice';
	import type { Track } from 'livekit-client';

	/** Svelte action: attach a LiveKit video track to a <video> element */
	function attachTrack(el: HTMLVideoElement, track: Track | null) {
		if (track) track.attach(el);
		return {
			update(newTrack: Track | null) {
				if (newTrack) {
					newTrack.attach(el);
				} else {
					el.srcObject = null;
				}
			},
			destroy() {
				if (track) track.detach(el);
			}
		};
	}

	// Build display list: participants that have a video track
	const videoParticipants = $derived(() => {
		const list: Array<{ identity: string; track: Track }> = [];
		for (const [identity, track] of $videoTracks) {
			list.push({ identity, track });
		}
		return list;
	});

	// Per-participant volume state (0–200%)
	let volumes = $state<Map<string, number>>(new Map());

	function getVolume(identity: string): number {
		return volumes.get(identity) ?? 100;
	}

	function setVolume(identity: string, value: number) {
		volumes = new Map(volumes).set(identity, value);
		// Apply to LiveKit participant
		const room = $activeRoomStore;
		if (!room) return;
		room.remoteParticipants.forEach((p) => {
			if (p.identity === identity) p.setVolume(value / 100);
		});
	}

	// Grid columns: 1 for 2 people, 2 for 3-4, 3 for 5+
	const gridCols = $derived(() => {
		const count = $videoTracks.size;
		if (count <= 2) return 'grid-cols-1';
		if (count <= 4) return 'grid-cols-2';
		return 'grid-cols-3';
	});
</script>

{#if $videoTracks.size > 0}
	<div class="p-2 border-t border-gray-700 bg-gray-900">
		<div class="grid {gridCols()} gap-2">
			{#each videoParticipants() as { identity, track } (identity)}
				<div class="relative rounded overflow-hidden bg-gray-800 aspect-video">
					<!-- svelte-ignore a11y-media-has-caption -->
					<video
						use:attachTrack={track}
						autoplay
						playsinline
						class="w-full h-full object-cover"
					></video>
					<!-- Name overlay -->
					<div class="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/70 to-transparent px-2 py-1 flex items-center justify-between">
						<span class="text-white text-xs truncate">{identity}</span>
						<!-- Volume slider -->
						<input
							type="range"
							min="0"
							max="200"
							value={getVolume(identity)}
							oninput={(e) => setVolume(identity, Number((e.target as HTMLInputElement).value))}
							class="w-16 h-1 accent-indigo-500 cursor-pointer"
							title="Volume: {getVolume(identity)}%"
						/>
					</div>
				</div>
			{/each}
		</div>
	</div>
{/if}
