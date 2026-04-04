<script lang="ts">
  /**
   * @file Root layout — global auth guard and page wrapper
   * @purpose Redirects unauthenticated users to /login (except public routes)
   */
  import '../app.css';
  import { browser } from '$app/environment';
  import { onMount } from 'svelte';
  import ServerSetup from '$lib/components/modals/ServerSetup.svelte';
  import { registerServiceWorker } from '$lib/sw';

  const PUBLIC = ['/', '/login', '/register', '/reset-password', '/invite'];
  const INSTALL_DISMISSED_KEY = 'opencorde_install_dismissed';

  type BeforeInstallPromptEvent = Event & {
    prompt: () => Promise<void>;
    userChoice: Promise<{ outcome: 'accepted' | 'dismissed'; platform?: string }>;
  };

  // Show server setup if running in Tauri desktop and no server URL configured yet
  const needsServerSetup =
    browser && typeof (window as any).__TAURI_INTERNALS__ !== 'undefined' && !localStorage.getItem('opencorde_server');

  let { children } = $props();
  let installPrompt = $state<BeforeInstallPromptEvent | null>(null);
  let installVisible = $state(false);
  let installBusy = $state(false);
  let installDismissed = $state(false);

  onMount(() => {
    void registerServiceWorker();

    const isStandalone =
      window.matchMedia('(display-mode: standalone)').matches || (window.navigator as any).standalone === true;

    if (isStandalone) return;

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

    return () => {
      window.removeEventListener('beforeinstallprompt', handleBeforeInstallPrompt as EventListener);
      window.removeEventListener('appinstalled', handleInstalled);
    };
  });

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

  function dismissInstall() {
    installDismissed = true;
    installVisible = false;
    installPrompt = null;
    localStorage.setItem(INSTALL_DISMISSED_KEY, '1');
  }
</script>

{#if needsServerSetup}
  <ServerSetup />
{:else}
  {@render children()}
{/if}

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
            onclick={dismissInstall}
            class="px-3 py-1.5 rounded-lg bg-gray-800 hover:bg-gray-700 text-gray-300 text-sm font-medium transition-colors"
          >
            Not now
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
