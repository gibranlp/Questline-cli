<script>
  import { navigate } from '../lib/router.js';
  import { route } from '../lib/router.js';
  import { syncStatus, identity, sortedProjects } from '../lib/store.js';
  import { clearIdentity } from '../lib/auth.js';

  const links = [
    { path: '/dashboard',  label: 'Dashboard',        icon: '⚔️'  },
    // Campaigns is handled separately (expandable)
    { path: '/character',  label: 'Hero',             icon: '🧙'  },
    { path: '/library',    label: 'Library',          icon: '📚',  stub: false },
    { path: '/fellowship', label: 'Fellowship',        icon: '⚜️'  },
    { path: '/chronicles', label: 'Great Chronicle',  icon: '📜',  stub: true  },
    { path: '/settings',   label: 'Settings',         icon: '⚙️'  },
  ];

  let campaignsOpen = true;

  $: currentPath = $route;
  $: campaignsActive = currentPath.startsWith('/projects');

  function logout() {
    clearIdentity();
    window.location.reload();
  }
</script>

<nav class="sidebar">
  <div class="brand">
    <span class="brand-text">QUESTLINE</span>
    <span class="sync-dot" class:syncing={$syncStatus === 'syncing'} class:error={$syncStatus === 'error'}></span>
  </div>

  <ul>
    <!-- ── Dashboard ─────────────────────────────────────── -->
    <li>
      <button
        class="nav-link"
        class:active={currentPath === '/dashboard'}
        on:click={() => navigate('/dashboard')}
      >
        <span class="icon">⚔️</span>
        <span class="label">Dashboard</span>
      </button>
    </li>

    <!-- ── Campaigns (expandable) ─────────────────────────── -->
    <li>
      <button
        class="nav-link campaigns-toggle"
        class:active={campaignsActive}
        on:click={() => campaignsOpen = !campaignsOpen}
      >
        <span class="icon">🗺️</span>
        <span class="label">Campaigns</span>
        <span class="chevron" class:open={campaignsOpen}>›</span>
      </button>

      {#if campaignsOpen}
        <ul class="sub-list">
          {#each $sortedProjects as project (project.id)}
            <li>
              <button
                class="sub-link"
                class:active={currentPath === '/projects/' + project.id}
                on:click={() => navigate('/projects/' + project.id)}
              >
                <span class="sub-dot">·</span>
                {project.name}
              </button>
            </li>
          {/each}

          {#if $sortedProjects.length === 0}
            <li><span class="sub-empty">No campaigns yet</span></li>
          {/if}

          <li>
            <button class="sub-link all-link" on:click={() => navigate('/projects')}>
              <span class="sub-dot">+</span>
              All campaigns
            </button>
          </li>
        </ul>
      {/if}
    </li>

    <!-- ── Remaining links (Hero, Library, Fellowship, Chronicle, Settings) ── -->
    {#each links.slice(1) as link}
      <li>
        {#if link.stub}
          <span class="nav-link stub">
            <span class="icon">{link.icon}</span>
            <span class="label">{link.label}</span>
            <span class="soon">soon</span>
          </span>
        {:else}
          <button
            class="nav-link"
            class:active={currentPath.startsWith(link.path)}
            on:click={() => navigate(link.path)}
          >
            <span class="icon">{link.icon}</span>
            <span class="label">{link.label}</span>
          </button>
        {/if}
      </li>
    {/each}
  </ul>

  <div class="nav-footer">
    <div class="user-info">
      <span class="dim">{$identity?.user_uuid?.slice(0, 8) ?? ''}…</span>
    </div>
    <button class="logout-btn" on:click={logout}>Sign out</button>
  </div>
</nav>

<style>
  .sidebar {
    width: 300px;
    min-height: 100vh;
    background-color: #0a0a0a;
    background-image: var(--menu-bg);
    background-position: top center;
    background-size: cover;
    background-repeat: no-repeat;
    border-right: 1px solid #1c1c1c;
    display: flex;
    flex-direction: column;
    padding: 1.5rem 0;
    position: fixed;
    left: 0;
    top: 0;
    bottom: 0;
    z-index: 50;
    overflow-y: auto;
  }

  .brand {
    padding: 0 1.5rem 1.5rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    border-bottom: 1px solid #1c1c1c;
    margin-bottom: 1rem;
    flex-shrink: 0;
  }

  .brand-text {
    font-size: 0.9rem;
    font-weight: 700;
    letter-spacing: 0.25em;
    background: linear-gradient(90deg, #a855f7, #06b6d4);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .sync-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #22c55e;
    margin-left: auto;
    flex-shrink: 0;
  }
  .sync-dot.syncing { background: #f59e0b; animation: pulse 1s infinite; }
  .sync-dot.error   { background: #ef4444; }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50%       { opacity: 0.3; }
  }

  ul {
    list-style: none;
    flex: 1;
    padding: 0 0.75rem;
  }

  li { margin-bottom: 2px; }

  .nav-link {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6rem 0.75rem;
    background: none;
    border: none;
    border-radius: 6px;
    color: #888;
    font-family: inherit;
    font-size: 0.85rem;
    letter-spacing: 0.05em;
    cursor: pointer;
    text-align: left;
    transition: background 0.15s, color 0.15s;
  }

  .nav-link:hover {
    background: rgba(168, 85, 247, 0.1);
    color: #d4d4d4;
  }

  .nav-link.active {
    background: rgba(168, 85, 247, 0.15);
    color: #a855f7;
  }

  .campaigns-toggle { user-select: none; }

  .chevron {
    margin-left: auto;
    font-size: 1rem;
    color: #444;
    transition: transform 0.2s;
    display: inline-block;
    transform: rotate(0deg);
  }
  .chevron.open { transform: rotate(90deg); }

  .icon  { font-size: 1rem; width: 20px; text-align: center; }
  .label { text-transform: uppercase; flex: 1; }

  .stub {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6rem 0.75rem;
    border-radius: 6px;
    color: #383838;
    font-family: inherit;
    font-size: 0.85rem;
    letter-spacing: 0.05em;
    cursor: default;
    user-select: none;
  }

  .soon {
    margin-left: auto;
    font-size: 0.6rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #2a2a2a;
    border: 1px solid #2a2a2a;
    border-radius: 3px;
    padding: 0.1em 0.35em;
  }

  /* ── Sub-list (campaign items) ───────────────────────── */
  .sub-list {
    padding: 0 0 0.25rem 2.25rem;
    margin-top: 2px;
  }

  .sub-list li { margin-bottom: 1px; }

  .sub-link {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35rem 0.5rem;
    background: none;
    border: none;
    border-radius: 4px;
    color: #555;
    font-family: inherit;
    font-size: 0.78rem;
    letter-spacing: 0.03em;
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: background 0.12s, color 0.12s;
  }

  .sub-link:hover { background: rgba(168,85,247,0.08); color: #aaa; }
  .sub-link.active { color: #a855f7; }

  .all-link { color: #444; font-style: italic; }
  .all-link:hover { color: #888; }

  .sub-dot { color: #333; width: 10px; text-align: center; flex-shrink: 0; }

  .sub-empty {
    display: block;
    padding: 0.3rem 0.5rem;
    font-size: 0.75rem;
    color: #333;
    font-style: italic;
  }

  /* ── Footer ───────────────────────────────────────────── */
  .nav-footer {
    padding: 1rem 1.5rem 0;
    border-top: 1px solid #1c1c1c;
    margin-top: auto;
    flex-shrink: 0;
  }

  .user-info { margin-bottom: 0.5rem; }

  .dim {
    font-size: 0.7rem;
    color: #555;
    font-family: inherit;
  }

  .logout-btn {
    background: none;
    border: 1px solid #2a2a2a;
    color: #666;
    padding: 0.35rem 0.75rem;
    border-radius: 4px;
    font-family: inherit;
    font-size: 0.75rem;
    cursor: pointer;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    transition: border-color 0.15s, color 0.15s;
  }

  .logout-btn:hover {
    border-color: #ef4444;
    color: #ef4444;
  }
</style>
