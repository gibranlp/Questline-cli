<script>
  import { identity, addToast } from '../lib/store.js';
  import { importFromQuestline, catchupFromQuestline } from '../lib/sync.js';
  import { ApiClient, QUESTLINE_API_BASE } from '../lib/api.js';

  export let api;

  $: apiUrl = api?.base ?? 'not connected';

  // ── Initial import ────────────────────────────────────────────────────────
  let importing    = false;
  let importDone   = false;
  let importCount  = 0;
  let importError  = '';
  let progress     = 0;

  async function startImport() {
    importing    = true;
    importDone   = false;
    importError  = '';
    importCount  = 0;
    progress     = 0;

    try {
      const total = await importFromQuestline(api, $identity, (count) => {
        importCount = count;
        progress    = Math.min(count / 10, 95);
      });
      importCount = total;
      progress    = 100;
      importDone  = true;
      addToast(`Import complete — ${total} events loaded`, 'info');
    } catch (err) {
      importError = `${err.message || 'Import failed'} (API: ${api?.base})`;
    } finally {
      importing = false;
    }
  }

  // ── Manual sync from CLI ──────────────────────────────────────────────────
  let syncing      = false;
  let syncCount    = 0;
  let syncError    = '';

  async function pullFromCLI() {
    syncing   = true;
    syncError = '';
    syncCount = 0;

    try {
      const total = await catchupFromQuestline(api, $identity, (n) => { syncCount = n; });
      syncCount = total;
      if (total === 0) {
        addToast('Already up to date', 'info');
      } else {
        addToast(`Pulled ${total} new event${total === 1 ? '' : 's'} from CLI`, 'info');
      }
    } catch (err) {
      syncError = err.message || 'Sync failed';
    } finally {
      syncing = false;
    }
  }
</script>

<div class="settings-page">
  <header class="page-header">
    <h1>Settings</h1>
  </header>

  <!-- ── Import section ──────────────────────────────────────────────── -->
  <section class="card">
    <h2 class="section-title">Import from Questline CLI</h2>
    <p class="section-desc">
      Pull your complete CLI history into this webapp's database. After import, the
      webapp reads from its own fast database and stays in sync with the CLI automatically
      via webhook.
    </p>
    <p class="api-label">API endpoint: <code>{apiUrl}</code></p>

    {#if importDone}
      <div class="status-box success">
        Import complete — <strong>{importCount.toLocaleString()} events</strong> loaded.
        Your data is now available and syncing.
      </div>

    {:else if importing}
      <div class="status-box syncing">
        <div class="spinner"></div>
        <span>Importing… <strong>{importCount.toLocaleString()}</strong> events so far</span>
      </div>
      <div class="progress-bar">
        <div class="progress-fill" style="width: {progress}%"></div>
      </div>

    {:else}
      {#if importError}
        <div class="status-box error">{importError}</div>
      {/if}
      <button class="btn-import" on:click={startImport}>
        Import CLI Data
      </button>
      <p class="hint">
        This fetches all your sync events from questlinecli.com and stores them locally.
        Safe to run multiple times — duplicates are ignored.
      </p>
    {/if}
  </section>

  <!-- ── Manual sync from CLI ───────────────────────────────────────────── -->
  <section class="card">
    <h2 class="section-title">Sync from CLI</h2>
    <p class="section-desc">
      Pull any CLI events that missed the webhook — useful after creating tasks
      in the CLI that haven't appeared here yet.
    </p>

    {#if syncError}
      <div class="status-box error">{syncError}</div>
    {/if}

    {#if syncing}
      <div class="status-box syncing">
        <div class="spinner"></div>
        <span>Pulling… <strong>{syncCount}</strong> new events</span>
      </div>
    {:else}
      <button class="btn-sync" on:click={pullFromCLI}>
        Pull from CLI
      </button>
      <p class="hint">Safe to run anytime — duplicates are ignored.</p>
    {/if}
  </section>
</div>

<style>
  .settings-page {
    padding: 2rem 2.5rem;
    max-width: 680px;
    font-family: 'JetBrains Mono', monospace;
  }

  .page-header { margin-bottom: 2rem; }

  h1 {
    font-size: 1.1rem;
    font-weight: 700;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: #a855f7;
  }

  .card {
    background: rgba(255,255,255,0.03);
    border: 1px solid #1c1c1c;
    border-radius: 10px;
    padding: 1.75rem 2rem;
    margin-bottom: 1.5rem;
  }

  .section-title {
    font-size: 0.85rem;
    font-weight: 700;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    color: #d4d4d4;
    margin-bottom: 0.75rem;
  }

  .section-desc {
    font-size: 0.8rem;
    color: #666;
    line-height: 1.7;
    margin-bottom: 1.5rem;
  }

  .btn-import {
    background: rgba(168,85,247,0.12);
    border: 1px solid #a855f7;
    color: #a855f7;
    font-family: inherit;
    font-size: 0.82rem;
    font-weight: 700;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    padding: 0.65rem 1.4rem;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.15s, box-shadow 0.15s;
    margin-bottom: 0.75rem;
  }

  .btn-import:hover {
    background: rgba(168,85,247,0.24);
    box-shadow: 0 0 16px rgba(168,85,247,0.2);
  }

  .hint {
    font-size: 0.72rem;
    color: #444;
    line-height: 1.6;
  }

  .btn-sync {
    background: rgba(6,182,212,0.08);
    border: 1px solid #06b6d4;
    color: #06b6d4;
    font-family: inherit;
    font-size: 0.82rem;
    font-weight: 700;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    padding: 0.65rem 1.4rem;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.15s;
    margin-bottom: 0.75rem;
  }

  .btn-sync:hover {
    background: rgba(6,182,212,0.18);
  }

  .api-label {
    font-size: 0.72rem;
    color: #444;
    margin-bottom: 1.25rem;
  }

  .api-label code {
    color: #666;
    background: rgba(255,255,255,0.04);
    padding: 0.1em 0.35em;
    border-radius: 3px;
  }

  .status-box {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.8rem 1rem;
    border-radius: 6px;
    font-size: 0.8rem;
    margin-bottom: 1rem;
  }

  .status-box.success {
    background: rgba(34,197,94,0.08);
    border: 1px solid #22c55e;
    color: #4ade80;
  }

  .status-box.syncing {
    background: rgba(168,85,247,0.08);
    border: 1px solid #a855f7;
    color: #c084fc;
  }

  .status-box.error {
    background: rgba(239,68,68,0.08);
    border: 1px solid #ef4444;
    color: #f87171;
    margin-bottom: 1rem;
  }

  .progress-bar {
    height: 3px;
    background: #1c1c1c;
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #a855f7, #06b6d4);
    transition: width 0.3s ease;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(168,85,247,0.3);
    border-top-color: #a855f7;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    flex-shrink: 0;
  }

  @keyframes spin { to { transform: rotate(360deg); } }
</style>
