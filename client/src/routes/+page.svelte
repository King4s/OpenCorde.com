<script lang="ts">
  /**
   * @file Home page — redirects based on auth state
   * @purpose Entry point, restores session then redirects
   */
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { restoreSession, isAuthenticated } from '$lib/stores/auth';

  onMount(() => {
    restoreSession().then(() => {
      const unsub = isAuthenticated.subscribe((v) => {
        if (v) {
          goto('/servers');
        } else {
          goto('/login');
        }
      });
      return unsub;
    });
  });
</script>

<div class="flex items-center justify-center h-screen bg-gray-900">
  <div class="text-center">
    <h1 class="text-3xl font-bold text-white mb-2">OpenCorde</h1>
    <p class="text-gray-400">Loading...</p>
  </div>
</div>
