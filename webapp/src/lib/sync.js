// Sync engine — primary API is webapp.questline.com (gibranlp_webappquest DB).
// Push events go to both webapp API and questlinecli.com to keep the CLI in sync.
// E2EE: payloads are AES-GCM encrypted before transmission.
// IndexedDB: decrypted state is cached locally for instant boot.

import { get } from 'svelte/store';
import {
  applySyncEvent, syncStatus, addToast,
  projects, tasks, notes, codices, journalEntries,
  milestones, achievements, rituals, focusSessions,
  loreUnlocks, userStats, zenTree, streaks,
  dataKey,
} from './store.js';
import { encryptPayload, decryptPayload } from './crypto.js';
import { saveEntity, deleteEntity, loadAllEntities } from './db.js';
import { ApiClient, QUESTLINE_API_BASE, pullAllFromQuestline } from './api.js';

const SEQ_KEY = 'questline_sync_seq';

export function getLastSeq() {
  return parseInt(localStorage.getItem(SEQ_KEY) || '0', 10);
}

function setLastSeq(seq) {
  localStorage.setItem(SEQ_KEY, String(seq));
}

// ── Boot: populate Svelte stores from IndexedDB (no network needed) ─────────

export async function loadLocalCache() {
  const entityStoreMap = [
    ['projects',       projects],
    ['tasks',          tasks],
    ['notes',          notes],
    ['codices',        codices],
    ['journal_entries', journalEntries],
    ['milestones',     milestones],
    ['achievements',   achievements],
    ['rituals',        rituals],
    ['focus_sessions', focusSessions],
    ['lore_unlocks',   loreUnlocks],
  ];

  for (const [storeName, svStore] of entityStoreMap) {
    try {
      const rows = await loadAllEntities(storeName);
      svStore.set(new Map(rows.map(r => [r.id, r])));
    } catch {
      // non-fatal — store stays empty
    }
  }

  // Singletons — stored with id='singleton', unwrap before setting
  const singletons = [
    ['user_stats', userStats],
    ['zen_tree',   zenTree],
    ['streaks',    streaks],
  ];
  for (const [storeName, svStore] of singletons) {
    try {
      const rows = await loadAllEntities(storeName);
      if (rows.length > 0) {
        const { id: _id, ...payload } = rows[0];
        svStore.set(payload);
      }
    } catch {
      // non-fatal
    }
  }
}

// ── Internal: decrypt + apply event to stores + persist to IndexedDB ────────

const ENTITY_STORE_NAME = {
  project:       'projects',
  task:          'tasks',
  note:          'notes',
  codex:         'codices',
  journal_entry: 'journal_entries',
  milestone:     'milestones',
  achievement:   'achievements',
  ritual:        'rituals',
  focus_session: 'focus_sessions',
  lore_unlock:   'lore_unlocks',
};

async function applyAndCacheEvent(event) {
  const { entity_type, entity_id, operation, content } = event;
  const key = get(dataKey);

  // Decrypt content when a key is available; fall back to raw if not encrypted
  let plainContent = content;
  if (key && content) {
    plainContent = await decryptPayload(content, key);
  }

  // Apply to Svelte stores (store.js handles all entity_type routing)
  applySyncEvent({ ...event, content: plainContent });

  // Persist decrypted entity to IndexedDB
  const payload = plainContent ? JSON.parse(plainContent) : null;

  if ((entity_type === 'user' || entity_type === 'user_stats') && payload) {
    try { await saveEntity('user_stats', { id: 'singleton', ...payload }); } catch {}
    return;
  }
  if (entity_type === 'zen_tree' && payload) {
    try { await saveEntity('zen_tree', { id: 'singleton', ...payload }); } catch {}
    return;
  }
  if (entity_type === 'streaks' && payload) {
    try { await saveEntity('streaks', { id: 'singleton', ...payload }); } catch {}
    return;
  }

  const storeName = ENTITY_STORE_NAME[entity_type];
  if (!storeName || !entity_id) return;

  try {
    if (operation === 'delete') {
      await deleteEntity(storeName, entity_id);
    } else if (payload) {
      await saveEntity(storeName, { ...payload, id: entity_id });
    }
  } catch (err) {
    console.warn(`[db] cache write failed for ${entity_type}:`, err);
  }
}

// ── Pull ─────────────────────────────────────────────────────────────────────

export async function pullSync(api, onProgress = null) {
  syncStatus.set('syncing');
  let seq = getLastSeq();

  // Only reset to seq=0 if we have no local data at all (true first sync)
  if (get(projects).size === 0) seq = 0;

  let totalPulled = 0;
  let hasMore = true;

  try {
    while (hasMore) {
      const events = await api.request('POST', 'sync/pull', null, { since_seq: seq });
      if (!Array.isArray(events) || events.length === 0) { hasMore = false; break; }

      for (const event of events) {
        await applyAndCacheEvent(event);
        if (event.seq > seq) seq = event.seq;
      }

      totalPulled += events.length;
      if (onProgress) onProgress(totalPulled);
      if (events.length < 500) hasMore = false;
    }

    setLastSeq(seq);
    syncStatus.set('idle');
    return totalPulled;
  } catch (err) {
    syncStatus.set('error');
    console.error('[sync] pull failed:', err);
    throw err;
  }
}

// ── Push ─────────────────────────────────────────────────────────────────────
// Dual-push: writes to the webapp DB (gibranlp_webappquest) AND questlinecli.com
// so the CLI picks up webapp changes on its next pull.

export async function pushEvent(api, entityType, entityId, operation, payload) {
  const key = get(dataKey);

  // Optimistic local save
  const storeName = ENTITY_STORE_NAME[entityType];
  if (storeName && entityId) {
    try {
      if (operation === 'delete') {
        await deleteEntity(storeName, entityId);
      } else if (payload) {
        await saveEntity(storeName, { ...payload, id: entityId });
      }
    } catch {}
  }

  // Encrypt payload before transmitting
  let contentString = payload ? JSON.stringify(payload) : null;
  if (key && contentString) {
    try {
      contentString = await encryptPayload(contentString, key);
    } catch (err) {
      console.warn('[sync] encryption failed, sending unencrypted:', err);
    }
  }

  const event = {
    id:          crypto.randomUUID(),
    entity_type: entityType,
    entity_id:   entityId,
    operation,
    content:     contentString,
    timestamp:   new Date().toISOString(),
  };

  // Push to primary webapp API
  try {
    await api.post('sync/push', [event]);
  } catch (err) {
    console.error('[sync] webapp push failed:', err);
    addToast('Sync failed — changes saved locally', 'warning');
    return;
  }

  // Dual-push to questlinecli.com so the CLI stays in sync
  try {
    const questlineClient = new ApiClient(api.identity, QUESTLINE_API_BASE);
    await questlineClient.post('sync/push', [event]);
  } catch {
    // Non-fatal — webapp DB is source of truth; CLI will catch up via its own sync
  }
}

// ── Import: pull all events from questlinecli.com → store in webapp DB ────────
//
// Flow:
//   1. Pull all sync events from questlinecli.com (seq=0, batches of 500)
//   2. POST each batch to the webapp API (webapp/import) → gibranlp_webappquest
//   3. Apply events locally (IndexedDB + Svelte stores) for instant UI
//   4. Register a webhook on questlinecli.com so future CLI pushes propagate here
//
// onProgress(count) is called after each batch with the running total.

export async function importFromQuestline(webappApi, identity, onProgress = null) {
  syncStatus.set('syncing');
  let total = 0;

  try {
    for await (const batch of pullAllFromQuestline(identity)) {
      // Send batch to webapp backend (stores in gibranlp_webappquest)
      await webappApi.post('webapp/import', batch);

      // Apply locally
      for (const event of batch) {
        await applyAndCacheEvent(event);
      }

      total += batch.length;
      if (onProgress) onProgress(total);
    }

    // Register a webhook so future CLI pushes replicate here automatically
    await registerWebhookOnQuestline(webappApi, identity);

    syncStatus.set('idle');
    return total;
  } catch (err) {
    syncStatus.set('error');
    console.error('[sync] import failed:', err);
    throw err;
  }
}

async function registerWebhookOnQuestline(webappApi, identity) {
  try {
    const secret = crypto.randomUUID().replace(/-/g, '');

    // Store secret on webapp backend so it can verify incoming webhook events
    await webappApi.post('webhook/setup', { secret });

    // Register the webhook on questlinecli.com (requires questlinecli.com auth)
    const questlineClient = new ApiClient(identity, QUESTLINE_API_BASE);
    const webappApiUrl = import.meta.env.VITE_API_URL || 'https://webapp.questline.com/api/';
    await questlineClient.post('webhooks/register', {
      url:    webappApiUrl + '?route=webhook/ingest',
      events: '*',
      secret,
    });
  } catch (err) {
    // Non-fatal — import succeeded, webhook registration failed (can retry later)
    console.warn('[sync] webhook registration failed:', err);
  }
}

// ── Catchup pull: fetch new events from questlinecli.com since last known seq ─
// Stores them in the webapp DB via webapp/import (INSERT IGNORE = safe to repeat).

export async function catchupFromQuestline(webappApi, identity, onProgress = null) {
  syncStatus.set('syncing');
  let total = 0;
  const seq = getLastSeq();

  try {
    const questlineClient = new ApiClient(identity, QUESTLINE_API_BASE);
    let cursor = seq;
    let hasMore = true;

    while (hasMore) {
      const events = await questlineClient.request('POST', 'sync/pull', null, { since_seq: cursor });
      if (!Array.isArray(events) || events.length === 0) break;

      await webappApi.post('webapp/import', events);
      for (const event of events) {
        await applyAndCacheEvent(event);
        if (event.seq > cursor) cursor = event.seq;
      }

      total += events.length;
      if (onProgress) onProgress(total);
      if (events.length < 500) hasMore = false;
    }

    if (cursor > seq) setLastSeq(cursor);
    syncStatus.set('idle');
    return total;
  } catch (err) {
    syncStatus.set('error');
    console.error('[sync] catchup failed:', err);
    throw err;
  }
}

// ── Background polling ────────────────────────────────────────────────────────
// Pulls from the webapp DB every 30s — picks up webhook-delivered CLI events
// and any changes made in other browser tabs.
// CLI catchup (catchupFromQuestline) is manual-only via Settings.

export function startBackgroundSync(api) {
  let pullTimer = null;

  const poll = async () => {
    try { await pullSync(api); } catch { /* retry next tick */ }
  };

  const start = () => {
    poll();
    pullTimer = setInterval(poll, 30_000);
  };

  const stop = () => {
    if (pullTimer) { clearInterval(pullTimer); pullTimer = null; }
  };

  const handleVisibility = () => {
    if (document.hidden) { stop(); } else { start(); }
  };

  document.addEventListener('visibilitychange', handleVisibility);
  start();

  return () => {
    stop();
    document.removeEventListener('visibilitychange', handleVisibility);
  };
}
