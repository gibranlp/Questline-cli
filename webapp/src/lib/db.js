// IndexedDB local cache — decrypted entity state, populated by the sync engine

const DB_NAME = 'questline_local_v1';
const DB_VERSION = 5;

// Encrypted Fellowship project keys — keyed by opaque routing_id.
// Records: { routing_id, project_id, key_hex, retired }. Durable at rest, like the
// CLI's project_encryption_keys table; delivered via invitation accept / rotation.
const PROJECT_KEYS_STORE = 'project_keys';
const PROJECT_OUTBOX_STORE = 'project_outbox';

// Regular entity stores — keyed by entity UUID
const ENTITY_STORES = [
  'projects', 'tasks', 'notes', 'codices',
  'journal_entries', 'milestones', 'achievements',
  'rituals', 'focus_sessions', 'lore_unlocks',
  'chronicle_messages',
  'task_assignments', 'notifications',
];
// Singleton stores — keyed by the fixed string 'singleton'
const SINGLETON_STORES = ['user_stats', 'zen_tree', 'streaks'];

const ALL_STORES = [...ENTITY_STORES, ...SINGLETON_STORES];

let _db = null;

function openDB() {
  if (_db) return Promise.resolve(_db);
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = (e) => {
      const db = e.target.result;
      for (const name of ALL_STORES) {
        if (!db.objectStoreNames.contains(name)) {
          db.createObjectStore(name, { keyPath: 'id' });
        }
      }
      if (!db.objectStoreNames.contains(PROJECT_KEYS_STORE)) {
        const store = db.createObjectStore(PROJECT_KEYS_STORE, { keyPath: 'routing_id' });
        store.createIndex('by_project', 'project_id', { unique: false });
      }
      if (!db.objectStoreNames.contains(PROJECT_OUTBOX_STORE)) {
        db.createObjectStore(PROJECT_OUTBOX_STORE, { keyPath: 'id' });
      }
    };
    req.onsuccess = () => { _db = req.result; resolve(_db); };
    req.onerror  = () => reject(req.error);
  });
}

export async function saveEntity(storeName, entity) {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(storeName, 'readwrite');
    tx.objectStore(storeName).put(entity);
    tx.oncomplete = resolve;
    tx.onerror    = () => reject(tx.error);
  });
}

export async function deleteEntity(storeName, id) {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(storeName, 'readwrite');
    tx.objectStore(storeName).delete(id);
    tx.oncomplete = resolve;
    tx.onerror    = () => reject(tx.error);
  });
}

export async function loadAllEntities(storeName) {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(storeName, 'readonly');
    const req = tx.objectStore(storeName).getAll();
    req.onsuccess = () => resolve(req.result);
    req.onerror   = () => reject(req.error);
  });
}

export async function clearLocalDatabase() {
  const db = await openDB();
  const stores = [...ALL_STORES, PROJECT_KEYS_STORE, PROJECT_OUTBOX_STORE];
  return new Promise((resolve, reject) => {
    const tx = db.transaction(stores, 'readwrite');
    for (const name of stores) tx.objectStore(name).clear();
    tx.oncomplete = resolve;
    tx.onerror    = () => reject(tx.error);
  });
}

export async function saveProjectOutboxEvent(event) {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(PROJECT_OUTBOX_STORE, 'readwrite');
    tx.objectStore(PROJECT_OUTBOX_STORE).put({ id: event.id, event });
    tx.oncomplete = resolve;
    tx.onerror = () => reject(tx.error);
  });
}

export async function deleteProjectOutboxEvent(id) {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(PROJECT_OUTBOX_STORE, 'readwrite');
    tx.objectStore(PROJECT_OUTBOX_STORE).delete(id);
    tx.oncomplete = resolve;
    tx.onerror = () => reject(tx.error);
  });
}

export async function loadProjectOutboxEvents() {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(PROJECT_OUTBOX_STORE, 'readonly');
    const req = tx.objectStore(PROJECT_OUTBOX_STORE).getAll();
    req.onsuccess = () => resolve(req.result || []);
    req.onerror = () => reject(req.error);
  });
}

// ── Encrypted Fellowship project keys ─────────────────────────────────────────

export async function saveProjectKey({ routing_id, project_id, key_hex, retired = false }) {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(PROJECT_KEYS_STORE, 'readwrite');
    tx.objectStore(PROJECT_KEYS_STORE).put({ routing_id, project_id, key_hex, retired });
    tx.oncomplete = resolve;
    tx.onerror    = () => reject(tx.error);
  });
}

export async function getProjectKeyByRouting(routingId) {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(PROJECT_KEYS_STORE, 'readonly');
    const req = tx.objectStore(PROJECT_KEYS_STORE).get(routingId);
    req.onsuccess = () => resolve(req.result || null);
    req.onerror   = () => reject(req.error);
  });
}

export async function deleteProjectKey(routingId) {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(PROJECT_KEYS_STORE, 'readwrite');
    tx.objectStore(PROJECT_KEYS_STORE).delete(routingId);
    tx.oncomplete = resolve;
    tx.onerror = () => reject(tx.error);
  });
}

// The active (non-retired) key for a project, if any.
export async function getActiveProjectKeyByProject(projectId) {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(PROJECT_KEYS_STORE, 'readonly');
    const req = tx.objectStore(PROJECT_KEYS_STORE).index('by_project').getAll(projectId);
    req.onsuccess = () => resolve((req.result || []).find(r => !r.retired) || null);
    req.onerror   = () => reject(req.error);
  });
}

export async function getAllProjectKeys() {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(PROJECT_KEYS_STORE, 'readonly');
    const req = tx.objectStore(PROJECT_KEYS_STORE).getAll();
    req.onsuccess = () => resolve(req.result || []);
    req.onerror   = () => reject(req.error);
  });
}

export async function markRouteRetired(routingId) {
  const rec = await getProjectKeyByRouting(routingId);
  if (!rec || rec.retired) return;
  await saveProjectKey({ ...rec, retired: true });
}
