<script lang="ts">
  /**
   * @file RolePermissionsTab.svelte — Permission flag editor for a single role
   * @purpose Render grouped permission toggles; emit change events on toggle
   * @used-by RoleManager.svelte
   */
  import {
    PERMISSIONS,
    PERMISSION_CATEGORIES,
    hasPermission,
    togglePermission,
    type PermissionKey,
    type PermissionCategory,
  } from '$lib/utils/permissions';

  interface Props {
    /** Current permission bitfield for the role being edited */
    permissions: bigint;
    /** Called whenever a flag is toggled; parent owns the state */
    onChange: (newPerms: bigint) => void;
  }

  let { permissions, onChange }: Props = $props();

  const ADMIN_BIT = PERMISSIONS.ADMINISTRATOR.bit;

  let isAdmin = $derived(hasPermission(permissions, ADMIN_BIT));

  function permsByCategory(cat: PermissionCategory) {
    return (Object.keys(PERMISSIONS) as PermissionKey[]).filter(
      (k) => PERMISSIONS[k].category === cat
    );
  }

  // Pre-compute General keys (excluding ADMINISTRATOR which has its own banner)
  const generalKeys = (Object.keys(PERMISSIONS) as PermissionKey[]).filter(
    (k) => PERMISSIONS[k].category === 'General' && k !== 'ADMINISTRATOR'
  );

  function toggle(key: PermissionKey, enabled: boolean) {
    if (key === 'ADMINISTRATOR') {
      onChange(togglePermission(permissions, ADMIN_BIT, enabled));
      return;
    }
    if (isAdmin) return; // locked when admin is on
    onChange(togglePermission(permissions, PERMISSIONS[key].bit, enabled));
  }

  function isChecked(key: PermissionKey): boolean {
    if (isAdmin && key !== 'ADMINISTRATOR') return true;
    return hasPermission(permissions, PERMISSIONS[key].bit);
  }
</script>

<div class="space-y-5">
  <!-- Administrator banner -->
  <div class="rounded-lg border border-yellow-600/40 bg-yellow-900/20 p-3">
    <label class="flex items-center gap-3 cursor-pointer">
      <input
        type="checkbox"
        checked={isAdmin}
        onchange={(e) => toggle('ADMINISTRATOR', (e.target as HTMLInputElement).checked)}
        class="w-4 h-4 accent-yellow-500"
      />
      <div class="flex-1">
        <p class="text-yellow-300 text-sm font-semibold">Administrator</p>
        <p class="text-yellow-400/70 text-xs">All permissions, bypasses channel overrides</p>
      </div>
    </label>
    {#if isAdmin}
      <p class="mt-2 text-xs text-yellow-400/60 pl-7">
        This role has full access — all other flags are implied.
      </p>
    {/if}
  </div>

  <!-- Per-category groups (skip General since ADMINISTRATOR lives there and we show it above) -->
  {#each PERMISSION_CATEGORIES as category}
    {#if category !== 'General'}
      {@const keys = permsByCategory(category)}
      {#if keys.length > 0}
        <div>
          <p class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2">
            {category}
          </p>
          <div class="space-y-1">
            {#each keys as key}
              {@const perm = PERMISSIONS[key]}
              <label
                class="flex items-center gap-3 px-2 py-1.5 rounded hover:bg-gray-700/40
                       {isAdmin ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}"
              >
                <input
                  type="checkbox"
                  checked={isChecked(key)}
                  disabled={isAdmin}
                  onchange={(e) => toggle(key, (e.target as HTMLInputElement).checked)}
                  class="w-4 h-4 accent-indigo-500 flex-shrink-0"
                />
                <div class="flex-1 min-w-0">
                  <p class="text-gray-200 text-sm">{perm.label}</p>
                  <p class="text-gray-500 text-xs truncate">{perm.desc}</p>
                </div>
              </label>
            {/each}
          </div>
        </div>
      {/if}
    {/if}
  {/each}

  <!-- General group (excluding ADMINISTRATOR which is shown above) -->
  {#if generalKeys.length > 0}
    <div>
      <p class="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2">General</p>
      <div class="space-y-1">
        {#each generalKeys as key}
          {@const perm = PERMISSIONS[key]}
          <label
            class="flex items-center gap-3 px-2 py-1.5 rounded hover:bg-gray-700/40
                   {isAdmin ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}"
          >
            <input
              type="checkbox"
              checked={isChecked(key)}
              disabled={isAdmin}
              onchange={(e) => toggle(key, (e.target as HTMLInputElement).checked)}
              class="w-4 h-4 accent-indigo-500 flex-shrink-0"
            />
            <div class="flex-1 min-w-0">
              <p class="text-gray-200 text-sm">{perm.label}</p>
              <p class="text-gray-500 text-xs truncate">{perm.desc}</p>
            </div>
          </label>
        {/each}
      </div>
    </div>
  {/if}
</div>
