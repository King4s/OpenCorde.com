<script lang="ts">
  /**
   * @file Home page — redirects to login or server list
   * @purpose Entry point, auth-gated redirect
   */
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  onMount(() => {
    // TODO: Check auth state, redirect accordingly
    goto('/login');
  });
</script>

<div class="flex items-center justify-center h-screen">
  <p class="text-gray-400">Loading...</p>
</div>
