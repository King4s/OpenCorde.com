<script lang="ts">
	/**
	 * @file Server icon in sidebar
	 * @purpose Circular server icon with tooltip, unread indicator
	 * @version 1.1.0
	 */
	interface Props {
		name: string;
		active?: boolean;
		hasUnread?: boolean;
		onclick?: () => void;
	}

	let { name, active = false, hasUnread = false, onclick }: Props = $props();

	let initials = $derived(
		name
			.split(' ')
			.map((w) => w[0])
			.join('')
			.slice(0, 2)
			.toUpperCase()
	);
</script>

<div class="relative">
	<button
		class="w-12 h-12 rounded-{active ? 'xl' : '2xl'} {active
			? 'bg-indigo-600'
			: 'bg-gray-700 hover:bg-indigo-600'} hover:rounded-xl transition-all flex items-center justify-center text-white font-semibold text-sm"
		{onclick}
		title={name}
	>
		{initials}
	</button>
	{#if hasUnread && !active}
		<span class="absolute right-0 bottom-1 w-3 h-3 bg-gray-950 rounded-full flex items-center justify-center pointer-events-none">
			<span class="w-2 h-2 bg-red-500 rounded-full"></span>
		</span>
	{/if}
</div>
