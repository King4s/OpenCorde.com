<script lang="ts">
	/**
	 * @file Delete confirmation modal component
	 * @purpose Confirm destructive admin operations
	 */

	interface Props {
		isOpen: boolean;
		type: 'user' | 'server';
		name: string;
		onCancel: () => void;
		onConfirm: () => void;
	}

	const { isOpen, type, name, onCancel, onConfirm }: Props = $props();
</script>

{#if isOpen}
	<div class="fixed inset-0 bg-black/80 flex items-center justify-center z-50">
		<div class="bg-gray-800 rounded-lg p-6 max-w-sm mx-4">
			<h3 class="text-lg font-semibold text-white mb-2">
				Delete {type === 'user' ? 'User' : 'Server'}?
			</h3>
			<p class="text-gray-400 text-sm mb-4">
				Permanently delete <strong>{name}</strong>? This cannot be undone.
			</p>
			<div class="flex gap-2 justify-end">
				<button
					onclick={onCancel}
					class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white text-sm rounded transition-colors"
				>
					Cancel
				</button>
				<button
					onclick={onConfirm}
					class="px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white text-sm rounded transition-colors"
				>
					Delete
				</button>
			</div>
		</div>
	</div>
{/if}
