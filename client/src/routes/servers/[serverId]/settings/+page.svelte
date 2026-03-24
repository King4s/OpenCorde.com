<script lang="ts">
  /**
   * @file Server settings page shell
   * @purpose Discord-style settings layout: left sidebar nav + right panel content area
   * @section routes/servers/[serverId]/settings
   */
  import { browser } from '$app/environment';
  import OverviewPanel from './panels/OverviewPanel.svelte';
  import RolesPanel from './panels/RolesPanel.svelte';
  import MembersPanel from './panels/MembersPanel.svelte';
  import BansPanel from './panels/BansPanel.svelte';
  import InvitesPanel from './panels/InvitesPanel.svelte';
  import ModerationPanel from './panels/ModerationPanel.svelte';
  import AutomodPanel from './panels/AutomodPanel.svelte';
  import AuditLogPanel from './panels/AuditLogPanel.svelte';
  import IntegrationsPanel from './panels/IntegrationsPanel.svelte';
  import EmojisPanel from './panels/EmojisPanel.svelte';

  type Section =
    | 'overview'
    | 'roles'
    | 'members'
    | 'bans'
    | 'invites'
    | 'moderation'
    | 'automod'
    | 'audit-log'
    | 'integrations'
    | 'emojis';

  interface NavGroup {
    label: string;
    items: { id: Section; label: string }[];
  }

  const navGroups: NavGroup[] = [
    {
      label: 'Overview',
      items: [{ id: 'overview', label: 'Overview' }]
    },
    {
      label: 'Server Settings',
      items: [
        { id: 'roles', label: 'Roles' },
        { id: 'members', label: 'Members' },
        { id: 'bans', label: 'Bans' },
        { id: 'invites', label: 'Invites' }
      ]
    },
    {
      label: 'Moderation',
      items: [
        { id: 'moderation', label: 'Moderation' },
        { id: 'automod', label: 'AutoMod' },
        { id: 'audit-log', label: 'Audit Log' }
      ]
    },
    {
      label: 'Integrations',
      items: [
        { id: 'integrations', label: 'Slash Commands' },
        { id: 'emojis', label: 'Emojis' }
      ]
    }
  ];

  let serverId = $state('');
  let activeSection = $state<Section>('overview');

  if (browser) {
    const match = window.location.pathname.match(/\/servers\/([^/]+)/);
    serverId = match?.[1] ?? '';
  }

  function navigate(section: Section) {
    activeSection = section;
  }

  function goBack() {
    if (browser) {
      window.location.href = `/servers/${serverId}`;
    }
  }
</script>

<div class="flex h-full bg-gray-800 text-white overflow-hidden">

  <!-- Left sidebar -->
  <nav class="w-56 flex-shrink-0 bg-gray-850 border-r border-gray-700 flex flex-col overflow-y-auto" style="background-color: #1e2124;">
    <div class="px-4 pt-6 pb-2">
      <p class="text-xs font-bold text-gray-400 uppercase tracking-wider truncate">Server Settings</p>
    </div>

    {#each navGroups as group}
      <div class="px-2 mt-4">
        <p class="px-2 mb-1 text-[11px] font-bold text-gray-500 uppercase tracking-wider">{group.label}</p>
        {#each group.items as item}
          <button
            onclick={() => navigate(item.id)}
            class="w-full text-left px-2 py-1.5 rounded text-sm transition-colors {activeSection === item.id
              ? 'bg-gray-700 text-white font-medium'
              : 'text-gray-400 hover:bg-gray-700/50 hover:text-gray-200'}"
          >
            {item.label}
          </button>
        {/each}
      </div>
    {/each}

    <!-- Divider + back button -->
    <div class="mt-auto px-2 pb-4 pt-4 border-t border-gray-700">
      <button
        onclick={goBack}
        class="w-full text-left px-2 py-1.5 rounded text-sm text-gray-400 hover:bg-gray-700/50 hover:text-gray-200 transition-colors flex items-center gap-2"
      >
        <span class="text-xs">&#8592;</span>
        Back to Server
      </button>
    </div>
  </nav>

  <!-- Right content panel -->
  <main class="flex-1 overflow-y-auto bg-gray-750" style="background-color: #313338;">
    {#if serverId}
      {#if activeSection === 'overview'}
        <OverviewPanel {serverId} />
      {:else if activeSection === 'roles'}
        <RolesPanel {serverId} />
      {:else if activeSection === 'members'}
        <MembersPanel {serverId} />
      {:else if activeSection === 'bans'}
        <BansPanel {serverId} />
      {:else if activeSection === 'invites'}
        <InvitesPanel {serverId} />
      {:else if activeSection === 'moderation'}
        <ModerationPanel {serverId} />
      {:else if activeSection === 'automod'}
        <AutomodPanel {serverId} />
      {:else if activeSection === 'audit-log'}
        <AuditLogPanel {serverId} />
      {:else if activeSection === 'integrations'}
        <IntegrationsPanel {serverId} />
      {:else if activeSection === 'emojis'}
        <EmojisPanel {serverId} />
      {/if}
    {:else}
      <div class="p-8 text-gray-400 text-sm">Loading...</div>
    {/if}
  </main>
</div>
