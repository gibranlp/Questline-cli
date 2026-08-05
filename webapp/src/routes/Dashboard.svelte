<script>
  import { onMount } from 'svelte';
  import ZenTree from '../components/ZenTree.svelte';
  import TaskItem from '../components/TaskItem.svelte';
  import { userStats, zenTree, streaks, tasks, dailyQuests, myAssignedTasks, notifications, addToast } from '../lib/store.js';
  import { saveEntity } from '../lib/db.js';
  import { pullSync } from '../lib/sync.js';
  import { navigate } from '../lib/router.js';
  import { get } from 'svelte/store';

  export let api;

  let loading = true;
  let syncProgress = 0;

  onMount(async () => {
    try {
      await pullSync(api, p => { syncProgress = p; });
      // Fetch snapshot for singleton entities that may have no recent sync events
      if (!get(userStats) || !get(zenTree)) {
        try {
          const snap = await api.get('webapp/snapshot');
          if (snap.user && !get(userStats)) userStats.set(snap.user);
          if (snap.zen_tree && !get(zenTree)) zenTree.set(snap.zen_tree);
        } catch { /* snapshot unavailable — non-fatal */ }
      }
    } catch (err) {
      addToast('Sync error: ' + err.message, 'error');
    } finally {
      loading = false;
    }
  });

  // Main quest = highest-priority incomplete task (mirrors planner.rs logic)
  $: allTasks = [...$tasks.values()];
  $: mainQuest = scoreAndRank(allTasks.filter(t => !t.completed))[0] ?? null;
  $: nextQuest = scoreAndRank(allTasks.filter(t => !t.completed))[1] ?? null;
  $: todayStr = new Date().toISOString().slice(0, 10);
  $: todayTasks = allTasks.filter(t => !t.completed && t.due_date?.startsWith(todayStr));
  $: unreadAssignments = [...$notifications.values()]
    .filter(n => n.notification_type === 'task_assignment' && !n.read)
    .sort((a, b) => String(b.created_at || '').localeCompare(String(a.created_at || '')));

  async function openAssignmentNotice(notice) {
    const updated = { ...notice, read: true };
    notifications.update(map => { const next = new Map(map); next.set(notice.id, updated); return next; });
    try { await saveEntity('notifications', updated); } catch {}
    if (notice.project_id) navigate(`/projects/${notice.project_id}`);
  }

  function scoreTask(t) {
    const now = new Date();
    const due = t.due_date ? new Date(t.due_date) : null;
    let score = 0;
    if (due) {
      const diff = Math.floor((due - now) / 86400000);
      if (diff < 0) score += 100;
      else if (diff === 0) score += 60;
      else if (diff === 1) score += 40;
      else if (diff <= 3) score += 25;
      else if (diff <= 7) score += 10;
    }
    const prio = { High: 30, Medium: 10, Low: 0 };
    score += prio[t.priority] ?? 0;
    return score;
  }

  function scoreAndRank(taskList) {
    return [...taskList].sort((a, b) => scoreTask(b) - scoreTask(a));
  }

  $: stats = $userStats;
  $: tree = $zenTree;
  $: streak = $streaks;

  function xpForLevel(level) {
    return 200 + level * level * 12;
  }

  $: xpProgress = stats
    ? Math.min(100, Math.round((stats.xp / xpForLevel(stats.level)) * 100))
    : 0;
</script>

<div class="dashboard">
  {#if loading}
    <div class="loading-screen">
      <div class="spinner"></div>
      <p>Syncing the Realm… {syncProgress > 0 ? `(${syncProgress} events)` : ''}</p>
    </div>
  {:else}
    <div class="grid">
      {#if unreadAssignments.length > 0}
        <div class="card notice-card">
          <h2 class="section-label">Fellowship Dispatches</h2>
          {#each unreadAssignments.slice(0, 5) as notice (notice.id)}
            <button class="notice-row" on:click={() => openAssignmentNotice(notice)}>
              <span>⚜</span>
              <span><strong>{notice.title}</strong><small>{notice.content}</small></span>
              <span>Open →</span>
            </button>
          {/each}
        </div>
      {/if}
      <!-- Main Quest -->
      <div class="card main-quest">
        <h2 class="section-label">Main Quest</h2>
        {#if mainQuest}
          <div class="quest-title">{mainQuest.title}</div>
          {#if mainQuest.due_date}
            <div class="quest-due">Due: {mainQuest.due_date}</div>
          {/if}
          <div class="quest-prio prio-{mainQuest.priority?.toLowerCase()}">{mainQuest.priority}</div>
          <TaskItem task={mainQuest} {api} />
        {:else}
          <div class="empty">No quests — the backlog is clear.</div>
        {/if}
      </div>

      <!-- Next Quest -->
      {#if nextQuest}
        <div class="card next-quest">
          <h2 class="section-label">Recommended Next</h2>
          <TaskItem task={nextQuest} {api} />
        </div>
      {/if}

      <!-- Character Stats -->
      <div class="card stats-card">
        <h2 class="section-label">Hero Status</h2>
        {#if stats}
          <div class="stat-row">
            <span class="stat-label">Class</span>
            <span class="stat-value accent">{stats.class ?? '—'}</span>
          </div>
          <div class="stat-row">
            <span class="stat-label">Level</span>
            <span class="stat-value">{stats.level ?? 1}</span>
          </div>
          <div class="xp-bar-wrap">
            <div class="xp-bar">
              <div class="xp-fill" style="width: {xpProgress}%"></div>
            </div>
            <span class="xp-label">{stats.xp ?? 0} XP</span>
          </div>
          <div class="stat-row">
            <span class="stat-label">Streak</span>
            <span class="stat-value">{streak?.current_streak ?? stats.streak ?? 0} days 🔥</span>
          </div>
        {:else}
          <div class="empty">No stats yet — complete a sync.</div>
        {/if}
      </div>

      <!-- Zen Tree -->
      <div class="card zen-card">
        <h2 class="section-label">The Evergrowth</h2>
        {#if tree}
          <ZenTree stage={tree.stage ?? 1} health={tree.health ?? 100} />
        {:else}
          <ZenTree stage={1} health={100} />
        {/if}
      </div>

      <!-- Quick tasks today -->
      <div class="card tasks-card">
        <h2 class="section-label">Due Today</h2>
        {#if todayTasks.length === 0}
          <div class="empty">Nothing due today.</div>
        {:else}
          {#each todayTasks as t (t.id)}
            <TaskItem task={t} {api} />
          {/each}
        {/if}
        <button class="btn-secondary" on:click={() => navigate('/projects')}>
          View all projects →
        </button>
      </div>

      <div class="card tasks-card">
        <h2 class="section-label">My Quests</h2>
        {#if $myAssignedTasks.length === 0}
          <div class="empty">No active quests are assigned to you.</div>
        {:else}
          {#each $myAssignedTasks.slice(0, 6) as t (t.id)}
            <TaskItem task={t} {api} />
          {/each}
          {#if $myAssignedTasks.length > 6}
            <div class="empty">+ {$myAssignedTasks.length - 6} more assigned quests</div>
          {/if}
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .dashboard { padding: 2rem; }

  .loading-screen {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 60vh;
    gap: 1rem;
    color: #555;
    font-size: 0.9rem;
  }

  .spinner {
    width: 36px;
    height: 36px;
    border: 2px solid #1c1c1c;
    border-top-color: #a855f7;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 1.25rem;
  }

  .card {
    background: rgba(0,0,0,0.6);
    border: 1px solid #1c1c1c;
    border-radius: 8px;
    padding: 1.25rem;
  }

  .notice-card { grid-column: 1 / -1; border-color: rgba(168,85,247,0.35); }
  .notice-row {
    width: 100%; display: grid; grid-template-columns: auto 1fr auto; align-items: center;
    gap: 0.75rem; background: none; border: 0; border-bottom: 1px solid #171717;
    color: #a855f7; cursor: pointer; font: inherit; padding: 0.65rem 0; text-align: left;
  }
  .notice-row:hover { color: #c084fc; }
  .notice-row strong, .notice-row small { display: block; }
  .notice-row strong { color: #d4d4d4; font-size: 0.78rem; }
  .notice-row small { color: #666; font-size: 0.7rem; margin-top: 0.1rem; }

  .section-label {
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: #555;
    margin-bottom: 1rem;
  }

  .quest-title {
    font-size: 1rem;
    color: #d4d4d4;
    margin-bottom: 0.25rem;
  }

  .quest-due { font-size: 0.8rem; color: #f59e0b; margin-bottom: 0.5rem; }

  .stat-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.4rem 0;
    border-bottom: 1px solid #111;
    font-size: 0.85rem;
  }

  .stat-label { color: #555; }
  .stat-value { color: #d4d4d4; }
  .accent { color: #a855f7; }

  .xp-bar-wrap {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6rem 0;
  }

  .xp-bar {
    flex: 1;
    height: 4px;
    background: #1c1c1c;
    border-radius: 2px;
    overflow: hidden;
  }

  .xp-fill {
    height: 100%;
    background: #a855f7;
    border-radius: 2px;
    transition: width 0.5s ease;
  }

  .xp-label { font-size: 0.75rem; color: #555; }

  .zen-card { text-align: center; }

  .empty { color: #444; font-size: 0.85rem; padding: 0.5rem 0; }

  .btn-secondary {
    background: none;
    border: 1px solid #2a2a2a;
    border-radius: 5px;
    color: #666;
    font-family: inherit;
    font-size: 0.78rem;
    padding: 0.4rem 0.75rem;
    cursor: pointer;
    margin-top: 0.75rem;
    letter-spacing: 0.05em;
    transition: color 0.15s, border-color 0.15s;
  }

  .btn-secondary:hover { color: #a855f7; border-color: #a855f7; }
</style>
