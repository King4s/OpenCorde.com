<!--
  @component RolesPermissionsPanel
  @purpose Space roles and permissions hub — role management plus guidance for permission overrides
  @section settings/panels
-->
<script lang="ts">
  import RolesPanel from './RolesPanel.svelte';

  let { spaceId }: { spaceId: string } = $props();

  const corePermissions = [
    'View channels',
    'Send messages',
    'Manage messages',
    'Connect to voice',
    'Speak in voice',
    'Attach files',
    'Add reactions',
    'Manage channels'
  ];
</script>

<div class="p-8 max-w-4xl space-y-6">
  <div>
    <h1 class="text-xl font-semibold text-white mb-2">Roles & Permissions</h1>
    <p class="text-gray-400 text-sm max-w-2xl">
      Manage server roles here. Channel-specific exceptions still live in each channel's Permissions tab,
      but this is the place to create, rename, color, and organize the roles that drive access control.
    </p>
  </div>

  <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
    <div class="bg-gray-900 border border-gray-700 rounded-lg p-4">
      <h2 class="text-sm font-semibold text-gray-300 uppercase mb-3">Core permission categories</h2>
      <ul class="space-y-2 text-sm text-gray-300">
        {#each corePermissions as permission}
          <li class="flex items-center gap-2">
            <span class="w-2 h-2 rounded-full bg-gray-500"></span>
            <span>{permission}</span>
          </li>
        {/each}
      </ul>
      <p class="text-gray-500 text-xs mt-4">
        This page will later host a true matrix view. For now, the backend role model and channel overrides already exist.
      </p>
    </div>

    <div class="bg-gray-900 border border-gray-700 rounded-lg p-4">
      <h2 class="text-sm font-semibold text-gray-300 uppercase mb-3">How permissions work</h2>
      <div class="space-y-3 text-sm text-gray-300 leading-relaxed">
        <p>• Space roles grant base access.</p>
        <p>• Channel overrides can allow, deny, or inherit per target.</p>
        <p>• The most specific override wins.</p>
        <p>• Use channel settings when you need exceptions for one place only.</p>
      </div>
    </div>
  </div>

  <div class="bg-gray-800 rounded-xl border border-gray-700">
    <RolesPanel {spaceId} />
  </div>
</div>
