<script>
  import { onMount } from 'svelte';
  import { apiClient, addToast } from '../lib/store.js';

  let loading = true;
  let activeChapter = null;
  let feed = [];

  const objectiveNames = {
    tasks_completed: 'Quests Completed',
    subtasks_completed: 'Steps Completed',
    focus_sessions: 'Focus Sessions Done',
    tree_waterings: 'Zen Tree Waterings',
    rituals_completed: 'Rituals Executed',
    reflections_written: 'Daily Reflections Logged',
    scrolls_created: 'Scrolls Penned'
  };

  async function loadData() {
    if (!$apiClient) return;
    loading = true;
    try {
      // 1. Load active chapter stats
      const chapterData = await $apiClient.get('chapter/active', { chapter_id: 'chapter_one' });
      activeChapter = chapterData;

      // 2. Load global chronicle feed
      const feedData = await $apiClient.get('global_chronicle');
      feed = Array.isArray(feedData) ? feedData : [];
    } catch (err) {
      addToast('Error loading Chronicles: ' + err.message, 'error');
    } finally {
      loading = false;
    }
  }

  onMount(loadData);

  function relativeTime(ts) {
    const dt = new Date(ts);
    if (isNaN(dt.getTime())) return ts;
    const diffMs = Date.now() - dt.getTime();
    const secs = Math.max(0, Math.floor(diffMs / 1000));
    if (secs < 60) return 'just now';
    if (secs < 3600) {
      const m = Math.floor(secs / 60);
      return `${m} minute${m === 1 ? '' : 's'} ago`;
    }
    if (secs < 86400) {
      const h = Math.floor(secs / 3600);
      return `${h} hour${h === 1 ? '' : 's'} ago`;
    }
    const days = Math.floor(secs / 86400);
    if (days === 1) return 'Yesterday';
    if (days < 7) return `${days} days ago`;
    return dt.toLocaleDateString();
  }

  function getIcon(eventType) {
    const icons = {
      LevelUp: '⚡',
      RealmComplete: '👑',
      Milestone: '🚩',
      Relic: '💎',
      Streak: '🔥',
      Memory: '🔮',
      QuestComplete: '⚔️',
      FocusSession: '🎯',
      ReflectionWritten: '📝',
      ScrollCreated: '📜',
      ChapterComplete: '🌟',
      Achievement: '🏆'
    };
    return icons[eventType] ?? '🛡️';
  }

  $: totalCur = activeChapter ? activeChapter.objectives.reduce((acc, o) => acc + Math.min(o.current_value, o.target_value), 0) : 0;
  $: totalTarget = activeChapter ? activeChapter.objectives.reduce((acc, o) => acc + o.target_value, 0) : 0;
  $: percent = totalTarget > 0 ? Math.round((totalCur / totalTarget) * 100) : 0;
</script>

<div class="chronicles-page">
  <div class="page-header">
    <h1 class="page-title">The Great Chronicle</h1>
    <button class="btn-primary" on:click={loadData} disabled={loading}>Refresh Feed</button>
  </div>

  {#if loading}
    <div class="loading-screen">
      <div class="spinner"></div>
      <p>Reading the Chronicle records…</p>
    </div>
  {:else}
    <div class="layout">
      <!-- Left side: Global Event Feed -->
      <div class="card feed-card">
        <h2 class="card-title">Realm Activity Feed</h2>
        <div class="feed-list">
          {#each feed as entry (entry.id)}
            <div class="feed-item">
              <div class="feed-icon">{getIcon(entry.event_type)}</div>
              <div class="feed-body">
                <div class="feed-line">
                  <span class="hero-name bold warning">{entry.hero_name}</span>
                  <span class="feed-desc">{entry.description}</span>
                </div>
                <div class="feed-time dim">{relativeTime(entry.timestamp)}</div>
              </div>
            </div>
          {:else}
            <div class="empty">The Great Chronicle is silent. The realm awaits action.</div>
          {/each}
        </div>
      </div>

      <!-- Right side: Living Chapter Stats -->
      <div class="right-panel">
        <div class="card chapter-card">
          <h2 class="card-title">Active Chapter</h2>
          {#if activeChapter}
            <div class="chapter-info">
              <h3 class="chapter-title bold text">Chapter One: The Notification Swarm</h3>
              <p class="chapter-lore italic dim">
                The realm is under siege by an endless flood of interruptions — alerts, pings, distractions without end. The only answer is focus. Join forces with other heroes to conquer the swarm.
              </p>

              <!-- Overall completion bar -->
              <div class="completion-box">
                <span class="label">Realm Progress</span>
                <div class="bar-container">
                  <div class="bar-fill" style="width: {percent}%"></div>
                </div>
                <div class="bar-labels">
                  <span class="bold color-primary">{percent}%</span>
                  <span class="dim">{totalCur.toLocaleString()} / {totalTarget.toLocaleString()} units</span>
                </div>
              </div>

              <!-- Objectives list -->
              <div class="objectives-list">
                <h4 class="sub-lbl bold">Objectives</h4>
                {#each activeChapter.objectives as obj}
                  {@const progress = Math.min(100, Math.round((obj.current_value / obj.target_value) * 100))}
                  <div class="obj-item">
                    <div class="obj-details">
                      <span class="obj-name bold text">{objectiveNames[obj.objective_type] ?? obj.objective_type}</span>
                      <span class="dim">{obj.current_value.toLocaleString()} / {obj.target_value.toLocaleString()}</span>
                    </div>
                    <div class="bar-container mini">
                      <div class="bar-fill objective" style="width: {progress}%"></div>
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {:else}
            <div class="empty">No active chapter loaded. Sync to refresh chapter progress.</div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .chronicles-page {
    padding: 2rem;
    max-width: 1400px;
    margin: 0 auto;
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 2rem;
  }

  .page-title {
    font-size: 1rem;
    font-weight: 700;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: #a855f7;
  }

  .layout {
    display: grid;
    grid-template-columns: 1fr 450px;
    gap: 1.5rem;
  }

  @media (max-width: 1024px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }

  .card {
    background: rgba(0, 0, 0, 0.6);
    border: 1px solid #1c1c1c;
    border-radius: 8px;
    padding: 1.5rem;
  }

  .card-title {
    font-size: 0.72rem;
    font-weight: 600;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: #555;
    margin-bottom: 1.25rem;
    border-bottom: 1px solid #111;
    padding-bottom: 0.5rem;
  }

  .feed-list {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    max-height: 70vh;
    overflow-y: auto;
    padding-right: 0.5rem;
  }

  .feed-item {
    display: flex;
    gap: 1rem;
    align-items: flex-start;
    padding: 0.5rem 0;
    border-bottom: 1px solid rgba(255,255,255,0.02);
  }

  .feed-icon {
    font-size: 1.25rem;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(168, 85, 247, 0.08);
    border-radius: 6px;
    border: 1px solid #1c1c1c;
  }

  .feed-body {
    flex: 1;
  }

  .feed-line {
    font-size: 0.88rem;
    line-height: 1.4;
  }

  .hero-name {
    margin-right: 0.5rem;
  }

  .feed-desc {
    color: #bbb;
  }

  .feed-time {
    font-size: 0.72rem;
    margin-top: 0.25rem;
  }

  .right-panel {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .chapter-info {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .chapter-title {
    font-size: 0.95rem;
  }

  .chapter-lore {
    font-size: 0.8rem;
    line-height: 1.5;
  }

  .completion-box {
    margin-top: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .bar-container {
    height: 8px;
    background: #1c1c1c;
    border-radius: 4px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    background: #a855f7;
    border-radius: 4px;
  }

  .bar-fill.objective {
    background: #06b6d4;
  }

  .bar-labels {
    display: flex;
    justify-content: space-between;
    font-size: 0.72rem;
  }

  .objectives-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    border-top: 1px solid #111;
    padding-top: 1rem;
  }

  .sub-lbl {
    font-size: 0.72rem;
    color: #555;
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  .obj-item {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .obj-details {
    display: flex;
    justify-content: space-between;
    font-size: 0.8rem;
  }

  .bar-container.mini {
    height: 4px;
  }

  .bold { font-weight: 600; }
  .text { color: #d4d4d4; }
  .dim { color: #555; }
  .warning { color: #f59e0b; }
  .italic { font-style: italic; }
  .color-primary { color: #a855f7; }

  .loading-screen {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 50vh;
    gap: 1rem;
    color: #555;
  }

  .spinner {
    width: 32px;
    height: 32px;
    border: 2px solid #1c1c1c;
    border-top-color: #a855f7;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  .empty {
    color: #444;
    font-size: 0.85rem;
    padding: 1rem 0;
  }

  .btn-primary {
    background: rgba(168, 85, 247, 0.15);
    border: 1px solid #a855f7;
    border-radius: 5px;
    color: #a855f7;
    font-family: inherit;
    font-size: 0.78rem;
    padding: 0.4rem 0.8rem;
    cursor: pointer;
    letter-spacing: 0.05em;
    font-weight: 600;
    text-transform: uppercase;
    transition: background 0.15s, color 0.15s;
  }

  .btn-primary:hover:not(:disabled) {
    background: rgba(168, 85, 247, 0.28);
    color: #c084fc;
  }
</style>
