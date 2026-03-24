<!--
  @component EventCard
  @purpose Displays a single server event with RSVP button.
  @version 1.0.0
-->
<script lang="ts">
  import type { ServerEvent } from '$lib/stores/events.svelte';
  import { eventStore } from '$lib/stores/events.svelte';

  let { event, canManage = false }: { event: ServerEvent; canManage?: boolean } = $props();

  let rsvped = $state(false);

  function formatDate(ts: string) {
    return new Date(ts).toLocaleDateString([], {
      weekday: 'short', month: 'short', day: 'numeric',
      hour: '2-digit', minute: '2-digit'
    });
  }

  function getStatusColor(status: string) {
    switch (status) {
      case 'active': return '#3ba55c';
      case 'completed': return '#b5bac1';
      case 'cancelled': return '#ed4245';
      default: return '#5865f2';
    }
  }

  async function handleRsvp() {
    if (rsvped) {
      await eventStore.unRsvp(event.id);
      rsvped = false;
    } else {
      await eventStore.rsvp(event.id);
      rsvped = true;
    }
  }
</script>

<div class="event-card">
  <div class="event-header">
    <div class="event-status" style="background: {getStatusColor(event.status)}">
      {event.status.toUpperCase()}
    </div>
    <div class="event-meta">
      <span class="event-location">
        {event.location_type === 'voice' ? '🔊' : '📍'}
        {event.location_name ?? event.location_type}
      </span>
    </div>
  </div>

  <div class="event-body">
    <h3 class="event-title">{event.title}</h3>
    {#if event.description}
      <p class="event-desc">{event.description}</p>
    {/if}
    <div class="event-time">
      <span>🗓 {formatDate(event.starts_at)}</span>
      {#if event.ends_at}
        <span> → {formatDate(event.ends_at)}</span>
      {/if}
    </div>
    <div class="event-footer">
      <span class="rsvp-count">👥 {event.rsvp_count} interested</span>
      {#if event.status === 'scheduled' || event.status === 'active'}
        <button
          class="rsvp-btn"
          class:rsvped
          onclick={handleRsvp}
        >
          {rsvped ? '✓ Interested' : 'Interested'}
        </button>
      {/if}
      {#if canManage && event.status !== 'cancelled'}
        <button class="cancel-btn" onclick={() => eventStore.cancel(event.id)}>Cancel</button>
      {/if}
    </div>
  </div>
</div>

<style>
  .event-card {
    background: #2b2d31;
    border-radius: 8px;
    overflow: hidden;
    border: 1px solid #35373c;
  }
  .event-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    background: #232428;
  }
  .event-status {
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.5px;
    padding: 3px 8px;
    border-radius: 10px;
    color: white;
  }
  .event-meta { font-size: 12px; color: #b5bac1; }
  .event-body { padding: 16px; }
  .event-title { margin: 0 0 8px 0; font-size: 16px; font-weight: 700; color: #f2f3f5; }
  .event-desc { margin: 0 0 12px 0; font-size: 13px; color: #b5bac1; line-height: 1.4; }
  .event-time { font-size: 13px; color: #dbdee1; margin-bottom: 12px; }
  .event-footer { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .rsvp-count { font-size: 13px; color: #b5bac1; flex: 1; }
  .rsvp-btn {
    background: #5865f2;
    border: none;
    border-radius: 4px;
    color: white;
    padding: 6px 16px;
    cursor: pointer;
    font-size: 13px;
    font-weight: 600;
  }
  .rsvp-btn:hover { background: #4752c4; }
  .rsvp-btn.rsvped { background: #35373c; color: #b5bac1; }
  .rsvp-btn.rsvped:hover { background: #ed4245; color: white; }
  .cancel-btn {
    background: none;
    border: 1px solid #ed4245;
    border-radius: 4px;
    color: #ed4245;
    padding: 5px 12px;
    cursor: pointer;
    font-size: 12px;
  }
  .cancel-btn:hover { background: #ed4245; color: white; }
</style>
