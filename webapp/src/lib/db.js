// IndexedDB local cache — decrypted entity state, populated by the sync engine

const DB_NAME = 'questline_local_v1';
const DB_VERSION = 2;

// Regular entity stores — keyed by entity UUID
const ENTITY_STORES = [
  'projects', 'tasks', 'notes', 'codices',
  'journal_entries', 'milestones', 'achievements',
  'rituals', 'focus_sessions', 'lore_unlocks',
  'chronicle_messages',
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
  return new Promise((resolve, reject) => {
    const tx = db.transaction(ALL_STORES, 'readwrite');
    for (const name of ALL_STORES) tx.objectStore(name).clear();
    tx.oncomplete = resolve;
    tx.onerror    = () => reject(tx.error);
  });
}
