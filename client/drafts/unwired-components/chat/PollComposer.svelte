<script lang="ts">
	/**
	 * @file Poll composer modal for creating polls in messages
	 * @purpose Question input + add/remove answers + multiselect toggle + submit
	 */
	interface Props {
		onSubmit: (poll: { question: string; answers: { text: string }[]; allow_multiselect: boolean }) => void;
		onCancel: () => void;
	}

	let { onSubmit, onCancel }: Props = $props();

	let question = $state('');
	let answers = $state<{ id: number; text: string }[]>([
		{ id: 0, text: '' },
		{ id: 1, text: '' }
	]);
	let allowMultiselect = $state(false);
	let error = $state('');
	let nextId = $state(2);

	function addAnswer() {
		if (answers.length >= 10) return;
		answers = [...answers, { id: nextId, text: '' }];
		nextId++;
	}

	function removeAnswer(id: number) {
		if (answers.length <= 2) return;
		answers = answers.filter(a => a.id !== id);
	}

	function updateAnswer(id: number, text: string) {
		answers = answers.map(a => a.id === id ? { ...a, text } : a);
	}

	function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';

		if (!question.trim()) {
			error = 'Please enter a question';
			return;
		}
		if (question.length > 300) {
			error = 'Question must be 300 characters or less';
			return;
		}

		const validAnswers = answers.filter(a => a.text.trim());
		if (validAnswers.length < 2) {
			error = 'Please provide at least 2 answers';
			return;
		}

		for (const a of validAnswers) {
			if (a.text.length > 255) {
				error = 'Each answer must be 255 characters or less';
				return;
			}
		}

		onSubmit({
			question: question.trim(),
			answers: validAnswers.map(a => ({ text: a.text.trim() })),
			allow_multiselect: allowMultiselect
		});
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			onCancel();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60" onclick={onCancel} role="dialog" aria-label="Create poll">
	<!-- Prevent close when clicking the modal itself -->
	<div class="bg-gray-800 border border-gray-600 rounded-lg shadow-2xl w-full max-w-md mx-4 p-5" onclick={(e) => e.stopPropagation()}>
		<div class="flex items-center justify-between mb-4">
			<h2 class="text-white font-semibold text-lg">Create Poll</h2>
			<button
				class="text-gray-400 hover:text-gray-200 text-xl leading-none"
				onclick={onCancel}
				aria-label="Close"
			>✕</button>
		</div>

		<form onsubmit={handleSubmit}>
			<!-- Question -->
			<div class="mb-4">
				<label class="block text-gray-400 text-xs font-medium mb-1 uppercase tracking-wider">Question</label>
				<input
					type="text"
					bind:value={question}
					placeholder="What's your poll about?"
					maxlength={300}
					class="w-full bg-gray-700 text-gray-200 rounded px-3 py-2 text-sm border border-gray-600 focus:border-gray-400 focus:outline-none placeholder-gray-500"
				/>
				<span class="text-gray-500 text-xs">{question.length}/300</span>
			</div>

			<!-- Answers -->
			<div class="mb-4">
				<label class="block text-gray-400 text-xs font-medium mb-2 uppercase tracking-wider">Answers</label>
				<div class="flex flex-col gap-2">
					{#each answers as answer (answer.id)}
						<div class="flex items-center gap-2">
							<input
								type="text"
								value={answer.text}
								oninput={(e) => updateAnswer(answer.id, (e.target as HTMLInputElement).value)}
								placeholder="Answer {answer.id + 1}"
								maxlength={255}
								class="flex-1 bg-gray-700 text-gray-200 rounded px-3 py-2 text-sm border border-gray-600 focus:border-gray-400 focus:outline-none placeholder-gray-500"
							/>
							{#if answers.length > 2}
								<button
									type="button"
									onclick={() => removeAnswer(answer.id)}
									class="text-gray-500 hover:text-red-400 text-lg leading-none px-1"
									aria-label="Remove answer"
								>✕</button>
							{/if}
						</div>
					{/each}
				</div>
				{#if answers.length < 10}
					<button
						type="button"
						onclick={addAnswer}
						class="mt-2 text-gray-400 hover:text-gray-200 text-sm flex items-center gap-1"
					>
						<span>+ Add answer</span>
					</button>
				{/if}
			</div>

			<!-- Options -->
			<div class="mb-4 flex items-center gap-2">
				<input
					type="checkbox"
					id="multiselect"
					bind:checked={allowMultiselect}
					class="rounded bg-gray-700 border-gray-600"
				/>
				<label for="multiselect" class="text-gray-300 text-sm">Allow multiple answers</label>
			</div>

			{#if error}
				<p class="text-red-400 text-xs mb-3">{error}</p>
			{/if}

			<!-- Actions -->
			<div class="flex justify-between items-center">
				<button
					type="button"
					onclick={onCancel}
					class="text-gray-400 hover:text-gray-200 text-sm"
				>Cancel</button>
				<button
					type="submit"
					class="bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium px-4 py-2 rounded transition-colors"
				>Create Poll</button>
			</div>
		</form>
	</div>
</div>
