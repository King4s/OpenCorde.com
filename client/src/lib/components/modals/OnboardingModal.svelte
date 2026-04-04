<!--
  @component OnboardingModal
  @purpose Shown to new members when a server has onboarding enabled
  @version 1.0.0
  @uses api/client
-->
<script lang="ts">
	import { edgeResize } from '$lib/actions/edgeResize';

	interface Prompt {
		id: string;
		title: string;
		description?: string;
		required?: boolean;
		options: Array<{ id: string; label: string; description?: string }>;
	}

	interface Props {
		serverName: string;
		welcomeMessage: string | null;
		prompts: Prompt[];
		onDismiss: () => void;
	}

	let { serverName, welcomeMessage, prompts, onDismiss }: Props = $props();

	let step = $state(0);
	let selections = $state<Record<string, string[]>>({});

	const totalSteps = $derived(prompts.length + 1); // +1 for welcome

	function toggleOption(promptId: string, optionId: string) {
		const current = selections[promptId] ?? [];
		if (current.includes(optionId)) {
			selections = { ...selections, [promptId]: current.filter(id => id !== optionId) };
		} else {
			selections = { ...selections, [promptId]: [...current, optionId] };
		}
	}

	function canAdvance(): boolean {
		if (step === 0) return true;
		const prompt = prompts[step - 1];
		if (!prompt) return true;
		if (prompt.required) {
			return (selections[prompt.id] ?? []).length > 0;
		}
		return true;
	}

	function advance() {
		if (step < totalSteps - 1) {
			step++;
		} else {
			onDismiss();
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onDismiss();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- Backdrop -->
<div
	class="fixed inset-0 z-50 bg-black/60 flex items-center justify-center px-4"
	role="dialog"
	aria-modal="true"
	aria-label="Server onboarding"
>
	<div use:edgeResize={{ handles: ['left', 'right'], minWidth: 320, maxWidth: 1024 }} class="bg-gray-800 rounded-2xl shadow-2xl w-full max-w-2xl mx-4 overflow-auto onboarding-window"
		style="min-width: 20rem; min-height: 24rem; max-width: calc(100vw - 2rem); max-height: calc(100vh - 2rem);">
		<!-- Progress bar -->
		<div class="h-1 bg-gray-700">
			<div
				class="h-1 bg-gray-500 transition-all duration-300"
				style="width: {((step + 1) / totalSteps) * 100}%"
			></div>
		</div>

		<div class="p-6">
			{#if step === 0}
				<!-- Welcome screen -->
				<div class="text-center">
					<div class="text-4xl mb-4">👋</div>
					<h2 class="text-xl font-bold text-white mb-2">Welcome to {serverName}!</h2>
					{#if welcomeMessage}
						<p class="text-gray-300 text-sm leading-relaxed mb-6">{welcomeMessage}</p>
					{:else}
						<p class="text-gray-400 text-sm mb-6">We'll help you get set up in just a few steps.</p>
					{/if}
				</div>
			{:else}
				<!-- Prompt step -->
				{@const prompt = prompts[step - 1]}
				{#if prompt}
					<div>
						<h2 class="text-lg font-bold text-white mb-1">{prompt.title}</h2>
						{#if prompt.description}
							<p class="text-gray-400 text-sm mb-4">{prompt.description}</p>
						{:else}
							<div class="mb-4"></div>
						{/if}
						<div class="space-y-2 max-h-64 overflow-y-auto pr-1">
							{#each prompt.options as opt (opt.id)}
								{@const selected = (selections[prompt.id] ?? []).includes(opt.id)}
								<button
									onclick={() => toggleOption(prompt.id, opt.id)}
									class="w-full text-left px-4 py-3 rounded-lg border transition-colors
										{selected
											? 'bg-gray-600 border-gray-500 text-white'
											: 'bg-gray-700 border-gray-600 text-gray-300 hover:border-gray-500/50 hover:bg-gray-600'}"
								>
									<div class="font-medium text-sm">{opt.label}</div>
									{#if opt.description}
										<div class="text-xs mt-0.5 {selected ? 'text-gray-200' : 'text-gray-500'}">{opt.description}</div>
									{/if}
								</button>
							{/each}
						</div>
						{#if prompt.required}
							<p class="text-gray-500 text-xs mt-2">* Selection required</p>
						{/if}
					</div>
				{/if}
			{/if}

			<!-- Navigation -->
			<div class="flex justify-between items-center mt-6">
				<div class="text-xs text-gray-500">{step + 1} / {totalSteps}</div>
				<div class="flex gap-2">
					{#if step < totalSteps - 1}
						<button
							onclick={onDismiss}
							class="px-4 py-2 text-sm text-gray-400 hover:text-white transition-colors"
						>Skip</button>
					{/if}
					<button
						onclick={advance}
						disabled={!canAdvance()}
						class="px-5 py-2 text-sm font-medium bg-gray-600 hover:bg-gray-700 disabled:opacity-50 text-white rounded-lg transition-colors"
					>
						{step < totalSteps - 1 ? 'Next' : "Let's Go!"}
					</button>
				</div>
			</div>
		</div>
	</div>
</div>
