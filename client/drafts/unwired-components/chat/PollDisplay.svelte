<script lang="ts">
	/**
	 * @file Poll display component for message list
	 * @purpose Renders poll question + answers with vote buttons and progress bars
	 */
	import type { Poll, PollAnswer } from '$lib/api/types';
	import api from '$lib/api/client';

	interface Props {
		poll: Poll;
		messageId: string;
		currentUserId: string;
		onVote?: (messageId: string) => void;
	}

	let { poll, messageId, currentUserId, onVote }: Props = $props();

	let localPoll = $state<Poll>(structuredClone(poll));
	let voting = $state(false);
	let voteError = $state('');

	// Total votes across all answers
	let totalVotes = $derived(
		localPoll.answers.reduce((sum, a) => sum + a.votes.length, 0)
	);

	// Has the current user voted?
	let userVoted = $derived(
		localPoll.answers.some(a => a.votes.includes(Number(currentUserId)))
	);

	// Which answer(s) the user voted for
	let userVoteIds = $derived(
		localPoll.answers
			.filter(a => a.votes.includes(Number(currentUserId)))
			.map(a => a.id)
	);

	async function handleVote(answerId: number) {
		if (voting) return;
		if (localPoll.closed) return;

		// If already voted for this answer in single-select mode, do nothing
		if (!localPoll.allow_multiselect && userVoteIds.includes(answerId)) return;

		voting = true;
		voteError = '';

		try {
			const updated = await api.post<Poll>(
				`/messages/${messageId}/vote`,
				{ answer_id: answerId }
			);
			localPoll = updated;
			onVote?.(messageId);
		} catch (err: any) {
			voteError = err.message || 'Failed to vote';
		} finally {
			voting = false;
		}
	}

	function getPercentage(answer: PollAnswer): number {
		if (totalVotes === 0) return 0;
		return Math.round((answer.votes.length / totalVotes) * 100);
	}

	function isUserAnswer(answerId: number): boolean {
		return userVoteIds.includes(answerId);
	}
</script>

<div class="poll-container bg-gray-700/30 border border-gray-600/30 rounded-lg p-4 mt-2 max-w-md">
	<!-- Question -->
	<h3 class="text-white font-semibold text-sm mb-3">{localPoll.question}</h3>

	<!-- Answers -->
	<div class="flex flex-col gap-2">
		{#each localPoll.answers as answer (answer.id)}
			{@const pct = getPercentage(answer)}
			{@const active = isUserAnswer(answer.id)}
			<button
				class="relative w-full text-left rounded border transition-colors overflow-hidden
					{localPoll.closed ? 'cursor-default' : 'cursor-pointer hover:border-gray-400/50'}
					{active ? 'border-blue-500/50 bg-blue-500/10' : 'border-gray-600/30 bg-gray-700/20'}"
				onclick={() => handleVote(answer.id)}
				disabled={localPoll.closed}
				aria-label="Vote for {answer.text}"
			>
				<!-- Progress bar -->
				{#if totalVotes > 0}
					<div
						class="absolute inset-y-0 left-0 {active ? 'bg-blue-500/20' : 'bg-gray-500/15'} transition-all duration-300"
						style="width: {pct}%"
					></div>
				{/if}
				<!-- Content -->
				<div class="relative flex items-center justify-between px-3 py-2.5 z-10">
					<span class="text-gray-200 text-sm">{answer.text}</span>
					<span class="text-gray-400 text-xs ml-2 whitespace-nowrap">
						{#if totalVotes > 0}
							{pct}% ({answer.votes.length})
						{:else}
							0
						{/if}
					</span>
				</div>
			</button>
		{/each}
	</div>

	<!-- Footer -->
	<div class="flex items-center justify-between mt-3 text-xs text-gray-500">
		<span>
			{totalVotes} vote{totalVotes !== 1 ? 's' : ''}
			{#if localPoll.allow_multiselect}
				· multiple choice
			{/if}
		</span>
		{#if localPoll.closed}
			<span class="text-red-400 font-medium">Closed</span>
		{:else if userVoted && !localPoll.allow_multiselect}
			<span class="text-gray-400">Voted</span>
		{/if}
	</div>

	{#if voteError}
		<p class="text-red-400 text-xs mt-1">{voteError}</p>
	{/if}

	{#if voting}
		<p class="text-gray-400 text-xs mt-1">Voting...</p>
	{/if}
</div>
