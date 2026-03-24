<script lang="ts">
  /**
   * @file Root layout — global auth guard and page wrapper
   * @purpose Redirects unauthenticated users to /login (except public routes)
   */
  import '../app.css';
  import { browser } from '$app/environment';

  const PUBLIC = ['/', '/login', '/register', '/reset-password', '/invite'];

  if (browser) {
    const isPublic = PUBLIC.some(r => window.location.pathname.startsWith(r));
    if (!isPublic && !localStorage.getItem('opencorde_token')) {
      window.location.href = '/login';
    }
  }

  let { children } = $props();
</script>

{@render children()}
