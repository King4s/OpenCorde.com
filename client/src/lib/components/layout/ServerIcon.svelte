<script lang="ts">
	/**
	 * @file Space icon in sidebar
	 * @purpose Circular space icon with tooltip, unread indicator
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

<div class="relative group">
	<button
		class="w-12 h-12 rounded-{active ? 'xl' : '2xl'} {active
			? 'bg-gray-600 shadow-lg shadow-gray-600/25'
			: 'bg-gray-700 hover:bg-gray-600'} hover:rounded-xl transition-all duration-150 flex items-center justify-center text-white font-semibold text-sm"
		{onclick}
		title={name}
	>
		{initials}
	</button>
	{#if active}
		<span class="absolute -left-1.5 top-1/2 -translate-y-1/2 w-1.5 h-8 rounded-full bg-white"></span>
	{/if}
	{#if hasUnread && !active}
		<span class="absolute right-0 bottom-1 w-3 h-3 bg-gray-950 rounded-full flex items-center justify-center pointer-events-none">
			<span class="w-2 h-2 bg-gray-500 rounded-full"></span>
		</span>
	{/if}
</div>
