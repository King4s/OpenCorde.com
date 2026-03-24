<script lang="ts">
	/**
	 * @file Channel settings modal
	 * @purpose Edit channel name, topic, and NSFW flag
	 */
	import api from '$lib/api/client';

	interface Props {
		channelId: string;
		serverId: string;
		channelName: string;
		channelTopic: string | null;
		channelNsfw: boolean;
		onClose: () => void;
		onSave: (updated: { name: string; topic: string | null; nsfw: boolean }) => void;
	}

	let { channelId, serverId, channelName, channelTopic, channelNsfw, onClose, onSave }: Props = $props();

	let name = $state('');
	let topic = $state('');
	let nsfw = $state(false);
	let saving = $state(false);
	let error = $state('');

	// Initialize form state from props
	$effect(() => {
		name = channelName;
		topic = channelTopic ?? '';
		nsfw = channelNsfw;
	});

	async function handleSave() {
		saving = true;
		error = '';
		try {
			const body: Record<string, any> = {
				name: name.trim(),
				topic: topic.trim() || null,
				nsfw
			};

			await api.patch(`/channels/${channelId}`, body);
			onSave({ name: name.trim(), topic: topic.trim() || null, nsfw });
			onClose();
		} catch (e: any) {
			error = e.message ?? 'Failed to save channel settings';
		} finally {
			saving = false;
		}
	}

	function handleCancel() {
		onClose();
	}
</script>

<!-- Modal backdrop -->
<div
	class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
	role="presentation"
	onclick={handleCancel}
	onkeydown={(e) => e.key === 'Escape' && handleCancel()}
>
	<!-- Modal content (stop propagation) -->
	<div
		class="bg-gray-800 rounded-lg shadow-xl w-96 max-w-[90vw]"
		role="dialog"
		aria-modal="true"
		aria-labelledby="modal-title"
		tabindex={-1}
		onclick={(e) => e.stopPropagation()}
		onkeydown={(e) => e.stopPropagation()}
	>
		<div class="px-6 py-4 border-b border-gray-700">
			<h2 id="modal-title" class="text-lg font-semibold text-white">Channel Settings</h2>
		</div>

		<div class="px-6 py-4 space-y-4">
			{#if error}
				<div class="px-3 py-2 bg-red-900/40 border border-red-700/50 rounded text-red-300 text-sm">
					{error}
				</div>
			{/if}

			<!-- Name input -->
			<div>
				<label class="block text-xs text-gray-400 mb-1.5" for="modal-channel-name">Channel Name</label>
				<input
					id="modal-channel-name"
					type="text"
					bind:value={name}
					maxlength="100"
					placeholder="Channel name"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-indigo-500"
				/>
			</div>

			<!-- Topic input -->
			<div>
				<label class="block text-xs text-gray-400 mb-1.5" for="modal-channel-topic">Topic (Optional)</label>
				<input
					id="modal-channel-topic"
					type="text"
					bind:value={topic}
					maxlength="512"
					placeholder="Channel topic"
					class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-indigo-500"
				/>
			</div>

			<!-- NSFW checkbox -->
			<div class="flex items-center gap-2">
				<input
					id="modal-channel-nsfw"
					type="checkbox"
					bind:checked={nsfw}
					class="w-4 h-4 rounded border border-gray-600 bg-gray-900 cursor-pointer"
				/>
				<label for="modal-channel-nsfw" class="text-sm text-gray-300 cursor-pointer">Mark as NSFW</label>
			</div>
		</div>

		<div class="px-6 py-4 border-t border-gray-700 flex gap-2 justify-end">
			<button
				onclick={handleCancel}
				class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-gray-300 text-sm font-medium rounded transition-colors"
			>
				Cancel
			</button>
			<button
				onclick={handleSave}
				disabled={saving}
				class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white text-sm font-medium rounded transition-colors"
			>
				{saving ? 'Saving...' : 'Save'}
			</button>
		</div>
	</div>
</div>
