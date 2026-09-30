<script lang="ts">
	/**
	 * @file Waveform audio player for voice messages
	 * @purpose Renders a waveform visualization from audio data with play/pause and progress
	 * @version 1.0.0
	 *
	 * Fetches audio via URL, decodes it with Web Audio API, extracts peaks,
	 * and renders them on a canvas with a playback progress overlay.
	 */
	import { onMount } from 'svelte';

	interface Props {
		audioUrl: string;
		filename?: string;
		size?: number;
	}

	let { audioUrl, filename, size }: Props = $props();

	let canvasEl = $state<HTMLCanvasElement | null>(null);
	let audioCtx = $state<AudioContext | null>(null);
	let audioBuffer = $state<AudioBuffer | null>(null);
	let sourceNode = $state<AudioBufferSourceNode | null>(null);

	let isPlaying = $state(false);
	let currentTime = $state(0);
	let duration = $state(0);
	let peaks = $state<number[]>([]);
	let isLoading = $state(true);
	let loadError = $state(false);
	let animFrameId = $state<number | null>(null);

	const BAR_WIDTH = 2;
	const BAR_GAP = 1;

	function formatTime(seconds: number): string {
		const mins = Math.floor(seconds / 60);
		const secs = Math.floor(seconds % 60);
		return `${mins}:${secs.toString().padStart(2, '0')}`;
	}

	function formatFileSize(bytes: number | undefined): string {
		if (!bytes) return '';
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	function extractPeaks(buffer: AudioBuffer, numBars: number): number[] {
		const channelData = buffer.getChannelData(0);
		const samplesPerBar = Math.floor(channelData.length / numBars);
		const result: number[] = [];

		for (let i = 0; i < numBars; i++) {
			let max = 0;
			const start = i * samplesPerBar;
			const end = start + samplesPerBar;
			for (let j = start; j < end; j++) {
				const abs = Math.abs(channelData[j]);
				if (abs > max) max = abs;
			}
			result.push(max);
		}
		return result;
	}

	function drawWaveform(progress: number = 0) {
		const canvas = canvasEl;
		if (!canvas || peaks.length === 0) return;

		const dpr = window.devicePixelRatio || 1;
		const displayWidth = canvas.clientWidth;
		const displayHeight = canvas.clientHeight;
		canvas.width = displayWidth * dpr;
		canvas.height = displayHeight * dpr;

		const ctx = canvas.getContext('2d');
		if (!ctx) return;

		ctx.scale(dpr, dpr);

		// Clear
		ctx.clearRect(0, 0, displayWidth, displayHeight);

		const barTotalWidth = BAR_WIDTH + BAR_GAP;
		const totalBars = Math.min(peaks.length, Math.floor(displayWidth / barTotalWidth));
		const centerY = displayHeight / 2;

		for (let i = 0; i < totalBars; i++) {
			const peak = peaks[i];
			const barHeight = Math.max(2, peak * displayHeight * 0.85);
			const x = i * barTotalWidth;

			// Determine if this bar is before or at the progress point
			const barProgress = i / totalBars;
			const isPlayed = barProgress <= progress;

			// Draw bar (top half + bottom half symmetrical)
			const y = centerY - barHeight / 2;

			ctx.fillStyle = isPlayed ? '#ef4444' : '#6b7280';
			ctx.fillRect(x, y, BAR_WIDTH, barHeight);
		}
	}

	function updatePlayhead() {
		if (!audioCtx || !sourceNode) return;

		// AudioBufferSourceNode doesn't expose currentTime, track via animation
		const elapsed = audioCtx.currentTime - startTime;
		if (elapsed >= duration) {
			stopPlayback();
			return;
		}
		currentTime = elapsed;
		drawWaveform(elapsed / duration);
		animFrameId = requestAnimationFrame(updatePlayhead);
	}

	let startTime = 0;

	function startPlayback() {
		if (!audioBuffer || !audioCtx) return;

		// Resume context if suspended (autoplay policy)
		if (audioCtx.state === 'suspended') {
			audioCtx.resume();
		}

		// Create new source node (can only be used once)
		sourceNode = audioCtx.createBufferSource();
		sourceNode.buffer = audioBuffer;
		sourceNode.connect(audioCtx.destination);

		sourceNode.onended = () => {
			stopPlayback();
		};

		startTime = audioCtx.currentTime;
		sourceNode.start(0);
		isPlaying = true;

		animFrameId = requestAnimationFrame(updatePlayhead);
	}

	function stopPlayback(reset: boolean = true) {
		if (animFrameId) {
			cancelAnimationFrame(animFrameId);
			animFrameId = null;
		}
		if (sourceNode) {
			try {
				sourceNode.stop();
			} catch {
				// Already stopped, ignore
			}
			sourceNode = null;
		}
		isPlaying = false;
		if (reset) {
			currentTime = 0;
			drawWaveform(0);
		}
	}

	function togglePlay() {
		if (isPlaying) {
			stopPlayback(false);
		} else {
			startPlayback();
		}
	}

	async function loadAudio() {
		isLoading = true;
		loadError = false;

		try {
			const response = await fetch(audioUrl);
			if (!response.ok) throw new Error(`HTTP ${response.status}`);

			const arrayBuffer = await response.arrayBuffer();

			audioCtx = new AudioContext();
			audioBuffer = await audioCtx.decodeAudioData(arrayBuffer);
			duration = audioBuffer.duration;

			// Extract peaks for waveform display
			const canvasWidth = canvasEl?.clientWidth ?? 300;
			const barTotalWidth = BAR_WIDTH + BAR_GAP;
			const maxBars = Math.floor(canvasWidth / barTotalWidth);
			peaks = extractPeaks(audioBuffer, Math.max(20, maxBars));

			isLoading = false;

			// Draw initial waveform
			await tick();
			drawWaveform(0);
		} catch {
			loadError = true;
			isLoading = false;
		}
	}

	function handleCanvasClick(e: MouseEvent) {
		if (!canvasEl || duration === 0) return;

		const rect = canvasEl.getBoundingClientRect();
		const x = e.clientX - rect.left;
		const fraction = x / rect.width;
		const seekTime = fraction * duration;

		// Stop current playback
		stopPlayback(true);

		// Seek by creating a new source that starts at seekTime
		if (audioBuffer && audioCtx) {
			if (audioCtx.state === 'suspended') {
				audioCtx.resume();
			}

			sourceNode = audioCtx.createBufferSource();
			sourceNode.buffer = audioBuffer;
			sourceNode.connect(audioCtx.destination);

			sourceNode.onended = () => {
				stopPlayback();
			};

			startTime = audioCtx.currentTime - seekTime;
			currentTime = seekTime;
			sourceNode.start(0, seekTime);
			isPlaying = true;

			drawWaveform(fraction);
			animFrameId = requestAnimationFrame(updatePlayhead);
		}
	}

	import { tick } from 'svelte';

	onMount(() => {
		loadAudio();

		return () => {
			stopPlayback(true);
			if (audioCtx) {
				audioCtx.close();
				audioCtx = null;
			}
		};
	});
</script>

<div class="flex items-center gap-2 bg-gray-700/60 rounded-lg px-2.5 py-2 min-w-0 max-w-full sm:max-w-sm select-none">
	<!-- Play/Pause button -->
	<button
		type="button"
		onclick={togglePlay}
		disabled={isLoading || loadError}
		class="h-8 w-8 flex-shrink-0 rounded-full bg-gray-600 hover:bg-gray-500 disabled:opacity-40 flex items-center justify-center text-white transition-colors"
		aria-label={isPlaying ? 'Pause' : 'Play'}
	>
		{#if isLoading}
			<span class="text-xs animate-pulse">...</span>
		{:else if loadError}
			<span class="text-xs">⚠</span>
		{:else if isPlaying}
			<span class="text-xs ml-px">⏸</span>
		{:else}
			<span class="text-xs ml-0.5">▶</span>
		{/if}
	</button>

	<!-- Waveform canvas -->
	<div class="flex-1 min-w-0 h-10 relative cursor-pointer" role="button" tabindex="0" aria-label="Seek audio">
		<canvas
			bind:this={canvasEl}
			onclick={handleCanvasClick}
			class="w-full h-full rounded"
		></canvas>

		{#if isLoading}
			<div class="absolute inset-0 flex items-center justify-center">
				<span class="text-xs text-gray-500 animate-pulse">Loading audio...</span>
			</div>
		{:else if loadError}
			<div class="absolute inset-0 flex items-center justify-center">
				<span class="text-xs text-red-400">Failed to load audio</span>
			</div>
		{/if}
	</div>

	<!-- Time display -->
	<span class="text-xs text-gray-400 font-mono tabular-nums flex-shrink-0 min-w-[7ch] text-right">
		{formatTime(isPlaying ? currentTime : (duration > 0 ? duration : 0))}
	</span>
</div>

{#if filename || size}
	<div class="text-xs text-gray-500 px-1 mt-0.5">
		{#if filename}
			<span>{filename}</span>
		{/if}
		{#if size}
			<span> · {formatFileSize(size)}</span>
		{/if}
	</div>
{/if}
