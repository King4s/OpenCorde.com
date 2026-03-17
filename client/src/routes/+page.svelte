<script lang="ts">
  /**
   * @file Home page — redirect to /login
   * @purpose Simple redirect, no store dependencies
   */
  import { browser } from '$app/environment';

  if (browser) {
    const token = localStorage.getItem('opencorde_token');
    window.location.href = token ? '/servers' : '/login';
  }
</script>

<div class="flex items-center justify-center h-screen bg-gray-900">
  <div class="text-center">
    <h1 class="text-3xl font-bold text-white mb-2">OpenCorde</h1>
    <p class="text-gray-400">Loading...</p>
  </div>
</div>
