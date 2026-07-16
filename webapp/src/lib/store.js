// Svelte stores — in-memory state built from sync events

import { writable, derived } from 'svelte/store';

export const identity = writable(null);
export const apiClient = writable(null);
// In-memory data encryption key (CryptoKey). Never persisted beyond sessionStorage bytes.
export const dataKey = writable(null);

// Entity stores — Maps keyed by entity UUID
export const projects = writable(new Map());
export const tasks = writable(new Map());
export const notes = writable(new Map());
export const codices = writable(new Map());
export const journalEntries = writable(new Map());
export const milestones = writable(new Map());
export const achievements = writable(new Map());
export const rituals = writable(new Map());
export const focusSessions = writable(new Map());
export const loreUnlocks = writable(new Map()); // keyed by lore entry id → {id, unlocked, unlocked_at}
export const chronicleMessages = writable(new Map()); // keyed by project_id -> []
export const userStats = writable(null);
export const zenTree = writable(null);
export const streaks = writable(null);
export const dailyQuests = writable([]);

// UI state
export const currentRoute = writable('/');
export const currentProjectId = writable(null);
export const toasts = writable([]);
export const syncStatus = writable('idle'); // idle | syncing | error

// Derived: projects sorted by name
export const sortedProjects = derived(projects, $p =>
  [...$p.values()].sort((a, b) => a.name.localeCompare(b.name))
);

// Derived: tasks for current project
export const currentProjectTasks = derived(
  [tasks, currentProjectId],
  ([$tasks, $pid]) => {
    if (!$pid) return [];
    return [...$tasks.values()]
      .filter(t => t.project_id === $pid && !t.completed)
      .sort((a, b) => {
        const prio = { High: 0, Medium: 1, Low: 2 };
        return (prio[a.priority] ?? 1) - (prio[b.priority] ?? 1);
      });
  }
);

// Derived: notes for current project
export const currentProjectNotes = derived(
  [notes, currentProjectId],
  ([$notes, $pid]) => {
    if (!$pid) return [];
    return [...$notes.values()].filter(n => n.project_id === $pid);
  }
);

export function addToast(msg, type = 'info', duration = 4000) {
  const id = crypto.randomUUID();
  toasts.update(t => [...t, { id, msg, type }]);
  setTimeout(() => {
    toasts.update(t => t.filter(x => x.id !== id));
  }, duration);
}

// Apply a sync event to the appropriate store
export function applySyncEvent(event) {
  const { entity_type, entity_id, operation, content } = event;
  const payload = content ? JSON.parse(content) : null;

  const storeMap = {
    project: projects,
    task: tasks,
    note: notes,
    codex: codices,
    journal_entry: journalEntries,
    milestone: milestones,
    achievement: achievements,
    ritual: rituals,
    focus_session: focusSessions,
  };

  // CLI sends entity_type "user" for character data (level, xp, class)
  if ((entity_type === 'user' || entity_type === 'user_stats') && payload) {
    userStats.set(payload);
    return;
  }
  if (entity_type === 'zen_tree' && payload) {
    zenTree.set(payload);
    return;
  }
  if (entity_type === 'streaks' && payload) {
    streaks.set(payload);
    return;
  }
  if (entity_type === 'lore_unlock' && entity_id && payload) {
    loreUnlocks.update(m => { const n = new Map(m); n.set(entity_id, { id: entity_id, ...payload }); return n; });
    return;
  }

  const store = storeMap[entity_type];
  if (!store) return;

  store.update(map => {
    const next = new Map(map);
    if (operation === 'delete') {
      next.delete(entity_id);
    } else if (payload) {
      next.set(entity_id, { ...payload, id: entity_id });
    }
    return next;
  });
}
