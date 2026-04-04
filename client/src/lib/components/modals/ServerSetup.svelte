<!--
  @component ServerSetup
  @purpose First-launch modal in Tauri desktop app — lets user enter their OpenCorde server URL
  @shows When window.__TAURI_INTERNALS__ is defined and opencorde_server is not set in localStorage
-->
<script lang="ts">
	import { setServerUrl } from '$lib/api/client';

	let url = $state('');
	let error = $state('');
	let testing = $state(false);

	/** Normalize whatever the user typed into a full URL with protocol. */
	function normalizeUrl(input: string): string {
		let s = input.trim().replace(/\/$/, '');
		if (!s) return '';
		// Already has protocol
		if (/^https?:\/\//i.test(s)) return s;
		// Looks like a plain hostname or host:port — default to https, fall back to http on failure
		return 'https://' + s;
	}

	async function connect() {
		error = '';
		if (!url.trim()) {
			error = 'Enter your instance address';
			return;
		}

		const base = normalizeUrl(url);
		testing = true;

		// Try https first, then http for local servers
		const candidates = base.startsWith('https://')
			? [base, base.replace('https://', 'http://')]
			: [base];

		let reachable = '';
		for (const candidate of candidates) {
			try {
				const res = await fetch(`${candidate}/api/v1/health`, { signal: AbortSignal.timeout(6000) });
				if (res.ok || res.status < 500) { reachable = candidate; break; }
			} catch {
				// try next
			}
		}

		if (!reachable) {
			error = `Could not reach instance. Check the address and make sure OpenCorde is running.`;
			testing = false;
			return;
		}

		setServerUrl(reachable);
		window.location.reload();
	}
</script>

<div class="fixed inset-0 bg-black/80 flex items-center justify-center z-50">
	<div class="bg-gray-900 border border-gray-700 rounded-xl p-8 w-full max-w-md shadow-2xl">
		<h1 class="text-2xl font-bold text-white mb-2">Connect to OpenCorde</h1>
		<p class="text-gray-400 text-sm mb-6">
			Enter the address of your OpenCorde instance. You only need to do this once.
		</p>

		<label for="instance-address" class="block text-sm font-medium text-gray-300 mb-1">Instance address</label>
		<input
			id="instance-address"
			type="text"
			bind:value={url}
			placeholder="opencorde.com  or  192.168.1.10:8080"
			class="w-full bg-gray-800 border border-gray-600 rounded-lg px-4 py-2.5 text-white placeholder-gray-500 focus:outline-none focus:border-gray-500 mb-1"
			onkeydown={(e) => e.key === 'Enter' && connect()}
		/>
		{#if error}
			<p class="text-gray-400 text-sm mb-3">{error}</p>
		{:else}
			<p class="text-gray-600 text-xs mb-3">No need for http:// — just type the address</p>
		{/if}

		<button
			onclick={connect}
			disabled={testing}
			class="w-full bg-gray-600 hover:bg-gray-500 disabled:opacity-50 text-white font-semibold py-2.5 rounded-lg transition-colors"
		>
			{testing ? 'Connecting…' : 'Connect'}
		</button>
	</div>
</div>
