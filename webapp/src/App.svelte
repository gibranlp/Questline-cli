<script>
  import { onMount } from 'svelte';
  import { route, navigate } from './lib/router.js';
  import { loadIdentity, clearIdentity, loadKeyHexFromSession, clearSessionKey } from './lib/auth.js';
  import { importKeyHex } from './lib/crypto.js';
  import { ApiClient } from './lib/api.js';
  import { identity as identityStore, apiClient as apiStore, dataKey as dataKeyStore, addToast, userStats } from './lib/store.js';
  import { startBackgroundSync, loadLocalCache } from './lib/sync.js';
  import { clearLocalDatabase } from './lib/db.js';

  import Nav from './components/Nav.svelte';
  import Toast from './components/Toast.svelte';

  import Register from './routes/Register.svelte';
  import Login from './routes/Login.svelte';
  import Dashboard from './routes/Dashboard.svelte';
  import Projects from './routes/Projects.svelte';
  import Workspace from './routes/Workspace.svelte';
  import Fellowship from './routes/Fellowship.svelte';
  import Character from './routes/Character.svelte';
  import Focus from './routes/Focus.svelte';
  import Chronicles from './routes/Chronicles.svelte';
  import Legends from './routes/Legends.svelte';
  import Library from './routes/Library.svelte';
  import Settings from './routes/Settings.svelte';

  let ready = false;
  let api = null;
  let stopSync = null;
  let bootStatus = '';

  // Initialize API whenever identity appears — covers both page-load and post-login navigation
  $: if ($identityStore && !api) {
    api = new ApiClient($identityStore);
    apiStore.set(api);
    startSession(api);
  }

  // Tear down when identity is cleared (logout or revocation)
  $: if (!$identityStore && api) {
    if (stopSync) { stopSync(); stopSync = null; }
    api = null;
    apiStore.set(null);
    clearLocalDatabase().catch(() => {});
  }

  async function startSession(client) {
    try {
      const status = await client.get('webapp/supporter-status');
      if (!status.supporter) {
        addToast('Access revoked. Please contact support.', 'error');
        clearIdentity();
        identityStore.set(null);
        navigate('/login');
        return;
      }
      // Redirect to settings/import if no data has been imported yet
      if (status.needs_import) {
        navigate('/settings');
        if (stopSync) stopSync();
        stopSync = startBackgroundSync(client);
        return;
      }
    } catch {
      // allow offline use
    }

    if ($route === '/' || $route === '/login' || $route === '/register') {
      navigate('/dashboard');
    }

    // Load local cache for fresh logins (page-reload already did this in onMount)
    await loadLocalCache();

    if (stopSync) stopSync();
    stopSync = startBackgroundSync(client);
  }

  onMount(async () => {
    const id = loadIdentity();
    if (id) {
      if (window.location.pathname === '/') navigate('/dashboard');
      // Restore the data encryption key from sessionStorage (survives page refreshes)
      const keyHex = loadKeyHexFromSession();
      if (keyHex) {
        try {
          const key = await importKeyHex(keyHex);
          dataKeyStore.set(key);
        } catch {
          clearSessionKey(); // corrupted entry — discard
        }
      }

      // Populate stores from local IndexedDB before hitting the network
      bootStatus = 'Reading local records...';
      await loadLocalCache();
      bootStatus = 'Syncing with the Realm...';

      identityStore.set(id); // triggers reactive API/session init above
    } else {
      navigate('/login');
    }
    ready = true;
  });

  // Normalize class name → slug.
  // Handles both serde variant names ("CodeWarlock") and display names ("Code Warlock").
  const CLASS_BG = {
    'codewarlock':      'warlock',
    'taskpaladin':      'paladin',
    'mindsage':         'mindsage',
    'systemsarchitect': 'systemarchitect',
    'timechronomancer': 'timechronomancer',
    'archaccountant':   'archaccountant',
  };

  // Per-class accent colors — used across the whole UI for glows, borders, highlights
  const CLASS_COLORS = {
    warlock:          { accent: '#a855f7', accentDim: '#7c3aed', glow: 'rgba(168,85,247,0.25)' },
    paladin:          { accent: '#f59e0b', accentDim: '#b45309', glow: 'rgba(245,158,11,0.25)'  },
    mindsage:         { accent: '#06b6d4', accentDim: '#0e7490', glow: 'rgba(6,182,212,0.25)'   },
    systemarchitect:  { accent: '#3b82f6', accentDim: '#1d4ed8', glow: 'rgba(59,130,246,0.25)'  },
    timechronomancer: { accent: '#10b981', accentDim: '#047857', glow: 'rgba(16,185,129,0.25)'  },
    archaccountant:   { accent: '#d97706', accentDim: '#92400e', glow: 'rgba(217,119,6,0.25)'   },
  };

  // UI asset filenames — same names expected inside each class folder
  const UI_ASSETS = [
    'gf-tlc', 'gf-trc', 'gf-blc', 'gf-brc',
    'gf-et',  'gf-eb',  'gf-el',  'gf-er',
    'gf-gemt', 'gf-gemb',
    'i-n', 'i-a', 'i-e',
    'gc-n', 'gc-h', 'gc-p',
  ];

  $: classSlug = $userStats?.class
    ? (CLASS_BG[$userStats.class.toLowerCase().replace(/\s+/g, '')] ?? null)
    : null;

  $: {
    if (classSlug) {
      // Backgrounds
      document.documentElement.style.setProperty('--menu-bg', `url('/assets/backgrounds/${classSlug}-menu-background.png')`);
      document.documentElement.style.setProperty('--main-bg',  `url('/assets/backgrounds/${classSlug}-background.png')`);

      // UI asset paths — components use var(--ui-gf-tlc) etc. with fallback to default
      const uiBase = `/assets/ui/${classSlug}`;
      for (const asset of UI_ASSETS) {
        document.documentElement.style.setProperty(`--ui-${asset}`, `url('${uiBase}/${asset}.webp')`);
      }

      // Accent colors
      const colors = CLASS_COLORS[classSlug];
      document.documentElement.style.setProperty('--accent',     colors.accent);
      document.documentElement.style.setProperty('--accent-dim', colors.accentDim);
      document.documentElement.style.setProperty('--accent-glow',colors.glow);
    } else {
      document.documentElement.style.removeProperty('--menu-bg');
      document.documentElement.style.removeProperty('--main-bg');
      for (const asset of UI_ASSETS) {
        document.documentElement.style.removeProperty(`--ui-${asset}`);
      }
      document.documentElement.style.removeProperty('--accent');
      document.documentElement.style.removeProperty('--accent-dim');
      document.documentElement.style.removeProperty('--accent-glow');
    }
  }

  // Parse route into segments
  $: segments = $route.split('/').filter(Boolean);
  $: currentRoute = '/' + (segments[0] ?? '');
  $: routeParam = segments[1] ?? null;

  $: showNav = $identityStore !== null && currentRoute !== '/register' && currentRoute !== '/login';
</script>

{#if ready}
  {#if showNav}
    <Nav />
  {/if}

  <main class="main-content" class:with-nav={showNav}>
    {#if currentRoute === '/login' || (!$identityStore && currentRoute !== '/register')}
      <Login />
    {:else if currentRoute === '/register'}
      <Register />
    {:else if currentRoute === '/dashboard'}
      <Dashboard {api} />
    {:else if currentRoute === '/projects' && !routeParam}
      <Projects {api} />
    {:else if currentRoute === '/projects' && routeParam}
      <Workspace projectId={routeParam} {api} />
    {:else if currentRoute === '/fellowship'}
      <Fellowship {api} />
    {:else if currentRoute === '/character'}
      <Character />
    {:else if currentRoute === '/focus'}
      <Focus {api} />
    {:else if currentRoute === '/chronicles'}
      <Chronicles />
    {:else if currentRoute === '/library'}
      <Library />
    {:else if currentRoute === '/legends'}
      <Legends />
    {:else if currentRoute === '/settings'}
      <Settings {api} />
    {:else}
      <div class="not-found">
        <p>404 — The path doesn't exist.</p>
        <button on:click={() => navigate('/dashboard')}>← Return to Dashboard</button>
      </div>
    {/if}
  </main>

  <Toast />
{:else}
  <div class="boot-screen">
    <div class="boot-spinner"></div>
    {#if bootStatus}
      <p class="boot-status">{bootStatus}</p>
    {/if}
  </div>
{/if}

<style>
  :global(*, *::before, *::after) { box-sizing: border-box; margin: 0; padding: 0; }

  :global(body) {
    background: #080808;
    color: #d4d4d4;
    font-family: 'Pixelify Sans', sans-serif;
    line-height: 1.6;
    overflow-x: hidden;
  }

  /* Scanline overlay — matches server/styles.css */
  :global(body::before) {
    content: '';
    pointer-events: none;
    position: fixed;
    inset: 0;
    background: repeating-linear-gradient(
      0deg, transparent, transparent 2px,
      rgba(0,0,0,0.06) 2px, rgba(0,0,0,0.06) 4px
    );
    z-index: 9999;
  }

  .main-content {
    min-height: 100vh;
    transition: padding-left 0.2s;
    background-image: var(--main-bg);
    background-attachment: fixed;
    background-position: center;
    background-size: cover;
  }

  .main-content.with-nav {
    padding-left: 300px;
  }

  .boot-screen {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1.25rem;
    min-height: 100vh;
  }

  .boot-spinner {
    width: 40px;
    height: 40px;
    border: 2px solid #1c1c1c;
    border-top-color: #a855f7;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .boot-status {
    font-size: 0.75rem;
    color: #555;
    letter-spacing: 0.08em;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  .not-found {
    padding: 4rem;
    text-align: center;
    color: #555;
    font-size: 0.9rem;
  }

  .not-found button {
    background: none;
    border: none;
    color: #a855f7;
    font-family: inherit;
    font-size: 0.85rem;
    cursor: pointer;
    margin-top: 1rem;
  }
</style>
