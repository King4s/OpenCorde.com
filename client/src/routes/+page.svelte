<script lang="ts">
  /**
   * @file Home page — redirects to login or server list
   * @purpose Entry point, auth-gated redirect
   */
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { isAuthenticated } from '$lib/stores/auth';

  onMount(() => {
    const unsub = isAuthenticated.subscribe((v) => {
      if (v) {
        goto('/servers');
      } else {
        goto('/login');
      }
    });
    return unsub;
  });
</script>

<svelte:head>
  <noscript>
    <meta http-equiv="refresh" content="0;url=/login" />
  </noscript>
</svelte:head>

<div class="flex items-center justify-center h-screen bg-gray-900">
  <div class="text-center">
    <h1 class="text-3xl font-bold text-white mb-2">OpenCorde</h1>
    <p class="text-gray-400">Redirecting...</p>
  </div>
</div>
