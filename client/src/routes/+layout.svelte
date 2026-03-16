<script lang="ts">
  /**
   * @file Root layout — wraps all pages
   * @purpose Global styles, auth check, navigation structure
   */
  import '../app.css';

  let { children } = $props();
</script>

{@render children()}
