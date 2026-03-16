<script lang="ts">
  /**
   * @file Home page — redirects to login
   * @purpose Entry point redirect
   */
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  onMount(() => {
    goto('/login');
  });
</script>

<div class="flex items-center justify-center h-screen bg-gray-900">
  <div class="text-center">
    <h1 class="text-3xl font-bold text-white mb-2">OpenCorde</h1>
    <p class="text-gray-400">Redirecting...</p>
  </div>
</div>
