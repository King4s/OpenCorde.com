<!--
  @page Server Events
  @purpose View and manage scheduled events for a server.
-->
<script lang="ts">
  import { page } from '$app/stores';
  import { eventStore } from '$lib/stores/events.svelte';
  import EventCard from '$lib/components/events/EventCard.svelte';
  import { onMount } from 'svelte';

  let spaceId = $derived($page.params.serverId ?? '');
  let showCreate = $state(false);
  let creating = $state(false);
  let createError = $state('');

  // Create form fields
  let title = $state('');
  let description = $state('');
  let locationType = $state('external');
  let locationName = $state('');
  let startsAt = $state('');
  let endsAt = $state('');

  onMount(async () => {
    await eventStore.fetchForServer(spaceId);
  });

  async function handleCreate() {
    if (!title.trim() || !startsAt) return;
    creating = true;
    createError = '';
    try {
      await eventStore.create(spaceId, {
        title: title.trim(),
        description: description.trim() || undefined,
        location_type: locationType,
        location_name: locationName.trim() || undefined,
        starts_at: new Date(startsAt).toISOString(),
        ends_at: endsAt ? new Date(endsAt).toISOString() : undefined,
      });
      showCreate = false;
      title = ''; description = ''; locationName = ''; startsAt = ''; endsAt = '';
    } catch (e: any) {
      createError = (e as { message?: string }).message ?? 'Failed to create event';
    } finally {
      creating = false;
    }
  }
</script>

<div class="events-page">
  <div class="events-header">
    <h1>Events</h1>
    <button class="create-btn" onclick={() => showCreate = !showCreate}>
      {showCreate ? '✕ Cancel' : '+ Create Event'}
    </button>
  </div>

  {#if showCreate}
    <div class="create-form">
      <h3>New Event</h3>
      <div class="form-grid">
        <label>
          Title *
          <input bind:value={title} placeholder="Game Night" maxlength="100" />
        </label>
        <label>
          Location Type
          <select bind:value={locationType}>
            <option value="external">External</option>
            <option value="voice">Voice Channel</option>
            <option value="stage">Stage</option>
          </select>
        </label>
        <label>
          Location Name
          <input bind:value={locationName} placeholder="Discord / Twitch / etc." />
        </label>
        <label>
          Starts At *
          <input type="datetime-local" bind:value={startsAt} />
        </label>
        <label>
          Ends At
          <input type="datetime-local" bind:value={endsAt} />
        </label>
        <label class="full-width">
          Description
          <textarea bind:value={description} placeholder="What's happening?" rows="3"></textarea>
        </label>
      </div>
      {#if createError}<p class="error">{createError}</p>{/if}
      <button class="submit-btn" onclick={handleCreate} disabled={creating || !title.trim() || !startsAt}>
        {creating ? 'Creating...' : 'Create Event'}
      </button>
    </div>
  {/if}

  {#if eventStore.loading}
    <p class="loading">Loading events...</p>
  {:else if eventStore.events.length === 0}
    <div class="empty-state">
      <p>No upcoming events. Create one!</p>
    </div>
  {:else}
    <div class="events-grid">
      {#each eventStore.events as event (event.id)}
        <EventCard {event} canManage={true} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .events-page { padding: 24px; max-width: 800px; }
  .events-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 24px; }
  .events-header h1 { margin: 0; font-size: 20px; font-weight: 700; color: #f2f3f5; }
  .create-btn {
    background: #e5e7eb;
    border: none;
    border-radius: 4px;
    color: white;
    padding: 8px 16px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 600;
  }
  .create-btn:hover { background: #4752c4; }
  .create-form {
    background: #2b2d31;
    border-radius: 8px;
    padding: 20px;
    margin-bottom: 24px;
    border: 1px solid #35373c;
  }
  .create-form h3 { margin: 0 0 16px 0; color: #f2f3f5; }
  .form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin-bottom: 16px; }
  .full-width { grid-column: 1 / -1; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: #b5bac1; font-weight: 600; text-transform: uppercase; letter-spacing: 0.5px; }
  input, select, textarea {
    background: #1e1f22;
    border: 1px solid #35373c;
    border-radius: 4px;
    color: #dbdee1;
    padding: 8px 12px;
    font-size: 14px;
    outline: none;
    font-family: inherit;
  }
  input:focus, select:focus, textarea:focus { border-color: #e5e7eb; }
  textarea { resize: vertical; }
  .error { color: #ed4245; font-size: 13px; margin: 0 0 12px 0; }
  .submit-btn {
    background: #e5e7eb;
    border: none;
    border-radius: 4px;
    color: white;
    padding: 10px 24px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 600;
  }
  .submit-btn:disabled { opacity: 0.5; cursor: default; }
  .loading, .empty-state { color: #b5bac1; padding: 40px; text-align: center; }
  .events-grid { display: flex; flex-direction: column; gap: 12px; }
</style>
