<script lang="ts">
  /**
   * @file Root layout — global auth guard and page wrapper
   * @purpose Redirects unauthenticated users to /login (except public routes)
   */
  import '../app.css';
  import { browser } from '$app/environment';
  import ServerSetup from '$lib/components/modals/ServerSetup.svelte';

  const PUBLIC = ['/', '/login', '/register', '/reset-password', '/invite'];

  // Show server setup if running in Tauri desktop and no server URL configured yet
  const needsServerSetup = browser
    && typeof (window as any).__TAURI_INTERNALS__ !== 'undefined'
    && !localStorage.getItem('opencorde_server');

  if (browser && !needsServerSetup) {
    const isPublic = PUBLIC.some(r => window.location.pathname.startsWith(r));
    if (!isPublic && !localStorage.getItem('opencorde_token')) {
      window.location.href = '/login';
    }
  }

  let { children } = $props();
</script>

{#if needsServerSetup}
  <ServerSetup />
{:else}
  {@render children()}
{/if}
