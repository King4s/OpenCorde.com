<script lang="ts">
	/**
	 * @file Landing page
	 * @purpose Marketing page — project overview, self-hosting guide, GitHub link
	 */
	import { browser } from '$app/environment';

	const GITHUB_URL = 'https://github.com/opencorde/opencorde';

	if (browser && window.location.pathname === '/' && localStorage.getItem('opencorde_token')) {
		window.location.replace('/servers');
	}

	const features = [
		{
			icon: '🔐',
			label: 'End-to-End Encryption',
			title: 'End-to-End Encryption',
			desc: 'MLS protocol (RFC 9420) for text channels, voice, video, and file transfers. Keys never leave your device.'
		},
		{
			icon: '🎙️',
			label: 'Voice & Video',
			title: 'Voice & Video',
			desc: 'LiveKit-powered SFU with screen sharing, noise suppression, per-participant controls, and stage channels.'
		},
		{
			icon: '🌉',
			label: 'Discord Bridge',
			title: 'Discord Bridge',
			desc: 'Bidirectional message bridging with Discord via ghost users. Migrate communities without losing history.'
		},
		{
			icon: '🗂️',
			label: 'Forums & Events',
			title: 'Forums & Events',
			desc: 'Threaded forum channels with posts and replies. Scheduled events with RSVP and calendar integration.'
		},
		{
			icon: '🔍',
			label: 'Full-Text Search',
			title: 'Full-Text Search',
			desc: 'Tantivy-powered search across all messages, channels, and attachments — instant, server-side.'
		},
		{
			icon: '🕸️',
			label: 'Federation-Ready',
			title: 'Federation-Ready',
			desc: 'Built around Ed25519 node identity and a peer registry. Cross-instance messaging is on the roadmap — the architecture is designed for it from day one.'
		},
		{
			icon: '🛡️',
			label: 'Your Infrastructure',
			title: 'Your Infrastructure',
			desc: 'Self-host on any Linux server. PostgreSQL, Redis, MinIO, LiveKit — all standard open-source components.'
		}
	];

	const steps = [
		{
			n: '1',
			title: 'Clone & configure',
			code: `git clone https://github.com/opencorde/opencorde\ncp .env.example .env\n# Edit .env — set JWT_SECRET, database credentials, SMTP`
		},
		{
			n: '2',
			title: 'Start services',
			code: `docker compose up -d\n# Starts PostgreSQL, Redis, MinIO, LiveKit`
		},
		{
			n: '3',
			title: 'Run the API',
			code: `cargo run -p opencorde-api\n# Runs migrations automatically on first boot`
		},
		{
			n: '4',
			title: 'Launch the client',
			code: `cd client && npm install && npm run dev\n# Or build the Tauri desktop app:\nnpm run tauri build`
		}
	];
</script>

<svelte:head>
	<title>OpenCorde — Self-hosted Discord alternative with E2EE</title>
	<meta name="description" content="OpenCorde is a feature-complete, self-hosted Discord alternative. Servers, channels, voice, video, threads, forums, end-to-end encryption, and a Discord bridge — on infrastructure you own." />
	<meta name="keywords" content="self-hosted chat, discord alternative, open source, E2EE, voice chat, team communication, AGPL" />
	<link rel="canonical" href="https://opencorde.com/" />

	<!-- Open Graph -->
	<meta property="og:type" content="website" />
	<meta property="og:url" content="https://opencorde.com/" />
	<meta property="og:title" content="OpenCorde — Self-hosted Discord alternative with E2EE" />
	<meta property="og:description" content="Feature-complete Discord alternative you run yourself. Servers, voice, video, E2EE, Discord bridge — on infrastructure you own. AGPL-3.0." />
	<meta property="og:site_name" content="OpenCorde" />

	<!-- Twitter Card -->
	<meta name="twitter:card" content="summary" />
	<meta name="twitter:title" content="OpenCorde — Self-hosted Discord alternative" />
	<meta name="twitter:description" content="Feature-complete Discord alternative you run yourself. Voice, video, E2EE, Discord bridge — AGPL-3.0." />

	<!-- Crawling -->
	<meta name="robots" content="index, follow" />
</svelte:head>

<!-- Nav -->
<nav class="fixed top-0 left-0 right-0 z-50 bg-gray-950/80 backdrop-blur border-b border-white/5">
	<div class="max-w-6xl mx-auto px-4 sm:px-6 h-14 flex items-center justify-between">
		<div class="flex items-center gap-2">
			<span class="text-white font-bold text-lg tracking-tight">OpenCorde</span>
			<span class="px-1.5 py-0.5 rounded text-xs font-semibold bg-gray-500/20 text-gray-400 border border-gray-500/30 tracking-wide">BETA</span>
		</div>
		<div class="flex items-center gap-3">
			<a
				href={GITHUB_URL}
				target="_blank"
				rel="noopener noreferrer"
				class="flex items-center gap-1.5 text-gray-400 hover:text-white text-sm transition-colors"
			>
				<svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true">
					<path fill-rule="evenodd" clip-rule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844a9.59 9.59 0 012.504.337c1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" />
				</svg>
				GitHub
			</a>
			<a href="/login" class="text-gray-400 hover:text-white text-sm transition-colors">Sign in</a>
			<a
				href="/register"
				class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 text-white text-sm rounded font-medium transition-colors"
			>
				Get started
			</a>
		</div>
	</div>
</nav>

<main class="bg-gray-950 text-white min-h-screen pt-14">

	<!-- Hero -->
	<section class="max-w-6xl mx-auto px-4 sm:px-6 pt-20 sm:pt-24 pb-16 sm:pb-20 text-center">
		<div class="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-gray-950 border border-gray-800/50 text-gray-300 text-xs font-medium mb-5 sm:mb-6">
			AGPL-3.0 · Self-hosted · E2EE · Desktop App
		</div>
		<h1 class="text-4xl sm:text-5xl md:text-6xl font-bold tracking-tight mb-4 sm:mb-5 leading-tight">
			Team communication<br />
			<span class="text-gray-400">on your terms</span>
		</h1>
		<p class="text-gray-400 text-lg max-w-2xl mx-auto mb-8 leading-relaxed">
			OpenCorde is a self-hosted team communication platform you run yourself.
			Spaces, channels, voice, video, threads, forums, end-to-end encryption, and a Discord bridge —
			all on infrastructure you own.
		</p>
		<div class="flex flex-wrap items-center justify-center gap-3">
			<a
				href="/register"
				class="px-5 py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg font-medium transition-colors"
			>
				Create an account
			</a>
			<a
				href={GITHUB_URL}
				target="_blank"
				rel="noopener noreferrer"
				class="px-5 py-2.5 bg-gray-800 hover:bg-gray-700 text-white rounded-lg font-medium transition-colors flex items-center gap-2"
			>
				<svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true">
					<path fill-rule="evenodd" clip-rule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844a9.59 9.59 0 012.504.337c1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" />
				</svg>
				View on GitHub
			</a>
			<a href="/login" class="px-5 py-2.5 bg-gray-800 hover:bg-gray-700 text-gray-300 hover:text-white rounded-lg font-medium transition-colors">
				Sign in
			</a>
		</div>
	</section>

	<!-- Features -->
	<section class="max-w-6xl mx-auto px-6 pb-20">
		<h2 class="text-2xl font-bold text-center mb-10">Everything your team needs</h2>
		<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
			{#each features as f}
				<div class="bg-gray-900 border border-gray-800 rounded-xl p-5 hover:border-gray-700 transition-colors flex flex-col">
					<div class="flex items-center gap-3 mb-3">
						<span class="text-2xl" role="img" aria-label={f.label}>{f.icon}</span>
						<h3 class="text-white font-semibold">{f.title}</h3>
					</div>
					<p class="text-gray-400 text-sm leading-relaxed">{f.desc}</p>
				</div>
			{/each}
		</div>
	</section>

	<!-- Self-hosting guide -->
	<section class="max-w-6xl mx-auto px-6 pb-24">
		<div class="text-center mb-10">
			<h2 class="text-2xl font-bold mb-2">Self-host in minutes</h2>
			<p class="text-gray-400 text-sm">Requires: Docker, Rust toolchain, Node.js 20+</p>
		</div>
		<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
			{#each steps as s}
				<div class="bg-gray-900 border border-gray-800 rounded-xl p-5">
					<div class="flex items-center gap-3 mb-3">
						<span class="w-6 h-6 rounded-full bg-indigo-600/20 text-indigo-400 text-xs font-bold flex items-center justify-center flex-shrink-0 border border-indigo-600/30">{s.n}</span>
						<span class="text-white font-medium text-sm">{s.title}</span>
					</div>
					<pre class="bg-gray-950 rounded-lg p-3 text-xs text-gray-300 font-mono overflow-x-auto leading-relaxed whitespace-pre">{s.code}</pre>
				</div>
			{/each}
		</div>
		<p class="text-center text-gray-500 text-sm mt-6">
			Full documentation and Docker Compose reference in the
			<a href="{GITHUB_URL}#readme" target="_blank" rel="noopener noreferrer" class="text-gray-400 hover:text-gray-300 underline">README</a>.
		</p>
	</section>

	<!-- Footer -->
	<footer class="border-t border-gray-800 py-8">
		<div class="max-w-6xl mx-auto px-6 flex flex-col sm:flex-row items-center justify-between gap-4 text-sm text-gray-500">
			<span>OpenCorde — AGPL-3.0-or-later</span>
			<div class="flex items-center gap-5">
				<a href={GITHUB_URL} target="_blank" rel="noopener noreferrer" class="hover:text-gray-300 transition-colors">GitHub</a>
				<a href="/register" class="hover:text-gray-300 transition-colors">Register</a>
			</div>
		</div>
	</footer>

</main>
