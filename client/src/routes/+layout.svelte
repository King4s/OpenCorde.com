<script lang="ts">
  /**
   * @file Root layout — global auth guard, PWA lifecycle, and page wrapper
   * @purpose Redirects unauthenticated users to /login (except public routes),
   *          manages install prompt, SW update banners, and offline indicator.
   */
  import '../app.css';
  import { browser } from '$app/environment';
  import { onMount } from 'svelte';
  import ServerSetup from '$lib/components/modals/ServerSetup.svelte';
  import {
    registerServiceWorker,
    isUpdateAvailable,
    isOnline,
    subscribe,
    triggerUpdate,
    dismissUpdate,
  } from '$lib/sw';

  const PUBLIC = ['/', '/login', '/register', '/reset-password', '/invite'];
  const INSTALL_DISMISSED_KEY = 'opencorde_install_dismissed';

  type BeforeInstallPromptEvent = Event & {
    prompt: () => Promise<void>;
    userChoice: Promise<{ outcome: 'accepted' | 'dismissed'; platform?: string }>;
  };

  const needsServerSetup =
    browser && typeof (window as any).__TAURI_INTERNALS__ !== 'undefined' && !localStorage.getItem('opencorde_server');

  let { children } = $props();

  // ---- Install prompt state ----
  let installPrompt = $state<BeforeInstallPromptEvent | null>(null);
  let installVisible = $state(false);
  let installBusy = $state(false);
  let installDismissed = $state(false);

  // ---- PWA state (kept in sync with $lib/sw via subscribe) ----
  let swUpdateAvailable = $state(false);
  let swOnline = $state(true);
  let swUpdateDismissed = $state(false);

  let unsubscribeSw: (() => void) | null = null;

  onMount(() => {
    void registerServiceWorker();

    // Reactively sync SW state into Svelte runes
    unsubscribeSw = subscribe(() => {
      swUpdateAvailable = isUpdateAvailable();
      swOnline = isOnline();
    });

    // ---- Install prompt ----
    const isStandalone =
      window.matchMedia('(display-mode: standalone)').matches || (window.navigator as any).standalone === true;

    if (!isStandalone) {
      installDismissed = localStorage.getItem(INSTALL_DISMISSED_KEY) === '1';

      const handleBeforeInstallPrompt = (event: Event) => {
        event.preventDefault();
        installPrompt = event as BeforeInstallPromptEvent;
        installVisible = true;
      };

      const handleInstalled = () => {
        installPrompt = null;
        installVisible = false;
        installBusy = false;
        installDismissed = false;
        localStorage.removeItem(INSTALL_DISMISSED_KEY);
      };

      window.addEventListener('beforeinstallprompt', handleBeforeInstallPrompt as EventListener);
      window.addEventListener('appinstalled', handleInstalled);
    }

    return () => {
      unsubscribeSw?.();
    };
  });

  // ---- Install handlers ----
  async function installApp() {
    if (!installPrompt || installBusy) return;
    installBusy = true;
    try {
      await installPrompt.prompt();
      await installPrompt.userChoice;
    } finally {
      installPrompt = null;
      installVisible = false;
      installBusy = false;
    }
  }

  function dismissInstallPrompt() {
    installDismissed = true;
    installVisible = false;
    installPrompt = null;
    localStorage.setItem(INSTALL_DISMISSED_KEY, '1');
  }

  // ---- SW update handlers ----
  function handleUpdateNow() {
    triggerUpdate();
    window.location.reload();
  }

  function handleDismissUpdate() {
    swUpdateDismissed = true;
    dismissUpdate();
  }
</script>

{#if needsServerSetup}
  <ServerSetup />
{:else}
  {@render children()}
{/if}

<!-- ================================================================== -->
<!-- Offline banner                                                      -->
<!-- ================================================================== -->
{#if browser && !swOnline}
  <div class="fixed top-0 inset-x-0 z-[70] bg-amber-600/95 text-amber-50 text-center text-sm py-2 px-4 backdrop-blur">
    <span class="inline-flex items-center gap-1.5">
      <span class="text-base">⚡</span>
      You are offline — messages will send when you reconnect.
    </span>
  </div>
{/if}

<!-- ================================================================== -->
<!-- Update available banner                                             -->
<!-- ================================================================== -->
{#if browser && swUpdateAvailable && !swUpdateDismissed}
  <div class="fixed top-0 inset-x-0 z-[70] bg-indigo-600/95 text-indigo-50 text-center text-sm py-2 px-4 backdrop-blur">
    <span class="inline-flex items-center gap-2">
      <span class="text-base">🔄</span>
      A new version of OpenCorde is available.
      <button
        onclick={handleUpdateNow}
        class="ml-1 underline underline-offset-2 hover:text-white font-medium"
      >
        Update now
      </button>
      <button
        onclick={handleDismissUpdate}
        class="ml-2 text-indigo-200 hover:text-white"
        aria-label="Dismiss"
      >
        ✕
      </button>
    </span>
  </div>
{/if}

<!-- ================================================================== -->
<!-- Install prompt (PWA)                                                -->
<!-- ================================================================== -->
{#if browser && installVisible && !installDismissed && !needsServerSetup}
  <div class="fixed bottom-4 right-4 z-[60] w-[min(92vw,24rem)] rounded-2xl border border-gray-700 bg-gray-900/95 p-4 shadow-2xl backdrop-blur">
    <div class="flex items-start gap-3">
      <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-gray-600/20 text-gray-300 text-lg">
        ⤓
      </div>
      <div class="min-w-0 flex-1">
        <p class="text-xs font-semibold uppercase tracking-[0.18em] text-gray-500">Install OpenCorde</p>
        <p class="mt-1 text-sm text-gray-300">
          Install the app for a native feel, dock/taskbar presence, and faster access.
        </p>
        <div class="mt-3 flex flex-wrap gap-2">
          <button
            onclick={installApp}
            disabled={installBusy || !installPrompt}
            class="px-3 py-1.5 rounded-lg bg-gray-600 hover:bg-gray-500 disabled:opacity-50 text-white text-sm font-medium transition-colors"
          >
            {installBusy ? 'Opening…' : 'Install app'}
          </button>
          <button
            onclick={dismissInstallPrompt}
            class="px-3 py-1.5 rounded-lg bg-gray-800 hover:bg-gray-700 text-gray-300 text-sm font-medium transition-colors"
          >
            Not now
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
