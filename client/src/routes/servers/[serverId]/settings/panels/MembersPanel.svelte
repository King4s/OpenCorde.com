<!--
  @component MembersPanel
  @purpose List server members with search, role assignment, kick, ban, timeout actions
  @section settings/panels
-->
<script lang="ts">
  import { roles, fetchRoles } from '$lib/stores/roles';
  import api from '$lib/api/client';
  import type { Member } from '$lib/api/types';

  let { serverId }: { serverId: string } = $props();

  let members = $state<Member[]>([]);
  let loading = $state(false);
  let error = $state('');
  let search = $state('');
  let openDropdown = $state<string | null>(null);
  let timeoutInput = $state<Record<string, string>>({});
  let showTimeoutFor = $state<string | null>(null);

  $effect(() => {
    if (serverId) {
      loadMembers();
      fetchRoles(serverId).catch(() => {});
    }
  });

  async function loadMembers() {
    loading = true;
    error = '';
    try {
      members = await api.get<Member[]>(`/servers/${serverId}/members`);
    } catch (e: any) {
      error = e.message ?? 'Failed to load members';
    } finally {
      loading = false;
    }
  }

  async function kickMember(userId: string, username: string) {
    if (!confirm(`Kick ${username}?`)) return;
    try {
      await api.delete(`/servers/${serverId}/members/${userId}`);
      members = members.filter(m => m.user_id !== userId);
    } catch (e: any) {
      error = e.message ?? 'Failed to kick member';
    }
  }

  async function banMember(userId: string, username: string) {
    if (!confirm(`Ban ${username}? They will be removed and cannot rejoin.`)) return;
    try {
      await api.put(`/servers/${serverId}/bans/${userId}`, { reason: null, delete_messages: false });
      members = members.filter(m => m.user_id !== userId);
    } catch (e: any) {
      error = e.message ?? 'Failed to ban member';
    }
  }

  async function setMemberTimeout(userId: string) {
    const minutes = parseInt(timeoutInput[userId] ?? '');
    if (!minutes || minutes <= 0) { error = 'Enter a valid duration in minutes'; return; }
    try {
      await api.put(`/servers/${serverId}/members/${userId}/timeout`, {
        duration_seconds: minutes * 60,
        reason: null
      });
      showTimeoutFor = null;
      timeoutInput = { ...timeoutInput, [userId]: '' };
    } catch (e: any) {
      error = e.message ?? 'Failed to set timeout';
    }
  }

  let filtered = $derived(
    search.trim()
      ? members.filter(m =>
          m.username.toLowerCase().includes(search.trim().toLowerCase()) ||
          (m.nickname ?? '').toLowerCase().includes(search.trim().toLowerCase())
        )
      : members
  );
</script>

<div class="p-8 max-w-2xl">
  <div class="flex items-center justify-between mb-6">
    <h1 class="text-xl font-semibold text-white">Members <span class="text-gray-400 text-base font-normal ml-1">{members.length}</span></h1>
    <input
      type="search"
      placeholder="Search members..."
      bind:value={search}
      class="px-3 py-1.5 bg-gray-900 border border-gray-700 rounded text-white text-sm placeholder-gray-500 focus:outline-none focus:border-indigo-500 w-48"
    />
  </div>

  {#if error}
    <div class="mb-4 px-3 py-2 bg-red-900/40 border border-red-700/50 rounded text-red-300 text-sm">{error}</div>
  {/if}

  {#if loading}
    <p class="text-gray-400 text-sm">Loading members...</p>
  {:else if filtered.length === 0}
    <p class="text-gray-500 text-sm">{search ? 'No members match your search.' : 'No members found.'}</p>
  {:else}
    <div class="space-y-1">
      {#each filtered as member (member.user_id)}
        <div class="flex items-center gap-3 px-3 py-2.5 rounded hover:bg-gray-700/40 group">
          <!-- Avatar placeholder -->
          <div class="w-8 h-8 rounded-full bg-indigo-700 flex items-center justify-center text-white text-xs font-bold flex-shrink-0 select-none">
            {member.username.charAt(0).toUpperCase()}
          </div>

          <!-- Name -->
          <div class="flex-1 min-w-0">
            <span class="text-gray-200 text-sm font-medium truncate block">
              {member.nickname ?? member.username}
            </span>
            {#if member.nickname}
              <span class="text-gray-500 text-xs">{member.username}</span>
            {/if}
          </div>

          <!-- Actions -->
          <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
            <!-- Timeout -->
            {#if showTimeoutFor === member.user_id}
              <div class="flex items-center gap-1">
                <input
                  type="number"
                  placeholder="mins"
                  bind:value={timeoutInput[member.user_id]}
                  min="1"
                  class="w-16 px-1.5 py-0.5 bg-gray-900 border border-gray-600 rounded text-white text-xs focus:outline-none focus:border-indigo-500"
                />
                <button
                  onclick={() => setMemberTimeout(member.user_id)}
                  class="px-2 py-0.5 bg-yellow-700 hover:bg-yellow-600 text-white text-xs rounded"
                >Set</button>
                <button
                  onclick={() => (showTimeoutFor = null)}
                  class="px-2 py-0.5 bg-gray-600 hover:bg-gray-500 text-white text-xs rounded"
                >Cancel</button>
              </div>
            {:else}
              <button
                onclick={() => (showTimeoutFor = member.user_id)}
                class="px-2 py-0.5 text-xs text-yellow-400 hover:bg-gray-700 rounded transition-colors"
                title="Timeout"
              >Timeout</button>
              <button
                onclick={() => kickMember(member.user_id, member.username)}
                class="px-2 py-0.5 text-xs text-orange-400 hover:bg-gray-700 rounded transition-colors"
              >Kick</button>
              <button
                onclick={() => banMember(member.user_id, member.username)}
                class="px-2 py-0.5 text-xs text-red-400 hover:bg-gray-700 rounded transition-colors"
              >Ban</button>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
