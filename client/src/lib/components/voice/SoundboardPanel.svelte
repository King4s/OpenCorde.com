<!--
  @component SoundboardPanel
  @purpose Per-server soundboard: list, play, and (owner) upload/delete sounds
  @version 1.0.0
  @uses api/client, stores/voice, stores/auth
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import api from '$lib/api/client';
	import { currentUser } from '$lib/stores/auth';
	import { currentVoiceChannelId } from '$lib/stores/voice';

	interface SoundResponse {
		id: string;
		server_id: string;
		name: string;
		file_key: string;
		volume: number;
		created_at: string;
	}

	interface Props {
		spaceId: string;
		isOwner: boolean;
	}

	let { spaceId, isOwner }: Props = $props();

	let sounds = $state<SoundResponse[]>([]);
	let loading = $state(false);
	let playingId = $state<string | null>(null);
	let error = $state('');

	// Upload form
	let showUpload = $state(false);
	let uploadName = $state('');
	let uploadFileKey = $state('');
	let uploadVolume = $state(100);
	let uploading = $state(false);

	onMount(async () => {
		await loadSounds();
	});

	async function loadSounds() {
		if (!spaceId) return;
		loading = true;
		try {
			sounds = await api.get<SoundResponse[]>(`/servers/${spaceId}/soundboard`);
		} catch {
			sounds = [];
		} finally {
			loading = false;
		}
	}

	async function playSound(soundId: string) {
		if (!$currentVoiceChannelId) {
			error = 'Join a voice channel first';
			return;
		}
		error = '';
		playingId = soundId;
		try {
			await api.post(`/servers/${spaceId}/soundboard/${soundId}/play`, {});
		} catch (e: any) {
			error = e.message ?? 'Failed to play';
		} finally {
			setTimeout(() => { if (playingId === soundId) playingId = null; }, 1500);
		}
	}

	async function deleteSound(soundId: string, name: string) {
		if (!confirm(`Delete "${name}"?`)) return;
		try {
			await api.delete(`/servers/${spaceId}/soundboard/${soundId}`);
			sounds = sounds.filter(s => s.id !== soundId);
		} catch (e: any) {
			error = e.message ?? 'Failed to delete';
		}
	}

	async function handleUpload() {
		if (!uploadName.trim() || !uploadFileKey.trim()) return;
		uploading = true;
		error = '';
		try {
			const s = await api.post<SoundResponse>(`/servers/${spaceId}/soundboard`, {
				name: uploadName.trim(),
				file_key: uploadFileKey.trim(),
				volume: uploadVolume,
			});
			sounds = [...sounds, s];
			uploadName = '';
			uploadFileKey = '';
			uploadVolume = 100;
			showUpload = false;
		} catch (e: any) {
			error = e.message ?? 'Failed to upload';
		} finally {
			uploading = false;
		}
	}
</script>

<div class="bg-gray-850 border-t border-gray-700 p-3">
	<div class="flex items-center justify-between mb-2">
		<span class="text-xs font-semibold text-gray-400 uppercase tracking-wide">Soundboard</span>
		{#if isOwner}
			<button
				onclick={() => showUpload = !showUpload}
				class="text-xs text-gray-400 hover:text-gray-300 transition-colors"
				title="Add sound"
			>+ Add</button>
		{/if}
	</div>

	{#if error}
		<p class="text-gray-400 text-xs mb-2">{error}</p>
	{/if}

	{#if showUpload && isOwner}
		<div class="bg-gray-800 rounded p-2 mb-2 space-y-1.5">
			<input
				type="text"
				bind:value={uploadName}
				placeholder="Sound name (max 32 chars)"
				maxlength="32"
				class="w-full px-2 py-1 bg-gray-900 border border-gray-700 rounded text-white text-xs placeholder-gray-500 outline-none focus:border-gray-500"
			/>
			<input
				type="text"
				bind:value={uploadFileKey}
				placeholder="MinIO file key (e.g. sounds/clip.mp3)"
				class="w-full px-2 py-1 bg-gray-900 border border-gray-700 rounded text-white text-xs placeholder-gray-500 outline-none focus:border-gray-500"
			/>
			<div class="flex items-center gap-2">
				<label for="upload-volume" class="text-xs text-gray-400 flex-shrink-0">Vol</label>
				<input id="upload-volume" type="range" min="10" max="100" bind:value={uploadVolume} class="flex-1 h-1" />
				<span class="text-xs text-gray-400 w-7 text-right">{uploadVolume}%</span>
			</div>
			<button
				onclick={handleUpload}
				disabled={uploading || !uploadName.trim() || !uploadFileKey.trim()}
				class="w-full text-xs py-1 bg-gray-600 hover:bg-gray-700 disabled:opacity-50 text-white rounded"
			>{uploading ? 'Saving...' : 'Add Sound'}</button>
		</div>
	{/if}

	{#if loading}
		<p class="text-gray-500 text-xs text-center py-2">Loading...</p>
	{:else if sounds.length === 0}
		<p class="text-gray-600 text-xs text-center py-2">No sounds yet</p>
	{:else}
		<div class="grid grid-cols-2 gap-1">
			{#each sounds as sound (sound.id)}
				<div class="relative group">
					<button
						onclick={() => playSound(sound.id)}
						class="w-full text-xs px-2 py-1.5 rounded text-left transition-colors truncate
							{playingId === sound.id
								? 'bg-gray-700 text-white'
								: 'bg-gray-800 hover:bg-gray-700 text-gray-300'}"
						title="{sound.name} (vol {sound.volume}%)"
					>🔊 {sound.name}</button>
					{#if isOwner}
						<button
							onclick={() => deleteSound(sound.id, sound.name)}
							class="absolute top-0.5 right-0.5 text-gray-500 opacity-0 group-hover:opacity-100 transition-opacity text-xs leading-none px-1"
							title="Delete"
						>×</button>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>
