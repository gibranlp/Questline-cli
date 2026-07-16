<script>
  import { onMount } from 'svelte';
  import { userStats, streaks, zenTree, achievements, tasks, notes, projects, focusSessions } from '../lib/store.js';

  $: stats = $userStats;
  $: streak = $streaks;
  $: tree = $zenTree;

  // Safe store destructuring & evaluations
  $: totalTasksCompleted = $tasks ? [...$tasks.values()].filter(t => t.completed).length : 0;
  $: totalNotesCreated = $notes ? $notes.size : 0;
  $: totalProjects = $projects ? $projects.size : 0;
  $: totalFocusSessions = $focusSessions ? $focusSessions.size : 0;
  $: totalFocusHours = $focusSessions ? [...$focusSessions.values()].reduce((acc, s) => acc + (s.duration_mins || 0), 0) / 60 : 0;

  $: activeAchievements = $achievements ? [...$achievements.values()] : [];

  // Relics requirements mapping
  $: relics = [
    {
      id: 'ancient_quill',
      name: 'Ancient Quill',
      desc: 'A feather plucked from an owl of the high canopy. It writes with invisible ink that glows only under moonlight.',
      requirement: 'Create 25 notes (Scholar achievement)',
      unlocked: totalNotesCreated >= 25 || activeAchievements.some(a => a.achievement_type === 'scholar')
    },
    {
      id: 'crystal_compass',
      name: 'Crystal Compass',
      desc: 'Its needle does not point north, but toward the nearest unfinished task.',
      requirement: 'Complete 10 projects (Project Master achievement)',
      unlocked: ($projects ? [...$projects.values()].filter(p => p.completed).length : 0) >= 10 || activeAchievements.some(a => a.achievement_type === 'project_master')
    },
    {
      id: 'rune_tablet',
      name: 'Rune Tablet',
      desc: 'An ancient stone slab inscribed with glowing symbols that pulse in harmony with your tree.',
      requirement: 'Reach Level 50',
      unlocked: (stats?.level ?? 1) >= 50
    },
    {
      id: 'explorers_map',
      name: 'Explorer\'s Map',
      desc: 'A dusty parchment depicting shifting landscapes that update as your streak grows.',
      requirement: 'Reach a 30-day streak',
      unlocked: (streak?.best_streak ?? 0) >= 30
    },
    {
      id: 'clock_of_focus',
      name: 'Clock of Focus',
      desc: 'A pocket watch that ticks slower when you are concentrated, expanding time itself.',
      requirement: 'Complete 50 focus sessions (Deep Worker achievement)',
      unlocked: totalFocusSessions >= 50 || activeAchievements.some(a => a.achievement_type === 'deep_worker')
    }
  ];

  let selectedRelicIdx = 0;
  $: selectedRelic = relics[selectedRelicIdx];

  // Zen tree stage helper
  $: treeStage = (() => {
    const growth = tree?.growth ?? 0;
    if (growth < 10) return 'Acorn';
    if (growth < 25) return 'Entling';
    if (growth < 50) return 'Young Entling';
    if (growth < 100) return 'Grove Sapling';
    if (growth < 200) return 'Mallorn Tree';
    if (growth < 300) return 'Ancient Ent';
    return 'World Tree';
  })();
</script>

<div class="legends-page">
  <div class="page-header">
    <h1 class="page-title">The Hall of Legends</h1>
    <p class="subtitle dim">A sanctuary displaying your legendary accomplishments and relics.</p>
  </div>

  <div class="layout">
    <!-- Left panel: Cognitive Triumphs & Records -->
    <div class="card records-card">
      <h2 class="card-title">Cognitive Triumphs & Records</h2>
      <div class="records-list">
        <div class="record-item">
          <span class="label">Longest Streak:</span>
          <span class="val bold warning">{streak?.best_streak ?? 0} Days</span>
        </div>
        <div class="record-item">
          <span class="label">Total Quests Slayed:</span>
          <span class="val bold white">{totalTasksCompleted} Quests</span>
        </div>
        <div class="record-item">
          <span class="label">Total Scrolls Written:</span>
          <span class="val bold white">{totalNotesCreated} Scrolls</span>
        </div>
        <div class="record-item">
          <span class="label">Total Campaigns Begun:</span>
          <span class="val bold white">{totalProjects} Campaigns</span>
        </div>
        <div class="record-item">
          <span class="label">Zen Tree Stage:</span>
          <span class="val bold success">{treeStage} ({tree?.growth ?? 0} Growth)</span>
        </div>
        <div class="record-item">
          <span class="label">Deep Focus Honored:</span>
          <span class="val bold cyan">{totalFocusHours.toFixed(1)} Hours ({totalFocusSessions} sessions)</span>
        </div>
      </div>

      <div class="achievements-section">
        <h3 class="section-subtitle bold warning">Unlocked Achievements</h3>
        <div class="achievements-grid">
          {#each activeAchievements as ach}
            <div class="achievement-pill">
              <span class="pill-icon">🏆</span>
              <div class="pill-body">
                <span class="pill-name bold white">{ach.name ?? ach.achievement_type}</span>
                <span class="pill-desc dim">{ach.description ?? ''}</span>
              </div>
            </div>
          {:else}
            <div class="empty">No achievements unlocked yet. Stay consistent to earn your place in history.</div>
          {/each}
        </div>
      </div>
    </div>

    <!-- Right panel: Relics Inventory -->
    <div class="card relics-card">
      <h2 class="card-title">Relics Inventory</h2>
      <div class="relics-layout">
        <!-- Relic selector list -->
        <div class="relics-list">
          {#each relics as relic, idx}
            <button
              class="relic-item-btn"
              class:active={selectedRelicIdx === idx}
              class:locked={!relic.unlocked}
              on:click={() => selectedRelicIdx = idx}
            >
              <span class="relic-icon">{relic.unlocked ? '💎' : '🔒'}</span>
              <span class="relic-name bold">{relic.name}</span>
            </button>
          {/each}
        </div>

        <!-- Relic detail pane -->
        <div class="relic-detail-pane">
          {#if selectedRelic}
            <div class="relic-header">
              <span class="relic-big-icon">{selectedRelic.unlocked ? '💎' : '🔒'}</span>
              <h3 class="relic-title bold" class:unlocked={selectedRelic.unlocked} class:locked={!selectedRelic.unlocked}>
                {selectedRelic.name}
              </h3>
              <span class="badge" class:unlocked={selectedRelic.unlocked} class:locked={!selectedRelic.unlocked}>
                {selectedRelic.unlocked ? 'Legendary Relic' : 'Relic Locked'}
              </span>
            </div>

            <div class="relic-info">
              <p class="relic-desc italic text">
                {selectedRelic.unlocked ? selectedRelic.desc : 'This relic\'s mysteries remain hidden from you.'}
              </p>
              <div class="relic-req-box">
                <span class="req-label dim">Requirement:</span>
                <p class="req-val bold" class:success={selectedRelic.unlocked} class:warning={!selectedRelic.unlocked}>
                  {selectedRelic.requirement}
                </p>
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .legends-page {
    padding: 2rem;
    max-width: 1400px;
    margin: 0 auto;
  }

  .page-header {
    margin-bottom: 2rem;
  }

  .page-title {
    font-size: 1rem;
    font-weight: 700;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: #a855f7;
    margin-bottom: 0.25rem;
  }

  .subtitle {
    font-size: 0.85rem;
  }

  .layout {
    display: grid;
    grid-template-columns: 1fr 1.2fr;
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

  .records-list {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    border-bottom: 1px solid #111;
    padding-bottom: 1.25rem;
    margin-bottom: 1.25rem;
  }

  .record-item {
    display: flex;
    justify-content: space-between;
    font-size: 0.85rem;
  }

  .label {
    color: #555;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .val {
    color: #888;
  }

  .bold { font-weight: 600; }
  .white { color: #d4d4d4; }
  .cyan { color: #06b6d4; }
  .warning { color: #f59e0b; }
  .success { color: #22c55e; }
  .orange { color: #f97316; }
  .magenta { color: #ec4899; }
  .dim { color: #555; }
  .text { color: #bbb; }
  .italic { font-style: italic; }

  .achievements-section {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .section-subtitle {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }

  .achievements-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.75rem;
    max-height: 35vh;
    overflow-y: auto;
    padding-right: 0.25rem;
  }

  .achievement-pill {
    display: flex;
    gap: 0.75rem;
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid #1c1c1c;
    border-radius: 6px;
    padding: 0.6rem 0.8rem;
    align-items: flex-start;
  }

  .pill-icon {
    font-size: 1.1rem;
    margin-top: 0.15rem;
  }

  .pill-body {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }

  .pill-name {
    font-size: 0.8rem;
  }

  .pill-desc {
    font-size: 0.7rem;
    line-height: 1.3;
  }

  .relics-layout {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: 1.25rem;
    min-height: 350px;
  }

  .relics-list {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    border-right: 1px solid #111;
    padding-right: 0.5rem;
  }

  .relic-item-btn {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    background: none;
    border: none;
    color: #666;
    font-family: inherit;
    font-size: 0.82rem;
    padding: 0.5rem;
    text-align: left;
    cursor: pointer;
    border-radius: 4px;
    transition: background 0.15s, color 0.15s;
  }

  .relic-item-btn:hover {
    background: rgba(255, 255, 255, 0.02);
    color: #d4d4d4;
  }

  .relic-item-btn.active {
    background: rgba(168, 85, 247, 0.12);
    color: #a855f7;
  }

  .relic-item-btn.locked {
    opacity: 0.4;
  }

  .relic-detail-pane {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .relic-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 0.5rem;
    border-bottom: 1px solid #111;
    padding-bottom: 1rem;
  }

  .relic-big-icon {
    font-size: 3rem;
  }

  .relic-title {
    font-size: 1.1rem;
    letter-spacing: 0.05em;
  }

  .relic-title.unlocked { color: #a855f7; text-shadow: 0 0 10px rgba(168,85,247,0.3); }
  .relic-title.locked { color: #444; }

  .badge {
    font-size: 0.68rem;
    padding: 0.15rem 0.5rem;
    border-radius: 3px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .badge.unlocked { background: rgba(168, 85, 247, 0.12); border: 1px solid #a855f7; color: #a855f7; }
  .badge.locked { background: rgba(0,0,0,0.4); border: 1px solid #333; color: #555; }

  .relic-info {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .relic-desc {
    font-size: 0.88rem;
    line-height: 1.6;
    text-align: center;
  }

  .relic-req-box {
    background: rgba(0,0,0,0.3);
    border: 1px solid #1c1c1c;
    border-radius: 6px;
    padding: 0.75rem 1rem;
    text-align: center;
  }

  .req-label {
    display: block;
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    margin-bottom: 0.25rem;
  }

  .req-val {
    font-size: 0.82rem;
  }

  .empty {
    color: #444;
    font-size: 0.8rem;
    text-align: center;
    padding: 2rem 0;
  }
</style>
