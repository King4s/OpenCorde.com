<script lang="ts">
	/**
	 * @file Stage Panel — Shows active stage session with speakers and audience
	 * @purpose Display stage details, manage roles, raise hand
	 */
	import { currentUser } from '$lib/stores/auth';
	import {
		stageSession,
		stageParticipants,
		speakers,
		audience,
		handsRaised,
		startStage,
		endStage,
		joinStage,
		leaveStage,
		raiseHand,
		lowerHand,
		promoteSpeaker,
		demoteSpeaker,
	} from '$lib/stores/stage';

	interface Props {
		channelId: string;
		serverId: string;
		isOwner: boolean;
	}

	let { channelId, serverId, isOwner }: Props = $props();
	let showStartForm = $state(false);
	let topicInput = $state('');

	// Find current user in participants
	let currentUserParticipant = $derived.by(() => {
		return $stageParticipants.find((p) => p.user_id === $currentUser?.id);
	});

	async function handleStartStage() {
		try {
			await startStage(channelId, topicInput || undefined);
			showStartForm = false;
			topicInput = '';
		} catch (e) {
			console.error('Failed to start stage:', e);
		}
	}

	async function handleEndStage() {
		try {
			await endStage(channelId);
		} catch (e) {
			console.error('Failed to end stage:', e);
		}
	}

	async function handleJoinStage() {
		try {
			await joinStage(channelId);
		} catch (e) {
			console.error('Failed to join stage:', e);
		}
	}

	async function handleLeaveStage() {
		try {
			await leaveStage(channelId);
		} catch (e) {
			console.error('Failed to leave stage:', e);
		}
	}

	async function handleToggleHand() {
		try {
			if (currentUserParticipant?.hand_raised) {
				await lowerHand(channelId);
			} else {
				await raiseHand(channelId);
			}
		} catch (e) {
			console.error('Failed to toggle hand:', e);
		}
	}

	async function handlePromote(userId: string) {
		try {
			await promoteSpeaker(channelId, userId);
		} catch (e) {
			console.error('Failed to promote:', e);
		}
	}

	async function handleDemote(userId: string) {
		try {
			await demoteSpeaker(channelId, userId);
		} catch (e) {
			console.error('Failed to demote:', e);
		}
	}
</script>

<div class="bg-gray-750 border-t border-gray-700 p-3">
	<!-- Active Stage Session -->
	{#if $stageSession}
		<div class="mb-3">
			<div class="flex items-center justify-between mb-2">
				<h3 class="text-sm font-semibold text-white">
					🎙️ Stage {$stageSession.topic ? `: ${$stageSession.topic}` : ''}
				</h3>
				{#if $stageSession.started_by === $currentUser?.id}
					<button
						onclick={handleEndStage}
						class="px-2 py-1 bg-red-600 hover:bg-red-700 text-white text-xs rounded"
					>
						End Stage
					</button>
				{/if}
			</div>

			<!-- Speakers Section -->
			{#if $speakers.length > 0}
				<div class="mb-2 bg-gray-800 rounded p-2">
					<p class="text-xs font-semibold text-gray-400 mb-1">SPEAKERS ({$speakers.length})</p>
					<div class="space-y-1">
						{#each $speakers as speaker (speaker.id)}
							<div class="flex items-center justify-between text-xs bg-gray-700 rounded px-2 py-1">
								<div class="flex items-center gap-1 min-w-0">
									<span class="text-green-400">🎤</span>
									<span class="text-gray-200 truncate">{speaker.username}</span>
								</div>
								{#if $stageSession.started_by === $currentUser?.id && speaker.user_id !== $currentUser?.id}
									<button
										onclick={() => handleDemote(speaker.user_id)}
										class="ml-1 text-gray-500 hover:text-gray-300 text-xs"
										title="Demote to audience"
									>
										×
									</button>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Audience Section -->
			{#if $audience.length > 0}
				<div class="mb-2 bg-gray-800 rounded p-2">
					<p class="text-xs font-semibold text-gray-400 mb-1">AUDIENCE ({$audience.length})</p>
					<div class="space-y-1">
						{#each $audience as listener (listener.id)}
							<div
								class="flex items-center justify-between text-xs bg-gray-700 rounded px-2 py-1"
							>
								<div class="flex items-center gap-1 min-w-0">
									{#if listener.hand_raised}
										<span class="text-yellow-400">✋</span>
									{:else}
										<span class="text-gray-600">👤</span>
									{/if}
									<span class="text-gray-200 truncate">{listener.username}</span>
								</div>
								{#if $stageSession.started_by === $currentUser?.id}
									<button
										onclick={() => handlePromote(listener.user_id)}
										class="ml-1 text-gray-500 hover:text-gray-300 text-xs"
										title="Promote to speaker"
									>
										→
									</button>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<!-- User Controls -->
			{#if currentUserParticipant}
				<div class="flex gap-1">
					{#if currentUserParticipant.role === 'audience'}
						<button
							onclick={handleToggleHand}
							class="flex-1 px-2 py-1 text-xs rounded {currentUserParticipant.hand_raised
								? 'bg-yellow-600 hover:bg-yellow-700 text-white'
								: 'bg-indigo-600 hover:bg-indigo-700 text-white'}"
						>
							{currentUserParticipant.hand_raised ? 'Lower Hand' : 'Raise Hand'}
						</button>
					{/if}
					<button
						onclick={handleLeaveStage}
						class="flex-1 px-2 py-1 text-xs bg-gray-700 hover:bg-gray-600 text-gray-200 rounded"
					>
						Leave Stage
					</button>
				</div>
			{:else}
				<button
					onclick={handleJoinStage}
					class="w-full px-2 py-1 text-xs bg-indigo-600 hover:bg-indigo-700 text-white rounded"
				>
					Join Stage
				</button>
			{/if}
		</div>
	{:else if isOwner}
		<!-- Start Stage Button -->
		<div>
			{#if showStartForm}
				<div class="bg-gray-800 rounded p-2 mb-2">
					<input
						type="text"
						bind:value={topicInput}
						placeholder="Optional topic (e.g., Q&A Session)"
						class="w-full px-2 py-1 bg-gray-700 border border-gray-600 rounded text-white text-xs placeholder-gray-500 focus:outline-none focus:border-indigo-500 mb-1"
					/>
					<div class="flex gap-1">
						<button
							onclick={handleStartStage}
							class="flex-1 px-2 py-1 bg-indigo-600 hover:bg-indigo-700 text-white text-xs rounded"
						>
							Start
						</button>
						<button
							onclick={() => {
								showStartForm = false;
								topicInput = '';
							}}
							class="flex-1 px-2 py-1 bg-gray-700 hover:bg-gray-600 text-gray-200 text-xs rounded"
						>
							Cancel
						</button>
					</div>
				</div>
			{:else}
				<button
					onclick={() => (showStartForm = true)}
					class="w-full px-2 py-1 bg-indigo-600 hover:bg-indigo-700 text-white text-xs rounded"
				>
					🎙️ Start Stage
				</button>
			{/if}
		</div>
	{/if}
</div>
