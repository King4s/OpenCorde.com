<script lang="ts">
  /**
   * @file ToastContainer.svelte — Global toast renderer
   * @purpose Render ephemeral toasts stacked at bottom-right
   * @depends toasts store
   */
  import { getToasts, removeToast, type ToastType } from '$lib/stores/toasts.svelte';

  const toasts = getToasts();

  function typeClasses(type: ToastType): string {
    switch (type) {
      case 'error':
        return 'border-red-700/60 bg-red-950/90 text-red-100';
      case 'warning':
        return 'border-yellow-700/60 bg-yellow-950/90 text-yellow-100';
      case 'success':
        return 'border-green-700/60 bg-green-950/90 text-green-100';
      case 'info':
      default:
        return 'border-gray-600/60 bg-gray-900/95 text-gray-100';
    }
  }

  function iconFor(type: ToastType): string {
    switch (type) {
      case 'error':
        return '⚠';
      case 'warning':
        return '⚠';
      case 'success':
        return '✓';
      case 'info':
      default:
        return 'ℹ';
    }
  }
</script>

{#if toasts.length > 0}
  <div class="fixed bottom-4 right-4 z-[70] flex flex-col gap-2 w-[min(92vw,24rem)] pointer-events-none">
    {#each toasts as toast (toast.id)}
      <div
        class="pointer-events-auto rounded-xl border shadow-2xl backdrop-blur px-4 py-3 flex items-start gap-3 transition-all {typeClasses(toast.type)}"
        role="alert"
      >
        <span class="mt-0.5 text-sm shrink-0">{iconFor(toast.type)}</span>
        <div class="min-w-0 flex-1">
          {#if toast.title}
            <p class="text-xs font-semibold uppercase tracking-wider opacity-70">{toast.title}</p>
          {/if}
          <p class="text-sm leading-snug">{toast.message}</p>
        </div>
        <button
          onclick={() => removeToast(toast.id)}
          class="text-xs opacity-60 hover:opacity-100 shrink-0 mt-0.5"
          aria-label="Dismiss"
        >
          &#x2715;
        </button>
      </div>
    {/each}
  </div>
{/if}
