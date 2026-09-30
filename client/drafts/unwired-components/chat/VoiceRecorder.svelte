<script lang="ts">
	/**
	 * @file Voice message recorder
	 * @purpose Microphone recording UI for voice messages with MediaRecorder API
	 * @version 1.0.0
	 *
	 * States: idle → requesting → recording → (complete | cancelled | error)
	 * Emits the recorded audio blob + generated filename on completion.
	 */
	interface Props {
		onComplete: (blob: Blob, filename: string) => void;
		onCancel?: () => void;
	}

	let { onComplete, onCancel }: Props = $props();

	let state = $state<'idle' | 'requesting' | 'recording' | 'error'>('idle');
	let errorMessage = $state('');
	let durationSeconds = $state(0);
	let mediaRecorder: MediaRecorder | null = null;
	let stream: MediaStream | null = null;
	let chunks: Blob[] = [];
	let durationInterval: ReturnType<typeof setInterval> | null = null;

	function formatDuration(seconds: number): string {
		const mins = Math.floor(seconds / 60);
		const secs = seconds % 60;
		return `${mins}:${secs.toString().padStart(2, '0')}`;
	}

	function cleanupStream() {
		if (durationInterval) {
			clearInterval(durationInterval);
			durationInterval = null;
		}
		if (stream) {
			stream.getTracks().forEach((track) => track.stop());
			stream = null;
		}
		mediaRecorder = null;
		chunks = [];
	}

	async function startRecording() {
		state = 'requesting';
		errorMessage = '';

		try {
			stream = await navigator.mediaDevices.getUserMedia({ audio: true });

			const mimeType = MediaRecorder.isTypeSupported('audio/webm;codecs=opus')
				? 'audio/webm;codecs=opus'
				: MediaRecorder.isTypeSupported('audio/webm')
					? 'audio/webm'
					: 'audio/mp4';

			mediaRecorder = new MediaRecorder(stream, { mimeType });

			mediaRecorder.ondataavailable = (e: BlobEvent) => {
				if (e.data.size > 0) {
					chunks.push(e.data);
				}
			};

			mediaRecorder.onstop = () => {
				const extension = mimeType.includes('webm') ? 'webm' : 'mp4';
				const actualMimeType = mimeType.includes('webm') ? 'audio/webm' : 'audio/mp4';
				const blob = new Blob(chunks, { type: actualMimeType });
				const filename = `voice-message-${Date.now()}.${extension}`;
				cleanupStream();
				onComplete(blob, filename);
			};

			mediaRecorder.start(250); // collect chunks every 250ms
			state = 'recording';
			durationSeconds = 0;

			durationInterval = setInterval(() => {
				durationSeconds++;
			}, 1000);
		} catch (err: any) {
			state = 'error';
			if (err.name === 'NotAllowedError' || err.name === 'PermissionDeniedError') {
				errorMessage = 'Microphone access denied. Please allow mic permissions.';
			} else if (err.name === 'NotFoundError') {
				errorMessage = 'No microphone found on this device.';
			} else {
				errorMessage = `Could not access microphone: ${err.message ?? 'unknown error'}`;
			}
			cleanupStream();
		}
	}

	function stopRecording() {
		if (mediaRecorder && mediaRecorder.state === 'recording') {
			mediaRecorder.stop();
		}
	}

	function cancelRecording() {
		if (mediaRecorder && mediaRecorder.state === 'recording') {
			// Discard chunks before stopping
			chunks = [];
			mediaRecorder.stop();
		}
		cleanupStream();
		state = 'idle';
		onCancel?.();
	}

	function dismissError() {
		state = 'idle';
		errorMessage = '';
	}
</script>

<div class="flex items-center gap-2">
	{#if state === 'idle'}
		<button
			type="button"
			onclick={startRecording}
			class="h-8 w-8 flex-shrink-0 rounded hover:bg-gray-600/50 flex items-center justify-center text-gray-400 hover:text-gray-200 transition-colors"
			title="Record voice message"
			aria-label="Record voice message"
		>
			🎤
		</button>

	{:else if state === 'requesting'}
		<div class="flex items-center gap-2 text-xs text-gray-400">
			<span class="animate-pulse">🎤</span>
			<span>Requesting microphone...</span>
		</div>

	{:else if state === 'recording'}
		<div class="flex items-center gap-2 bg-red-500/10 border border-red-500/30 rounded-lg px-3 py-1.5 min-w-0">
			<!-- Recording indicator -->
			<span class="h-2.5 w-2.5 rounded-full bg-red-500 animate-pulse flex-shrink-0"></span>

			<!-- Duration -->
			<span class="text-sm text-red-400 font-mono tabular-nums min-w-[3ch]">
				{formatDuration(durationSeconds)}
			</span>

			<!-- Waveform-like bars (purely decorative, animated) -->
			<div class="flex items-end gap-px h-6 flex-1 min-w-0">
				{#each Array(12) as _, i}
					{@const height = 4 + Math.abs(Math.sin((Date.now() / 200) + i * 0.7)) * 18}
					<div
						class="w-1 bg-red-400/60 rounded-full transition-all duration-75"
						style="height: {height}px"
					></div>
				{/each}
			</div>

			<!-- Stop button -->
			<button
				type="button"
				onclick={stopRecording}
				class="h-7 w-7 flex-shrink-0 rounded-full bg-red-500 hover:bg-red-600 flex items-center justify-center text-white transition-colors"
				title="Stop recording"
				aria-label="Stop recording"
			>
				<span class="text-xs">⏹</span>
			</button>

			<!-- Cancel button -->
			<button
				type="button"
				onclick={cancelRecording}
				class="h-7 w-7 flex-shrink-0 rounded hover:bg-gray-600/50 flex items-center justify-center text-gray-400 hover:text-gray-200 transition-colors"
				title="Cancel recording"
				aria-label="Cancel recording"
			>
				✕
			</button>
		</div>

	{:else if state === 'error'}
		<div class="flex items-center gap-2 text-xs text-red-400 bg-red-500/10 border border-red-500/30 rounded-lg px-2 py-1">
			<span>⚠ {errorMessage}</span>
			<button
				type="button"
				onclick={dismissError}
				class="text-red-300 hover:text-red-100 ml-1"
				aria-label="Dismiss error"
			>
				✕
			</button>
		</div>
	{/if}
</div>
