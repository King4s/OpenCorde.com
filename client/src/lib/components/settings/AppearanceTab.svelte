<script lang="ts">
	/**
	 * @file AppearanceTab — theme and message display settings
	 * @purpose Toggle dark/light theme and message density (cozy/compact)
	 * @depends $lib/stores/theme
	 */
	import { themeStore } from '$lib/stores/theme';

	const messageStyle = themeStore.messageStyle;
</script>

<div class="space-y-4">
	<div class="bg-gray-800 rounded-lg p-4 space-y-4">
		<h2 class="text-sm font-semibold text-gray-400 uppercase">Appearance</h2>
		<div class="flex items-center justify-between">
			<span class="text-sm text-gray-300">Theme</span>
			<button onclick={() => themeStore.toggle()}
				class="px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white text-sm font-medium rounded transition-colors">
				{themeStore.isDark ? '☀️ Light Mode' : '🌙 Dark Mode'}
			</button>
		</div>

		<!-- Message Display -->
		<div>
			<span class="text-sm text-gray-300 block mb-2">Message Display</span>
			<div class="flex gap-2">
				<button
					onclick={() => $messageStyle === 'cozy' || themeStore.toggleMessageStyle()}
					class="px-4 py-2 text-sm font-medium rounded transition-colors {$messageStyle === 'cozy' ? 'bg-gray-600 text-white' : 'bg-gray-700 text-gray-300 hover:bg-gray-600'}"
				>
					Cozy
				</button>
				<button
					onclick={() => $messageStyle === 'compact' || themeStore.toggleMessageStyle()}
					class="px-4 py-2 text-sm font-medium rounded transition-colors {$messageStyle === 'compact' ? 'bg-gray-600 text-white' : 'bg-gray-700 text-gray-300 hover:bg-gray-600'}"
				>
					Compact
				</button>
			</div>
		</div>
	</div>
</div>
