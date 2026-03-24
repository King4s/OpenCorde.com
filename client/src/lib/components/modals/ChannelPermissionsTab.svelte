<script lang="ts">
	/**
	 * @file Channel permissions tab — per-role/member override editor
	 * @purpose List, add, edit, and delete channel permission overrides
	 */
	import api from '$lib/api/client';
	import { roles } from '$lib/stores/roles';

	interface Override {
		id: string;
		channel_id: string;
		target_type: string;
		target_id: string;
		allow: number;
		deny: number;
	}

	interface Props {
		channelId: string;
		serverId: string;
	}

	let { channelId, serverId }: Props = $props();

	// Permission bit definitions shown in the UI
	const PERMS = [
		{ bit: 1 << 10, label: 'View Channel' },
		{ bit: 1 << 11, label: 'Send Messages' },
		{ bit: 1 << 13, label: 'Manage Messages' },
		{ bit: 1 << 20, label: 'Connect (voice)' },
		{ bit: 1 << 21, label: 'Speak (voice)' },
		{ bit: 1 << 15, label: 'Attach Files' },
		{ bit: 1 << 6,  label: 'Add Reactions' },
		{ bit: 1 << 4,  label: 'Manage Channel' },
	];

	// State: overrides fetched from server
	let overrides = $state<Override[]>([]);
	// Local edits: map override id -> { allow, deny }
	let edits = $state<Record<string, { allow: number; deny: number }>>({});
	let loading = $state(true);
	let error = $state('');
	let saving = $state<Record<string, boolean>>({});

	// Fetch overrides on mount
	$effect(() => {
		loadOverrides();
	});

	async function loadOverrides() {
		loading = true;
		error = '';
		try {
			const list = await api.get<Override[]>(`/channels/${channelId}/permissions`);
			overrides = list;
			// Seed local edits from fetched data
			const seed: Record<string, { allow: number; deny: number }> = {};
			for (const ov of list) {
				seed[ov.id] = { allow: ov.allow, deny: ov.deny };
			}
			edits = seed;
		} catch (e: any) {
			error = e.message ?? 'Failed to load permissions';
		} finally {
			loading = false;
		}
	}

	/** Add a new role override row (optimistic — PUT immediately with 0/0) */
	async function addRoleOverride(roleId: string) {
		if (overrides.some((o) => o.target_type === 'role' && o.target_id === roleId)) return;
		try {
			const row = await api.put<Override>(
				`/channels/${channelId}/permissions/role/${roleId}`,
				{ allow: 0, deny: 0 }
			);
			overrides = [...overrides, row];
			edits = { ...edits, [row.id]: { allow: 0, deny: 0 } };
		} catch (e: any) {
			error = e.message ?? 'Failed to add override';
		}
	}

	/** Save edits for one override row */
	async function saveOverride(ov: Override) {
		saving = { ...saving, [ov.id]: true };
		try {
			const e = edits[ov.id] ?? { allow: ov.allow, deny: ov.deny };
			await api.put(
				`/channels/${channelId}/permissions/${ov.target_type}/${ov.target_id}`,
				{ allow: e.allow, deny: e.deny }
			);
			// Update local overrides array
			overrides = overrides.map((o) =>
				o.id === ov.id ? { ...o, allow: e.allow, deny: e.deny } : o
			);
		} catch (e: any) {
			error = e.message ?? 'Failed to save override';
		} finally {
			saving = { ...saving, [ov.id]: false };
		}
	}

	/** Remove an override row */
	async function removeOverride(ov: Override) {
		try {
			await api.delete(`/channels/${channelId}/permissions/${ov.target_type}/${ov.target_id}`);
			overrides = overrides.filter((o) => o.id !== ov.id);
			const { [ov.id]: _, ...rest } = edits;
			edits = rest;
		} catch (e: any) {
			error = e.message ?? 'Failed to remove override';
		}
	}

	/** Toggle a permission bit in allow/deny/inherit cycle */
	function toggleBit(ovId: string, bit: number, field: 'allow' | 'deny') {
		const cur = edits[ovId] ?? { allow: 0, deny: 0 };
		const other = field === 'allow' ? 'deny' : 'allow';
		// If bit already set in this field: clear it (inherit)
		// If bit set in other field: clear other, set this
		// Otherwise: set this
		if (cur[field] & bit) {
			edits = { ...edits, [ovId]: { ...cur, [field]: cur[field] & ~bit } };
		} else {
			edits = {
				...edits,
				[ovId]: { [field]: cur[field] | bit, [other]: cur[other] & ~bit }
			};
		}
	}

	function getState(ovId: string, bit: number): 'allow' | 'deny' | 'inherit' {
		const e = edits[ovId] ?? { allow: 0, deny: 0 };
		if (e.allow & bit) return 'allow';
		if (e.deny & bit) return 'deny';
		return 'inherit';
	}

	function labelForId(ov: Override): string {
		if (ov.target_type === 'role') {
			return $roles.find((r) => r.id === ov.target_id)?.name ?? `Role ${ov.target_id}`;
		}
		return `User ${ov.target_id}`;
	}

	function roleColor(ov: Override): string {
		if (ov.target_type !== 'role') return '#6b7280';
		const c = $roles.find((r) => r.id === ov.target_id)?.color;
		if (!c) return '#6b7280';
		return '#' + c.toString(16).padStart(6, '0');
	}

	// Roles not yet having an override
	const availableRoles = $derived(
		$roles.filter((r) => !overrides.some((o) => o.target_type === 'role' && o.target_id === r.id))
	);
</script>

{#if loading}
	<div class="py-6 text-center text-gray-400 text-sm">Loading permissions...</div>
{:else}
	{#if error}
		<div class="mb-3 px-3 py-2 bg-red-900/40 border border-red-700/50 rounded text-red-300 text-sm">
			{error}
		</div>
	{/if}

	<!-- Add role override -->
	{#if availableRoles.length > 0}
		<div class="mb-4">
			<label class="block text-xs text-gray-400 mb-1.5">Add role override</label>
			<select
				class="w-full px-3 py-2 bg-gray-900 border border-gray-700 rounded text-white text-sm focus:outline-none focus:border-indigo-500"
				onchange={(e) => {
					const v = (e.target as HTMLSelectElement).value;
					if (v) { addRoleOverride(v); (e.target as HTMLSelectElement).value = ''; }
				}}
			>
				<option value="">-- Select a role --</option>
				{#each availableRoles as role}
					<option value={role.id}>{role.name}</option>
				{/each}
			</select>
		</div>
	{/if}

	<!-- Override rows -->
	{#if overrides.length === 0}
		<p class="text-gray-500 text-sm text-center py-4">No overrides configured.</p>
	{:else}
		<div class="space-y-4">
			{#each overrides as ov (ov.id)}
				<div class="bg-gray-900 rounded p-3 border border-gray-700">
					<!-- Header row -->
					<div class="flex items-center justify-between mb-3">
						<div class="flex items-center gap-2">
							<span
								class="inline-block w-2.5 h-2.5 rounded-full"
								style="background:{roleColor(ov)}"
							></span>
							<span class="text-white text-sm font-medium">{labelForId(ov)}</span>
							<span class="text-gray-500 text-xs uppercase">{ov.target_type}</span>
						</div>
						<button
							class="text-gray-500 hover:text-red-400 text-lg leading-none px-1"
							onclick={() => removeOverride(ov)}
							aria-label="Remove override"
						>×</button>
					</div>

					<!-- Permission toggles -->
					<div class="space-y-1.5">
						{#each PERMS as perm}
							{@const state = getState(ov.id, perm.bit)}
							<div class="flex items-center justify-between">
								<span class="text-gray-300 text-xs">{perm.label}</span>
								<div class="flex gap-1">
									<!-- Allow -->
									<button
										class="w-7 h-7 rounded text-xs font-bold transition-colors {state === 'allow' ? 'bg-green-600 text-white' : 'bg-gray-700 text-gray-400 hover:bg-gray-600'}"
										onclick={() => toggleBit(ov.id, perm.bit, 'allow')}
										title="Allow"
									>✓</button>
									<!-- Inherit -->
									<button
										class="w-7 h-7 rounded text-xs font-bold transition-colors {state === 'inherit' ? 'bg-gray-500 text-white' : 'bg-gray-700 text-gray-400 hover:bg-gray-600'}"
										onclick={() => {
											const cur = edits[ov.id] ?? { allow: 0, deny: 0 };
											edits = { ...edits, [ov.id]: { allow: cur.allow & ~perm.bit, deny: cur.deny & ~perm.bit } };
										}}
										title="Inherit"
									>–</button>
									<!-- Deny -->
									<button
										class="w-7 h-7 rounded text-xs font-bold transition-colors {state === 'deny' ? 'bg-red-600 text-white' : 'bg-gray-700 text-gray-400 hover:bg-gray-600'}"
										onclick={() => toggleBit(ov.id, perm.bit, 'deny')}
										title="Deny"
									>✗</button>
								</div>
							</div>
						{/each}
					</div>

					<!-- Save button -->
					<div class="mt-3 flex justify-end">
						<button
							onclick={() => saveOverride(ov)}
							disabled={saving[ov.id]}
							class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white text-xs font-medium rounded transition-colors"
						>
							{saving[ov.id] ? 'Saving...' : 'Save'}
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
{/if}
