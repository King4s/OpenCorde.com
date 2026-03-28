<!--
  @component VoiceSettings
  @purpose Select mic / camera / speaker devices; persisted to localStorage
  @version 1.0.0
-->
<script lang="ts">
	import { onMount } from 'svelte';
	import { selectedMicId, selectedCamId } from '$lib/stores/voice';

	interface Props {
		onClose: () => void;
	}
	let { onClose }: Props = $props();

	interface DeviceInfo { deviceId: string; label: string; }
	let mics    = $state<DeviceInfo[]>([]);
	let cameras = $state<DeviceInfo[]>([]);
	let speakers = $state<DeviceInfo[]>([]);
	let selectedSpeakerId = $state<string | null>(
		typeof localStorage !== 'undefined' ? localStorage.getItem('oc_speaker') : null
	);

	onMount(async () => {
		// Request permission so labels are populated
		try { await navigator.mediaDevices.getUserMedia({ audio: true }); } catch { /* denied */ }
		const devices = await navigator.mediaDevices.enumerateDevices();
		mics     = devices.filter(d => d.kind === 'audioinput').map(d => ({ deviceId: d.deviceId, label: d.label || `Mic ${d.deviceId.slice(0,6)}` }));
		cameras  = devices.filter(d => d.kind === 'videoinput').map(d => ({ deviceId: d.deviceId, label: d.label || `Camera ${d.deviceId.slice(0,6)}` }));
		speakers = devices.filter(d => d.kind === 'audiooutput').map(d => ({ deviceId: d.deviceId, label: d.label || `Speaker ${d.deviceId.slice(0,6)}` }));
	});

	function saveSpeaker(id: string) {
		selectedSpeakerId = id;
		localStorage.setItem('oc_speaker', id);
	}
</script>

<!-- Backdrop -->
<div class="fixed inset-0 z-50 bg-black/60 flex items-center justify-center"
	role="button" tabindex="-1" aria-label="Close" onclick={onClose} onkeydown={(e) => e.key === 'Escape' && onClose()}>
	<div class="bg-gray-800 border border-gray-700 rounded-xl shadow-2xl w-full max-w-sm mx-4 p-4"
		role="dialog" aria-label="Voice settings"
		onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
		<div class="flex items-center justify-between mb-4">
			<h2 class="text-white font-semibold">Voice & Video Settings</h2>
			<button onclick={onClose} class="text-gray-400 hover:text-white text-lg leading-none">×</button>
		</div>

		<!-- Microphone -->
		<label class="block mb-3">
			<span class="text-xs text-gray-400 mb-1 block">Microphone</span>
			<select
				class="w-full px-2 py-1.5 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-indigo-500"
				value={$selectedMicId ?? ''}
				onchange={(e) => selectedMicId.set((e.target as HTMLSelectElement).value || null)}
			>
				<option value="">Default</option>
				{#each mics as mic}
					<option value={mic.deviceId}>{mic.label}</option>
				{/each}
			</select>
		</label>

		<!-- Camera -->
		<label class="block mb-3">
			<span class="text-xs text-gray-400 mb-1 block">Camera</span>
			<select
				class="w-full px-2 py-1.5 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-indigo-500"
				value={$selectedCamId ?? ''}
				onchange={(e) => selectedCamId.set((e.target as HTMLSelectElement).value || null)}
			>
				<option value="">Default</option>
				{#each cameras as cam}
					<option value={cam.deviceId}>{cam.label}</option>
				{/each}
			</select>
		</label>

		<!-- Speaker -->
		<label class="block mb-4">
			<span class="text-xs text-gray-400 mb-1 block">Speaker</span>
			<select
				class="w-full px-2 py-1.5 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-indigo-500"
				value={selectedSpeakerId ?? ''}
				onchange={(e) => saveSpeaker((e.target as HTMLSelectElement).value)}
			>
				<option value="">Default</option>
				{#each speakers as spk}
					<option value={spk.deviceId}>{spk.label}</option>
				{/each}
			</select>
		</label>

		<button onclick={onClose} class="w-full py-1.5 bg-indigo-600 hover:bg-indigo-700 text-white text-sm rounded transition-colors">
			Done
		</button>
	</div>
</div>
