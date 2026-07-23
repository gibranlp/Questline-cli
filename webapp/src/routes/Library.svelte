<script>
  import { loreUnlocks, userStats, achievements } from '../lib/store.js';
  import Legends from './Legends.svelte';

  const LORE_URL   = 'https://questlinecli.com/data/lore.json';
  const QUESTS_URL = 'https://questlinecli.com/data/quests.json';

  const CATEGORIES = ['Class Quests', 'Class Stories', 'World History', 'Achievement Lore', 'Memory Fragments'];

  let tab = 'library'; // 'library' | 'legends'
  let selectedCat = 0;
  let selectedItemIdx = 0;

  let loreEntries = [];
  let quests      = [];
  let loading     = true;
  let loadError   = '';

  // Fetch both data files in parallel on mount
  import { onMount } from 'svelte';
  onMount(async () => {
    try {
      const [loreRes, questsRes] = await Promise.all([
        fetch(LORE_URL),
        fetch(QUESTS_URL),
      ]);
      if (!loreRes.ok || !questsRes.ok) throw new Error('Data fetch failed');
      const loreData   = await loreRes.json();
      const questsData = await questsRes.json();
      loreEntries = loreData.entries   ?? [];
      quests      = questsData.quests  ?? [];
    } catch (e) {
      loadError = e.message ?? 'Failed to load library data';
    } finally {
      loading = false;
    }
  });

  // ── Derive unlock status from loreUnlocks store + user stats ──────────────

  function isUnlocked(entry) {
    // lore_unlock sync event always wins — the CLI is authoritative
    const synced = $loreUnlocks.get(entry.id);
    if (synced?.unlocked) return true;

    const u = entry.unlock;
    if (!u) return false;

    switch (u.type) {
      case 'free':
        return true;
      case 'level':
        return ($userStats?.level ?? 0) >= u.level;
      case 'class_level':
        return ($userStats?.class_name ?? $userStats?.class ?? '') === u.class
          && ($userStats?.level ?? 0) >= u.level;
      default:
        return false; // discovery / milestone / chapter_reward → only via lore_unlock sync
    }
  }

  function questUnlocked(q) {
    const level = $userStats?.level ?? 0;
    return level >= q.level;
  }

  function questCompleted(q) {
    // When a quest is completed the CLI creates quest_lore_{level}
    return $loreUnlocks.has(`quest_lore_${q.level}`) && $loreUnlocks.get(`quest_lore_${q.level}`)?.unlocked;
  }

  function questStatus(q) {
    if (questCompleted(q)) return 'Completed';
    if (questUnlocked(q))  return 'Available';
    return 'Locked';
  }

  // ── Build item lists per category ─────────────────────────────────────────

  $: userClass = $userStats?.class_name ?? $userStats?.class ?? '';

  $: classKey = (() => {
    const map = {
      'Code Warlock':      'warlock',
      'Task Paladin':      'paladin',
      'Mind Sage':         'sage',
      'Systems Architect': 'architect',
      'Time Chronomancer': 'chronomancer',
      'Arch Accountant':   'accountant',
    };
    return map[userClass] ?? '';
  })();

  $: myQuests = quests
    .filter(q => q.class === userClass)
    .sort((a, b) => a.level - b.level);

  $: classStories = loreEntries
    .filter(e => e.category === 'Class'
      && (e.id === 'class_six_orders'
          || e.id === 'class_council_orders'
          || (classKey && e.id.startsWith(`class_${classKey}_`))))
    .sort((a, b) => a.sort_order - b.sort_order);

  $: worldHistory = loreEntries
    .filter(e => e.category === 'World')
    .sort((a, b) => a.sort_order - b.sort_order);

  $: achievementLore = loreEntries
    .filter(e => e.category === 'Achievement')
    .sort((a, b) => a.sort_order - b.sort_order);

  $: memoryFragments = loreEntries
    .filter(e => e.category === 'Memory')
    .sort((a, b) => a.sort_order - b.sort_order);

  $: currentItems = (() => {
    switch (selectedCat) {
      case 0: return myQuests;
      case 1: return classStories;
      case 2: return worldHistory;
      case 3: return achievementLore;
      case 4: return memoryFragments;
      default: return [];
    }
  })();

  $: { currentItems; selectedItemIdx = 0; } // reset on category change

  $: selectedItem = currentItems[selectedItemIdx] ?? null;

  // Memory fragment counts
  $: foundFragments = memoryFragments.filter(e => isUnlocked(e)).length;
  $: totalFragments = memoryFragments.length;

  function rarityColor(rarity) {
    switch (rarity) {
      case 'legendary': return '#f59e0b';
      case 'rare':      return '#06b6d4';
      default:          return '#555';
    }
  }

  function statusColor(s) {
    switch (s) {
      case 'Completed': return '#22c55e';
      case 'Available': return '#06b6d4';
      default:          return '#383838';
    }
  }

  function objectiveLabel(q) {
    const o = q.objective;
    switch (o?.type) {
      case 'tasks_completed':   return `Complete ${o.target} tasks`;
      case 'focus_minutes':     return `${o.target} minutes of deep focus`;
      case 'zen_waterings':     return `Water the Zen Tree ${o.target} times`;
      case 'projects_completed':return `Complete ${o.target} project${o.target === 1 ? '' : 's'}`;
      case 'streak_days':       return `Maintain a ${o.target}-day streak`;
      default:                  return q.description ?? '';
    }
  }
</script>

<div class="library-page">
  <div class="page-header">
    <h1 class="page-title">The Lore Library</h1>
    <div class="tabs">
      <button class="tab" class:active={tab === 'library'} on:click={() => tab = 'library'}>
        📜 Archive
      </button>
      <button class="tab" class:active={tab === 'legends'} on:click={() => tab = 'legends'}>
        🏆 Hall of Legends
      </button>
    </div>
  </div>

  {#if tab === 'library'}
    {#if loading}
      <div class="loading-state">
        <div class="spinner"></div>
        <span>Loading the archive…</span>
      </div>

    {:else if loadError}
      <div class="error-state">
        <span>⚠ {loadError}</span>
      </div>

    {:else}
      <div class="three-col">

        <!-- ── Col 1: Categories ─────────────────────────────── -->
        <div class="col col-categories">
          <div class="col-title">Categories</div>
          <ul class="cat-list">
            {#each CATEGORIES as cat, i}
              <li>
                <button
                  class="cat-item"
                  class:active={selectedCat === i}
                  on:click={() => selectedCat = i}
                >
                  <span class="cat-prefix">{selectedCat === i ? '›' : ' '}</span>
                  {cat}
                  {#if i === 4}
                    <span class="frag-badge">{foundFragments}/{totalFragments}</span>
                  {/if}
                </button>
              </li>
            {/each}
          </ul>
        </div>

        <!-- ── Col 2: Items ──────────────────────────────────── -->
        <div class="col col-items">
          <div class="col-title">
            {#if selectedCat === 4}
              Memory Fragments · Found: {foundFragments}/{totalFragments}
            {:else}
              {CATEGORIES[selectedCat]}
            {/if}
          </div>

          {#if currentItems.length === 0}
            <div class="empty-col">No entries available.</div>
          {:else}
            <ul class="item-list">
              {#each currentItems as item, i}
                {@const unlocked = selectedCat === 0 ? questUnlocked(item) : isUnlocked(item)}
                {@const color = selectedCat === 4 ? rarityColor(item.rarity) : (unlocked ? '#888' : '#2a2a2a')}
                <li>
                  <button
                    class="item-btn"
                    class:active={selectedItemIdx === i}
                    style="--item-color: {color}"
                    on:click={() => selectedItemIdx = i}
                  >
                    <span class="item-prefix">{selectedItemIdx === i ? '›' : ' '}</span>
                    {#if !unlocked}<span class="lock-mark">L </span>{/if}
                    <span class="item-label">
                      {#if selectedCat === 0}
                        Lvl {item.level} — {item.name}
                      {:else}
                        {item.title}
                      {/if}
                    </span>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>

        <!-- ── Col 3: Detail ─────────────────────────────────── -->
        <div class="col col-detail">
          <div class="col-title">Chronicle Details</div>

          {#if !selectedItem}
            <div class="empty-col">Select an entry to read it.</div>

          {:else if selectedCat === 0}
            <!-- Class Quest detail -->
            {@const status = questStatus(selectedItem)}
            {@const sColor = statusColor(status)}
            <div class="detail-scroll">
              <div class="detail-field"><span class="df-label">QUEST:</span><span class="df-val bold">{selectedItem.name}</span></div>
              <div class="detail-field"><span class="df-label">CLASS:</span><span class="df-val accent">{selectedItem.class} (Level {selectedItem.level} Quest)</span></div>
              <div class="detail-field"><span class="df-label">STATUS:</span><span class="df-val bold" style="color:{sColor}">{status}</span></div>

              <div class="detail-section-title">OBJECTIVE</div>
              <div class="detail-body-text">{objectiveLabel(selectedItem)}</div>

              <div class="detail-section-title">REWARD</div>
              <div class="detail-body-text dim">{selectedItem.lore_reward}</div>

              {#if status === 'Available'}
                <div class="quest-cta cyan">Embark from the CLI app to start this quest.</div>
              {:else if status === 'Completed'}
                <div class="quest-cta green">Quest completed. Reward unlocked.</div>
              {:else}
                <div class="quest-cta dim">Reach Level {selectedItem.level} to unlock this quest.</div>
              {/if}
            </div>

          {:else}
            <!-- Lore entry detail -->
            {@const unlocked = isUnlocked(selectedItem)}
            <div class="detail-scroll">
              {#if unlocked}
                <div class="detail-field"><span class="df-label">TITLE:</span><span class="df-val bold">{selectedItem.title}</span></div>
                <div class="detail-field"><span class="df-label">TYPE: </span><span class="df-val accent">{CATEGORIES[selectedCat]}</span></div>
                {#if selectedItem.rarity}
                  <div class="detail-field">
                    <span class="df-label">RARITY:</span>
                    <span class="df-val bold" style="color:{rarityColor(selectedItem.rarity)}">
                      [{selectedItem.rarity.toUpperCase()}]
                    </span>
                  </div>
                {/if}

                <div class="detail-divider">
                  {selectedCat === 4 ? '─── MEMORY FRAGMENT ───' : '─── CHRONICLE ENTRY ───'}
                </div>

                <pre class="detail-content">{selectedItem.content}</pre>
              {:else}
                <div class="locked-entry">
                  <div class="locked-title">RECORD LOCKED</div>
                  <p class="locked-desc">This chapter of lore remains hidden in the shadow of unfinished deeds.</p>
                  <div class="locked-req">
                    <span class="df-label">REQUIREMENT:</span>
                    <span class="dim">{selectedItem.unlock?.display ?? 'Unknown'}</span>
                  </div>
                </div>
              {/if}
            </div>
          {/if}
        </div>

      </div>
    {/if}

  {:else}
    <Legends />
  {/if}
</div>

<style>
  .library-page {
    padding: 2rem 2.5rem;
    max-width: 1600px;
    font-family: inherit;
  }

  .page-header {
    display: flex;
    align-items: center;
    gap: 2rem;
    margin-bottom: 1.75rem;
    flex-wrap: wrap;
  }

  .page-title {
    font-size: 1rem;
    font-weight: 700;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: #a855f7;
    flex-shrink: 0;
  }

  .tabs {
    display: flex;
    gap: 0.25rem;
    background: rgba(0,0,0,0.4);
    border: 1px solid #1c1c1c;
    border-radius: 8px;
    padding: 0.25rem;
  }

  .tab {
    background: none;
    border: none;
    color: #555;
    font-family: inherit;
    font-size: 0.78rem;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    padding: 0.45rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .tab:hover  { background: rgba(168,85,247,0.08); color: #d4d4d4; }
  .tab.active { background: rgba(168,85,247,0.18); color: #a855f7; }

  /* ── Three-column layout ─────────────────────────────────── */

  .three-col {
    display: grid;
    grid-template-columns: 200px 280px 1fr;
    gap: 0;
    border: 1px solid #1c1c1c;
    border-radius: 8px;
    overflow: hidden;
    min-height: calc(100vh - 180px);
  }

  @media (max-width: 900px) {
    .three-col { grid-template-columns: 1fr; }
  }

  .col {
    display: flex;
    flex-direction: column;
    border-right: 1px solid #111;
    background: rgba(0,0,0,0.5);
  }

  .col:last-child { border-right: none; }

  .col-title {
    padding: 0.6rem 0.8rem;
    font-size: 0.68rem;
    font-weight: 700;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    color: #444;
    border-bottom: 2px solid #111;
    flex-shrink: 0;
  }

  /* ── Categories ──────────────────────────────────────────── */

  .cat-list { list-style: none; padding: 0.35rem; flex: 1; }

  .cat-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.45rem 0.5rem;
    background: none;
    border: none;
    border-radius: 4px;
    color: #666;
    font-family: inherit;
    font-size: 0.8rem;
    cursor: pointer;
    text-align: left;
    letter-spacing: 0.03em;
    transition: background 0.12s, color 0.12s;
  }

  .cat-item:hover  { background: rgba(168,85,247,0.08); color: #ccc; }
  .cat-item.active { background: rgba(168,85,247,0.14); color: #a855f7; }
  .cat-prefix      { color: #a855f7; width: 10px; flex-shrink: 0; }

  .frag-badge {
    margin-left: auto;
    font-size: 0.62rem;
    color: #444;
    border: 1px solid #2a2a2a;
    border-radius: 8px;
    padding: 0.05em 0.4em;
  }

  /* ── Items ───────────────────────────────────────────────── */

  .item-list { list-style: none; padding: 0.35rem; flex: 1; overflow-y: auto; max-height: calc(100vh - 260px); }

  .item-btn {
    width: 100%;
    display: flex;
    align-items: flex-start;
    gap: 0.3rem;
    padding: 0.4rem 0.5rem;
    background: none;
    border: none;
    border-radius: 4px;
    color: var(--item-color, #555);
    font-family: inherit;
    font-size: 0.78rem;
    cursor: pointer;
    text-align: left;
    line-height: 1.4;
    transition: background 0.12s, color 0.12s;
  }

  .item-btn:hover  { background: rgba(168,85,247,0.07); }
  .item-btn.active { background: rgba(168,85,247,0.14); color: #d4d4d4; }

  .item-prefix  { color: #a855f7; width: 10px; flex-shrink: 0; margin-top: 1px; }
  .lock-mark    { color: #333; flex-shrink: 0; }
  .item-label   { flex: 1; }

  /* ── Detail ──────────────────────────────────────────────── */

  .col-detail { border-right: none; border: none; }

  .detail-scroll {
    padding: 1.25rem 1.5rem;
    overflow-y: auto;
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .detail-field {
    display: flex;
    gap: 0.75rem;
    font-size: 0.8rem;
    align-items: baseline;
    flex-wrap: wrap;
  }

  .df-label {
    color: #555;
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    flex-shrink: 0;
    width: 68px;
  }

  .df-val  { color: #bbb; }
  .bold    { font-weight: 700; }
  .accent  { color: #a855f7; }
  .dim     { color: #555; }

  .detail-section-title {
    font-size: 0.68rem;
    font-weight: 700;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    color: #f59e0b;
    margin-top: 0.75rem;
  }

  .detail-body-text {
    font-size: 0.82rem;
    color: #d4d4d4;
    line-height: 1.6;
  }

  .detail-divider {
    font-size: 0.72rem;
    font-weight: 700;
    color: #f59e0b;
    letter-spacing: 0.1em;
    margin: 0.75rem 0 0.25rem;
  }

  .detail-content {
    font-family: inherit;
    font-size: 0.82rem;
    color: #d4d4d4;
    line-height: 1.75;
    white-space: pre-wrap;
    word-break: break-word;
    margin: 0;
  }

  .quest-cta {
    margin-top: 1rem;
    padding: 0.6rem 0.8rem;
    border-radius: 5px;
    font-size: 0.78rem;
    font-style: italic;
    border: 1px solid currentColor;
  }

  .quest-cta.cyan  { color: #06b6d4; border-color: rgba(6,182,212,0.3); background: rgba(6,182,212,0.06); }
  .quest-cta.green { color: #22c55e; border-color: rgba(34,197,94,0.3); background: rgba(34,197,94,0.06); }
  .quest-cta.dim   { color: #444;    border-color: #1c1c1c;              background: transparent; }

  /* ── Locked state ────────────────────────────────────────── */

  .locked-entry {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 2rem 0;
    text-align: center;
  }

  .locked-title {
    font-size: 0.82rem;
    font-weight: 700;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: #ef4444;
  }

  .locked-desc {
    font-size: 0.82rem;
    color: #555;
    line-height: 1.6;
    font-style: italic;
  }

  .locked-req {
    display: flex;
    gap: 0.5rem;
    align-items: baseline;
    justify-content: center;
    font-size: 0.78rem;
    flex-wrap: wrap;
  }

  /* ── Loading / error ─────────────────────────────────────── */

  .loading-state, .error-state {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 3rem 1rem;
    color: #555;
    font-size: 0.82rem;
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid #1c1c1c;
    border-top-color: #a855f7;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    flex-shrink: 0;
  }

  .empty-col {
    padding: 2rem 1rem;
    font-size: 0.78rem;
    color: #333;
    font-style: italic;
    text-align: center;
  }

  @keyframes spin { to { transform: rotate(360deg); } }
</style>
