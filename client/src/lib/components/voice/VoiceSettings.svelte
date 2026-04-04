<!--
	@component VoiceSettings
	@purpose Select audio/video input/output devices for voice channel
	@version 1.0.0
-->
<script lang="ts">
	import { onMount } from 'svelte';

	interface Props {
		onClose: () => void;
	}

	let { onClose }: Props = $props();

	interface Device { deviceId: string; label: string; }

	let mics = $state<Device[]>([]);
	let cameras = $state<Device[]>([]);
	let speakers = $state<Device[]>([]);
	let selectedMic = $state(localStorage.getItem('oc_mic') ?? '');
	let selectedCamera = $state(localStorage.getItem('oc_camera') ?? '');
	let selectedSpeaker = $state(localStorage.getItem('oc_speaker') ?? '');
	let permError = $state('');

	onMount(async () => {
		try {
			// Request permission so labels are available
			await navigator.mediaDevices.getUserMedia({ audio: true });
		} catch { /* permission denied — labels will be empty */ }
		try {
			const devices = await navigator.mediaDevices.enumerateDevices();
			mics = devices.filter(d => d.kind === 'audioinput').map(d => ({ deviceId: d.deviceId, label: d.label || `Mic ${d.deviceId.slice(0,6)}` }));
			cameras = devices.filter(d => d.kind === 'videoinput').map(d => ({ deviceId: d.deviceId, label: d.label || `Camera ${d.deviceId.slice(0,6)}` }));
			speakers = devices.filter(d => d.kind === 'audiooutput').map(d => ({ deviceId: d.deviceId, label: d.label || `Speaker ${d.deviceId.slice(0,6)}` }));
		} catch (e: any) {
			permError = e.message ?? 'Could not enumerate devices';
		}
	});

	function save() {
		localStorage.setItem('oc_mic', selectedMic);
		localStorage.setItem('oc_camera', selectedCamera);
		localStorage.setItem('oc_speaker', selectedSpeaker);
		onClose();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- Backdrop -->
<div class="fixed inset-0 z-50 bg-black/60 flex items-end justify-center sm:items-center"
	role="dialog" aria-modal="true" aria-label="Voice Settings">
	<div class="bg-gray-800 border border-gray-700 rounded-xl shadow-2xl w-full max-w-sm mx-4 mb-4 sm:mb-0 p-5">
		<div class="flex items-center justify-between mb-4">
			<h2 class="text-base font-semibold text-white">Voice &amp; Video Settings</h2>
			<button onclick={onClose} class="text-gray-400 hover:text-white text-xl leading-none">×</button>
		</div>

		{#if permError}
			<p class="text-gray-400 text-xs mb-3">{permError}</p>
		{/if}

		<div class="space-y-4">
			<!-- Microphone -->
			<div>
				<label class="block text-xs font-medium text-gray-300 mb-1" for="vc-mic">Microphone</label>
				<select id="vc-mic" bind:value={selectedMic}
					class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white text-sm outline-none focus:border-gray-500">
					<option value="">System default</option>
					{#each mics as d (d.deviceId)}
						<option value={d.deviceId}>{d.label}</option>
					{/each}
				</select>
			</div>

			<!-- Camera -->
			<div>
				<label class="block text-xs font-medium text-gray-300 mb-1" for="vc-cam">Camera</label>
				<select id="vc-cam" bind:value={selectedCamera}
					class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white text-sm outline-none focus:border-gray-500">
					<option value="">System default</option>
					{#each cameras as d (d.deviceId)}
						<option value={d.deviceId}>{d.label}</option>
					{/each}
				</select>
			</div>

			<!-- Speaker -->
			{#if speakers.length > 0}
			<div>
				<label class="block text-xs font-medium text-gray-300 mb-1" for="vc-spk">Speaker</label>
				<select id="vc-spk" bind:value={selectedSpeaker}
					class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white text-sm outline-none focus:border-gray-500">
					<option value="">System default</option>
					{#each speakers as d (d.deviceId)}
						<option value={d.deviceId}>{d.label}</option>
					{/each}
				</select>
			</div>
			{/if}
		</div>

		<div class="mt-5 flex justify-end gap-2">
			<button onclick={onClose}
				class="px-4 py-2 text-sm text-gray-400 hover:text-white transition-colors">Cancel</button>
			<button onclick={save}
				class="px-5 py-2 text-sm font-medium bg-gray-600 hover:bg-gray-700 text-white rounded-lg transition-colors">
				Save
			</button>
		</div>
	</div>
</div>
