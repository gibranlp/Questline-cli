// Sync engine — primary API is webapp.questlinecli.com (gibranlp_webappquest DB).
// Push events go to both webapp API and questlinecli.com to keep the CLI in sync.
// E2EE: payloads are AES-GCM encrypted before transmission.
// IndexedDB: decrypted state is cached locally for instant boot.

import { get } from 'svelte/store';
import {
  applySyncEvent, syncStatus, addToast,
  projects, tasks, notes, codices, journalEntries,
  milestones, achievements, rituals, focusSessions,
  loreUnlocks, chronicleMessages, userStats, zenTree, streaks,
  taskAssignments, notifications, identity,
  dataKey,
} from './store.js';
import { encryptPayload, decryptPayload, signEvent, verifyEvent,
  importProjectKey, projectKeyFromHex } from './crypto.js';
import { saveEntity, deleteEntity, loadAllEntities,
  getProjectKeyByRouting, getActiveProjectKeyByProject } from './db.js';
import { saveProjectOutboxEvent, deleteProjectOutboxEvent, loadProjectOutboxEvents } from './db.js';
import { ApiClient, QUESTLINE_API_BASE, pullAllFromQuestline } from './api.js';
import { processKeyEnvelopesAndRevocations } from './fellowship.js';
import { assignmentNotification } from './collaboration.js';

const SEQ_KEY     = 'questline_sync_seq';  // webapp DB (gibranlp_webappquest) pull cursor
const CLI_SEQ_KEY = 'questline_cli_seq';   // questlinecli.com pull cursor — different DB, different seq space

export function getLastSeq() {
  return parseInt(localStorage.getItem(SEQ_KEY) || '0', 10);
}

function setLastSeq(seq) {
  localStorage.setItem(SEQ_KEY, String(seq));
}

function getLastCLISeq() {
  return parseInt(localStorage.getItem(CLI_SEQ_KEY) || '0', 10);
}

function setLastCLISeq(seq) {
  localStorage.setItem(CLI_SEQ_KEY, String(seq));
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
    ['task_assignments', taskAssignments],
    ['notifications', notifications],
  ];

  for (const [storeName, svStore] of entityStoreMap) {
    try {
      const rows = await loadAllEntities(storeName);
      svStore.set(new Map(rows.map(r => [r.id, r])));
    } catch {
      // non-fatal — store stays empty
    }
  }

  try {
    const rows = await loadAllEntities('chronicle_messages');
    const grouped = new Map();
    for (const row of rows) {
      if (!row.project_id) continue;
      const list = grouped.get(row.project_id) || [];
      list.push(row);
      grouped.set(row.project_id, list);
    }
    for (const list of grouped.values()) {
      list.sort((a, b) => String(a.timestamp || '').localeCompare(String(b.timestamp || '')));
    }
    chronicleMessages.set(grouped);
  } catch {
    // non-fatal
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
  project:           'projects',
  task:              'tasks',
  note:              'notes',
  codex:             'codices',
  journal_entry:     'journal_entries',
  milestone:         'milestones',
  achievement:       'achievements',
  ritual:            'rituals',
  focus_session:     'focus_sessions',
  lore_unlock:       'lore_unlocks',
  chronicle_message: 'chronicle_messages',
  task_assignment:   'task_assignments',
  notification:      'notifications',
};

async function applyAndCacheEvent(event, signaturesRequired = false) {
  const hasSignature = Boolean(event.author_public_key && event.event_signature);
  if (signaturesRequired && !hasSignature) {
    throw new Error(`Unsigned sync event ${event.id} received after signature cutover`);
  }
  if (hasSignature && !(await verifyEvent(event))) {
    throw new Error(`Invalid durable signature on sync event ${event.id}`);
  }
  const { entity_type, entity_id, operation } = event;

  // Encrypted Fellowship events decrypt with the per-project key (looked up by
  // routing_id) instead of the account data key. If we don't hold that key yet,
  // skip the event without failing the pull so it can arrive after key delivery.
  let key;
  if (event.key_id === 'project-v1' || event.scope === 'project') {
    const rec = await getProjectKeyByRouting(event.routing_id);
    if (!rec) return false;
    key = await importProjectKey(projectKeyFromHex(rec.key_hex));
  } else {
    key = get(dataKey);
  }
  const plainContent = await decryptPayload(event, key);

  // Apply to Svelte stores (store.js handles all entity_type routing)
  applySyncEvent({ ...event, content: plainContent });

  // Persist decrypted entity to IndexedDB
  const payload = plainContent ? JSON.parse(plainContent) : null;

  if (entity_type === 'task_assignment' && operation !== 'delete' && payload) {
    const mine = String(get(identity)?.public_key || '').toLowerCase();
    if (mine && String(payload.user_identity || '').toLowerCase() === mine) {
      const title = get(tasks).get(payload.task_id)?.title || 'A quest';
      const notice = assignmentNotification(event.id, payload, title);
      if (notice) {
        try { await saveEntity('notifications', notice); } catch {}
      }
    }
  }

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
  let totalPulled = 0;
  let hasMore = true;

  try {
    while (hasMore) {
      const page = await api.request('POST', 'sync/v2/pull', null, { since_seq: seq, limit: 500, include_meta: 1 });
      const events = page.events;
      if (!Array.isArray(events) || events.length === 0) { hasMore = false; break; }

      for (const event of events) {
        await applyAndCacheEvent(event, Boolean(page.signatures_required));
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
  // A project we hold a key for is an encrypted Fellowship project: its events are
  // project-scoped, encrypted under the project key, and routed via questlinecli.com
  // only (the webapp DB cannot store project scope).
  const projectId = entityType === 'project' ? entityId : payload?.project_id;
  const projectKeyRec = projectId ? await getActiveProjectKeyByProject(projectId) : null;

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

  const event = {
    version:     2,
    id:          crypto.randomUUID(),
    entity_type: entityType,
    entity_id:   entityId,
    operation,
    timestamp:   new Date().toISOString(),
    key_id:      'account-v1',
    scope:       'account',
    routing_id:  '',
    device_id:   api.identity.device_id,
  };

  if (projectKeyRec) {
    event.key_id     = 'project-v1';
    event.scope      = 'project';
    event.routing_id = projectKeyRec.routing_id;
    const projectKey = await importProjectKey(projectKeyFromHex(projectKeyRec.key_hex));
    const encrypted = await encryptPayload(payload ? JSON.stringify(payload) : 'null', projectKey, event);
    Object.assign(event, encrypted);
    await signEvent(event, api.identity);
    await saveProjectOutboxEvent(event);

    // Project events go to questlinecli.com only; the server fans them out to the
    // other route members' encrypted accounts.
    try {
      const questlineClient = new ApiClient(api.identity, QUESTLINE_API_BASE);
      await questlineClient.post('sync/v2/push', [event]);
      await deleteProjectOutboxEvent(event.id);
      return { queued: false };
    } catch (err) {
      console.error('[sync] Fellowship push failed:', err);
      addToast('Fellowship change queued until the Realm reconnects', 'warning');
      return { queued: true };
    }
  }

  const key = get(dataKey);
  if (!key) throw new Error('Sync encryption key is unavailable');
  const encrypted = await encryptPayload(payload ? JSON.stringify(payload) : 'null', key, event);
  Object.assign(event, encrypted);
  await signEvent(event, api.identity);

  // Push to primary webapp API
  try {
    await api.post('sync/v2/push', [event]);
  } catch (err) {
    console.error('[sync] webapp push failed:', err);
    addToast('Sync failed — changes saved locally', 'warning');
    return;
  }

  // Dual-push to questlinecli.com so the CLI stays in sync
  try {
    const questlineClient = new ApiClient(api.identity, QUESTLINE_API_BASE);
    await questlineClient.post('sync/v2/push', [event]);
  } catch {
    // Non-fatal — webapp DB is source of truth; CLI will catch up via its own sync
  }
}

export async function flushProjectOutbox(identityValue) {
  const queued = await loadProjectOutboxEvents();
  if (queued.length === 0) return 0;
  const questlineClient = new ApiClient(identityValue, QUESTLINE_API_BASE);
  let sent = 0;
  for (const item of queued) {
    try {
      await questlineClient.post('sync/v2/push', [item.event]);
      await deleteProjectOutboxEvent(item.id);
      sent += 1;
    } catch {
      // Preserve order: a later event may depend on this one.
      break;
    }
  }
  return sent;
}

// ── Project pull ───────────────────────────────────────────────────────────────
// Project events live only on questlinecli.com. We pull them on a dedicated cursor,
// applying project-scoped events locally (account events arrive via the webapp poll).
// Rotation keys/revocations are processed first so new-route events can decrypt.

const PROJECT_SEQ_KEY = 'questline_project_seq';
function getLastProjectSeq() { return parseInt(localStorage.getItem(PROJECT_SEQ_KEY) || '0', 10); }
function setLastProjectSeq(seq) { localStorage.setItem(PROJECT_SEQ_KEY, String(seq)); }

export async function pullProjectSync(identity) {
  await processKeyEnvelopesAndRevocations(identity);

  const questlineClient = new ApiClient(identity, QUESTLINE_API_BASE);
  let seq = getLastProjectSeq();
  let hasMore = true;

  while (hasMore) {
    const page = await questlineClient.request('POST', 'sync/v2/pull', null, { since_seq: seq, limit: 500, include_meta: 1 });
    const events = page.events;
    if (!Array.isArray(events) || events.length === 0) break;

    for (const event of events) {
      if (event.scope === 'project' || event.key_id === 'project-v1') {
        try {
          await applyAndCacheEvent(event, Boolean(page.signatures_required));
        } catch (err) {
          // Do not advance the durable cursor past an event we could not
          // authenticate or decrypt. Earlier events may replay on retry, but
          // cache writes are idempotent and the failed event remains recoverable.
          throw new Error(`Could not apply Fellowship event ${event.id}: ${err.message}`, { cause: err });
        }
      }
      if (event.seq > seq) seq = event.seq;
    }
    if (events.length < 500) hasMore = false;
  }

  setLastProjectSeq(seq);
}

// ── Import: pull all events from questlinecli.com → store in webapp DB ────────
//
// Flow:
//   1. Pull all encrypted sync-v2 events from questlinecli.com (seq=0, batches of 500)
//   2. POST each batch to the webapp API (webapp/import) → gibranlp_webappquest
//   3. Apply events locally (IndexedDB + Svelte stores) for instant UI
//   4. Register a webhook on questlinecli.com so future CLI pushes propagate here
//
// onProgress(count) is called after each batch with the running total.

export async function importFromQuestline(webappApi, identity, onProgress = null) {
  syncStatus.set('syncing');
  let total = 0;
  let cliCursor = getLastCLISeq();

  try {
    for await (const batch of pullAllFromQuestline(identity)) {
      // Send batch to webapp backend (stores in gibranlp_webappquest)
      await webappApi.post('webapp/import', batch);

      // Apply locally
      for (const event of batch) {
        await applyAndCacheEvent(event, Boolean(batch.signatures_required));
        if (event.seq > cliCursor) cliCursor = event.seq;
      }

      total += batch.length;
      if (onProgress) onProgress(total);
    }

    // Register a webhook so future CLI pushes replicate here automatically
    await registerWebhookOnQuestline(webappApi, identity);

    setLastCLISeq(cliCursor);
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
    const webappApiUrl = import.meta.env.VITE_API_URL || 'https://webapp.questlinecli.com/api/';
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
  const seq = getLastCLISeq();

  try {
    const questlineClient = new ApiClient(identity, QUESTLINE_API_BASE);
    let cursor = seq;
    let hasMore = true;

    while (hasMore) {
      const page = await questlineClient.request('POST', 'sync/v2/pull', null, { since_seq: cursor, limit: 500, include_meta: 1 });
      const events = page.events;
      if (!Array.isArray(events) || events.length === 0) break;

      await webappApi.post('webapp/import', events);
      for (const event of events) {
        await applyAndCacheEvent(event, Boolean(page.signatures_required));
        if (event.seq > cursor) cursor = event.seq;
      }

      total += events.length;
      if (onProgress) onProgress(total);
      if (events.length < 500) hasMore = false;
    }

    if (cursor > seq) setLastCLISeq(cursor);
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
  let pullInFlight = false;

  const poll = async () => {
    if (pullInFlight) return;
    pullInFlight = true;
    try { await flushProjectOutbox(api.identity); } catch { /* retry next tick */ }
    try { await pullSync(api); } catch { /* retry next tick */ }
    try { await pullProjectSync(api.identity); } catch { /* retry next tick */ }
    finally { pullInFlight = false; }
  };

  const start = () => {
    if (pullTimer) return;
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
