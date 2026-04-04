<script lang="ts">
	/**
	 * @file Channel settings modal
	 * @purpose Edit channel settings (Overview tab) and manage permission overrides (Permissions tab)
	 */
import api from '$lib/api/client';
import { initE2EEGroup } from '$lib/stores/e2ee';
import { edgeResize } from '$lib/actions/edgeResize';
import ChannelPermissionsTab from './ChannelPermissionsTab.svelte';

	type Tab = 'overview' | 'permissions';

	interface Props {
		channelId: string;
		spaceId: string;
		channelName: string;
		channelTopic: string | null;
		channelNsfw: boolean;
		channelSlowmode: number;
		channelE2EEEnabled?: boolean;
		onClose: () => void;
		onSave: (updated: { name: string; topic: string | null; nsfw: boolean; slowmode_delay: number }) => void;
	}

	let { channelId, spaceId, channelName, channelTopic, channelNsfw, channelSlowmode, channelE2EEEnabled = false, onClose, onSave }: Props =
		$props();

	let activeTab = $state<Tab>('overview');
	let name = $state('');
	let topic = $state('');
	let nsfw = $state(false);
	let slowmodeDelay = $state(0);
	let e2eeEnabled = $state(false);
	let saving = $state(false);
	let error = $state('');
	let idCopied = $state(false);

	function copyChannelId() {
		navigator.clipboard.writeText(channelId).then(() => {
			idCopied = true;
			setTimeout(() => { idCopied = false; }, 1500);
		});
	}

	// Initialise form state from props whenever they change
	$effect(() => {
		name = channelName;
		topic = channelTopic ?? '';
		nsfw = channelNsfw;
		slowmodeDelay = channelSlowmode;
		e2eeEnabled = channelE2EEEnabled ?? false;
	});

	async function handleSave() {
		saving = true;
		error = '';
		try {
			const body: Record<string, unknown> = {
				name: name.trim(),
				topic: topic.trim() || null,
				nsfw,
				slowmode_delay: slowmodeDelay
			};
			await api.patch(`/channels/${channelId}`, body);
			onSave({ name: name.trim(), topic: topic.trim() || null, nsfw, slowmode_delay: slowmodeDelay });
			onClose();
		} catch (e: any) {
			error = e.message ?? 'Failed to save channel settings';
		} finally {
			saving = false;
		}
	}

	async function handleE2EEToggle(): Promise<void> {
		const newVal = !e2eeEnabled;
		try {
			const updated = await api.patch<{ e2ee_enabled: boolean }>(`/channels/${channelId}`, { e2ee_enabled: newVal });
			e2eeEnabled = updated.e2ee_enabled;
			// If enabling and in Tauri: fetch member key packages and init group
			if (newVal && typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
				const members = await api.get<{ id: string }[]>(`/servers/${spaceId}/members`);
				const kpMap = new Map<string, string>();
				for (const m of members) {
					try {
						const kpRes = await api.get<{ key_package: string }>(`/users/${m.id}/key-packages/one`);
						kpMap.set(m.id, kpRes.key_package);
					} catch { /* member has no key package */ }
				}
				if (kpMap.size > 0) {
					await initE2EEGroup(channelId, kpMap);
				}
			}
		} catch (err: any) {
			console.error('[E2EE] Toggle failed:', err);
		}
	}

	function handleCancel() {
		onClose();
	}
</script>

<!-- Modal backdrop -->
<div
	class="fixed inset-0 z-50 flex items-start justify-center bg-black/60 px-4 py-6 backdrop-blur-sm"
	role="presentation"
	onclick={handleCancel}
	onkeydown={(e) => e.key === 'Escape' && handleCancel()}
>
	<!-- Modal content (stop propagation) -->
	<div
		use:edgeResize={{ handles: ['left', 'right'], minWidth: 540, maxWidth: 1000 }}
		class="w-[760px] max-w-[calc(100vw-2rem)] max-h-[calc(100vh-3rem)] overflow-auto rounded-2xl border border-gray-700 bg-gray-800 shadow-2xl ring-1 ring-white/10 flex flex-col"
		style="min-width: 540px; min-height: 420px;"
		role="dialog"
		aria-modal="true"
		aria-labelledby="modal-title"
		tabindex={-1}
		onclick={(e) => e.stopPropagation()}
		onkeydown={(e) => e.stopPropagation()}
	>
		<!-- Header -->
		<div class="px-6 py-4 border-b border-gray-700 flex-shrink-0 flex items-start justify-between gap-4 bg-gray-800/95 backdrop-blur">
			<div>
				<p class="text-[11px] font-semibold uppercase tracking-[0.2em] text-gray-500">Floating editor</p>
				<h2 id="modal-title" class="text-lg font-semibold text-white">Channel Settings</h2>
			</div>
			<button
				onclick={handleCancel}
				class="rounded-md p-2 text-gray-400 hover:bg-gray-700 hover:text-white transition-colors"
				aria-label="Close channel settings"
				title="Close"
			>
				✕
			</button>
		</div>

		<!-- Tab bar -->
		<div class="flex border-b border-gray-700 flex-shrink-0 px-6 bg-gray-800/90">
			<button
				class="py-3 px-1 mr-6 text-sm font-medium border-b-2 transition-colors {activeTab === 'overview'
					? 'border-gray-500 text-white'
					: 'border-transparent text-gray-400 hover:text-gray-200'}"
				onclick={() => (activeTab = 'overview')}
			>
				Overview
			</button>
			<button
				class="py-3 px-1 text-sm font-medium border-b-2 transition-colors {activeTab === 'permissions'
					? 'border-gray-500 text-white'
					: 'border-transparent text-gray-400 hover:text-gray-200'}"
				onclick={() => (activeTab = 'permissions')}
			>
				Permissions
			</button>
		</div>

		<!-- Scrollable body -->
		<div class="px-6 py-5 overflow-y-auto flex-1 bg-gray-800/85">
			{#if activeTab === 'overview'}
				<div class="space-y-4">
					{#if error}
						<div
							class="px-3 py-2 bg-gray-900/40 border border-gray-700/50 rounded text-gray-300 text-sm"
						>
							{error}
						</div>
					{/if}

					<!-- Name input -->
					<div>
						<label class="block text-xs text-gray-400 mb-1.5" for="modal-channel-name"
							>Channel Name</label
						>
						<input
							id="modal-channel-name"
							type="text"
							bind:value={name}
							maxlength="100"
							placeholder="Channel name"
							class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-gray-500"
						/>
					</div>

					<!-- Topic input -->
					<div>
						<label class="block text-xs text-gray-400 mb-1.5" for="modal-channel-topic"
							>Topic (Optional)</label
						>
						<input
							id="modal-channel-topic"
							type="text"
							bind:value={topic}
							maxlength="512"
							placeholder="Channel topic"
							class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-gray-500"
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
						<label for="modal-channel-nsfw" class="text-sm text-gray-300 cursor-pointer"
							>Mark as NSFW</label
						>
					</div>

					<!-- Slowmode -->
					<div>
						<label class="block text-xs text-gray-400 mb-1.5" for="modal-slowmode">
							Slowmode (seconds, 0 = off)
						</label>
						<input
							id="modal-slowmode"
							type="number"
							bind:value={slowmodeDelay}
							min="0"
							max="21600"
							placeholder="0"
							class="w-32 px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-gray-500"
						/>
					</div>

					<!-- Channel ID (for bridge / developer use) -->
					<div class="flex items-center justify-between py-2 border-t border-gray-700/50">
						<div>
							<span class="text-xs font-medium text-gray-400 uppercase tracking-wide">Channel ID</span>
							<p class="text-xs font-mono text-gray-300 mt-0.5 select-all">{channelId}</p>
						</div>
						<button
							onclick={copyChannelId}
							class="px-2 py-1 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-300 transition-colors"
							title="Copy channel ID"
						>
							{idCopied ? 'Copied!' : 'Copy'}
						</button>
					</div>

					<!-- E2EE toggle -->
					<div class="flex items-center justify-between py-2">
						<div>
							<span class="text-sm font-medium text-gray-200">End-to-End Encryption</span>
							<p class="text-xs text-gray-400 mt-0.5">Encrypt messages using OpenMLS (Tauri app only)</p>
						</div>
						<button
							onclick={handleE2EEToggle}
							class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {e2eeEnabled ? 'bg-gray-600' : 'bg-gray-600'}"
							aria-label="Toggle E2EE"
						>
							<span class="inline-block h-4 w-4 transform rounded-full bg-white transition-transform {e2eeEnabled ? 'translate-x-6' : 'translate-x-1'}"></span>
						</button>
					</div>
				</div>
			{:else}
				<ChannelPermissionsTab {channelId} {spaceId} />
			{/if}
		</div>

		<!-- Footer (only shown on overview tab) -->
		{#if activeTab === 'overview'}
			<div class="px-6 py-4 border-t border-gray-700 bg-gray-800/95 backdrop-blur flex gap-2 justify-end flex-shrink-0">
				<button
					onclick={handleCancel}
					class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-gray-300 text-sm font-medium rounded transition-colors"
				>
					Cancel
				</button>
				<button
					onclick={handleSave}
					disabled={saving}
					class="px-4 py-2 bg-gray-600 hover:bg-gray-700 disabled:opacity-50 text-white text-sm font-medium rounded transition-colors"
				>
					{saving ? 'Saving...' : 'Save'}
				</button>
			</div>
		{:else}
			<div class="px-6 py-3 border-t border-gray-700 flex justify-end flex-shrink-0">
				<button
					onclick={handleCancel}
					class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-gray-300 text-sm font-medium rounded transition-colors"
				>
					Close
				</button>
			</div>
		{/if}
	</div>
</div>
