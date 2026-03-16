<script lang="ts">
	/**
	 * @file Server icon in sidebar
	 * @purpose Circular server icon with tooltip
	 * @version 1.0.0
	 */
	interface Props {
		name: string;
		active?: boolean;
		onclick?: () => void;
	}

	let { name, active = false, onclick }: Props = $props();

	let initials = $derived(
		name
			.split(' ')
			.map((w) => w[0])
			.join('')
			.slice(0, 2)
			.toUpperCase()
	);
</script>

<button
	class="w-12 h-12 rounded-{active ? 'xl' : '2xl'} {active
		? 'bg-indigo-600'
		: 'bg-gray-700 hover:bg-indigo-600'} hover:rounded-xl transition-all flex items-center justify-center text-white font-semibold text-sm"
	{onclick}
	title={name}
>
	{initials}
</button>
