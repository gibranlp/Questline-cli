<script>
  import { onMount } from 'svelte';
  import { userStats, achievements, streaks, zenTree, tasks, projects, notes, journalEntries, focusSessions, apiClient, addToast } from '../lib/store.js';
  import { pushEvent } from '../lib/sync.js';
  import Modal from '../components/Modal.svelte';

  // Class definitions — mirrors user.rs class definitions, mottoes, descriptions, passives
  const classDetails = {
    CodeWarlock: {
      name: 'Code Warlock',
      description: 'Summoner of scripts, breaker of production.',
      motto: 'Caffeine in. Code out. The loop must continue.',
      order: 'The Terminal Covenant',
      passive: '+5 XP per note  |  +15 XP new project  |  +10 XP on sync',
      color: '#a855f7',
      specs: [
        { name: 'Automation Mage', desc: '+10% XP from Note Creation' },
        { name: 'System Weaver', desc: '+10% XP from Project Completion' },
        { name: 'Bug Hunter', desc: '+10% XP from Task Completion' }
      ],
      powers: [
        'Terminal Spark — A blinking cursor becomes a gateway',
        'Hello World Ritual — Print one line of text and begin',
        'Hex Compiler — Scripts run 10% faster in your mind',
        'Debug Vision — See root causes at a glance',
        'Daemon Binding — Automate recurring tasks',
        'Sigil Weaving — Create powerful macros',
        'Dark Compilation — Compile code in dreams',
        'Void Parsing — Parse any format instantly',
        'Memory Leech — Extract data from any source',
        'Stack Trace Oracle — Predict errors before they happen',
        'Null Pointer Charm — Prevent null reference errors',
        'Arcane Caching — Cache results automatically',
        'Syntax Conjuration — Write code by thought',
        'Runtime Hex — Modify programs while running',
        'Refactor Ritual — Improve code quality passively',
        'Dependency Daemon — Manage packages effortlessly',
        'Test Oracle — Foresee test outcomes',
        'CI/CD Familiar — Deploy without fear',
        'Architecture Vision — See system design clearly',
        'Daemon Lord Ascension — Master of all scripts'
      ]
    },
    TaskPaladin: {
      name: 'Task Paladin',
      description: 'Holy warrior against procrastination.',
      motto: 'The to-do list shall be purified.',
      order: 'The Sacred Checklist',
      passive: '+5 XP per task  |  +10 XP high priority  |  +15 XP full daily chain',
      color: '#ff69b4',
      specs: [
        { name: 'Execution Knight', desc: '+10% XP from Task Completion' },
        { name: 'Guardian of Order', desc: '+10% XP from Project Completion' },
        { name: 'Momentum Crusader', desc: '+10% XP from Note Creation' }
      ],
      powers: [
        'Squire\'s Resolve — Stand firm before the backlog',
        'Checklist Blessing — Complete tasks with precision',
        'Holy Checklist — Complete tasks with divine precision',
        'Shield of Focus — Block distractions',
        'Crusader\'s Pace — Maintain steady progress',
        'Smite Procrastination — Destroy delays',
        'Sacred Deadline — Honor all commitments',
        'Battle Prioritization — Always know what matters most',
        'Righteous Momentum — Build unstoppable streaks',
        'Paladin\'s Oath — Never abandon a quest',
        'Divine Scheduling — Plan with holy clarity',
        'Judgment Day — Clear backlog decisively',
        'Holy Delegation — Assign tasks perfectly',
        'Consecrate Project — Bless team productivity',
        'Aura of Completion — Inspire others to finish',
        'Sacred Archive — Perfect record keeping',
        'Templar\'s Rhythm — Work in perfect cycles',
        'Crusade Leader — Guide team quests',
        'Holy Retrospective — Learn from every sprint',
        'Paladin Grandmaster — The ultimate executor'
      ]
    },
    MindSage: {
      name: 'Mind Sage',
      description: 'Cartographer of thoughts and master of knowledge trees.',
      motto: 'Every idea is a node. Every node is power.',
      order: 'The Silent Archive',
      passive: '+10 XP long notes  |  +5 XP journals  |  +10% fragment chance  |  +5 XP per fragment',
      color: '#06b6d4',
      specs: [
        { name: 'Knowledge Keeper', desc: '+10% XP from Note Creation' },
        { name: 'Cognitive Cartographer', desc: '+10% XP from Project Completion' },
        { name: 'Insight Seeker', desc: '+10% XP from Task Completion' }
      ],
      powers: [
        'Node Connection — Map your first thoughts',
        'Mind Palace Initiate — Retain basic lore',
        'Knowledge Synthesis — Connect ideas across domains',
        'Memory Palace — Perfect information retention',
        'Deep Contemplation — Solve complex problems',
        'Wisdom Archive — Curate knowledge perfectly',
        'Mindful Focus — Enter flow state at will',
        'Conceptual Mapping — Visualize any system',
        'Insight Cascade — Generate breakthrough ideas',
        'Socratic Method — Question to clarity',
        'Pattern Recognition — See hidden connections',
        'Abstract Thinking — Transcend concrete limits',
        'Knowledge Crystallization — Make complex simple',
        'Philosophical Framework — Build mental models',
        'Metacognitive Awareness — Think about thinking',
        'Zettelkasten Mastery — Perfect note organization',
        'Second Brain Activation — Offload cognitive load',
        'Wisdom Synthesis — Merge disparate knowledge',
        'Enlightened Note-taking — Capture everything perfectly',
        'Omniscient Archive — Know everything you\'ve learned'
      ]
    },
    SystemsArchitect: {
      name: 'Systems Architect',
      description: 'Builder of order from chaos.',
      motto: 'Give me enough folders and I will organize the universe.',
      order: 'The Builders of Order',
      passive: '+10 XP new project  |  +15 XP archive  |  +5 XP restore',
      color: '#3b82f6',
      specs: [
        { name: 'Infrastructure Builder', desc: '+10% XP from Project Completion' },
        { name: 'Process Optimizer', desc: '+10% XP from Task Completion' },
        { name: 'Modular Designer', desc: '+10% XP from Note Creation' }
      ],
      powers: [
        'Directory Blueprint — Plan your folders',
        'Process Drafter — Define simple steps',
        'Blueprint Vision — See system design clearly',
        'Process Optimization — Eliminate inefficiencies',
        'Workflow Mastery — Design perfect processes',
        'Systems Thinking — Understand complex interactions',
        'Architecture Review — Spot structural flaws',
        'Dependency Mapping — Track all connections',
        'Performance Engineering — Optimize at every level',
        'Scalability Foresight — Design for growth',
        'Technical Documentation — Write perfect specs',
        'Infrastructure as Code — Automate everything',
        'Observability Master — Monitor everything',
        'Resilience Engineering — Build fault-tolerant systems',
        'Capacity Planning — Predict future needs',
        'Security by Design — Build secure foundations',
        'Microservices Mastery — Decompose systems perfectly',
        'Event-Driven Architecture — Design reactive systems',
        'Data Architecture — Design perfect schemas',
        'Systems Grand Architect — Master of all systems'
      ]
    },
    TimeChronomancer: {
      name: 'Time Chronomancer',
      description: 'Manipulator of hours, minutes, and deadlines.',
      motto: 'Time is not money. Time is everything.',
      order: 'The Keepers of Hours',
      passive: '+10 XP focus sessions  |  +25 XP pomodoros  |  +5 XP daily adventures',
      color: '#f97316',
      specs: [
        { name: 'Temporal Ward', desc: '+10% XP from Task Completion' },
        { name: 'History Weaver', desc: '+10% XP from Note Creation' },
        { name: 'Timeline Editor', desc: '+10% XP from Project Completion' }
      ],
      powers: [
        'Second Tracker — Track your first minute',
        'Hourglass Initiate — Measure focus intervals',
        'Time Dilation — Focus makes hours feel like minutes',
        'Temporal Planning — Schedule with perfect precision',
        'Pomodoro Mastery — Work in perfect cycles',
        'Time Blocking — Defend your calendar',
        'Deep Work Portal — Enter unbreakable focus',
        'Priority Time Shift — Always work on what matters',
        'Chronological Archive — Perfect time tracking',
        'Time Audit Vision — See where time really goes',
        'Deadline Manipulation — Meet every deadline',
        'Meeting Minimization — Reclaim lost hours',
        'Energy Management — Work with your natural rhythms',
        'Async Mastery — Communicate without meetings',
        'Batching Temporal Magic — Group similar tasks',
        'Time ROI Vision — Focus on high-value activities',
        'Procrastination Banishment — Start immediately',
        'Flow State Summoning — Enter peak performance',
        'Temporal Boundaries — Protect personal time',
        'Master of Chronos — Bend time to your will'
      ]
    },
    ArchAccountant: {
      name: 'Arch Accountant',
      description: 'Master of ledgers, destroyer of bad financial decisions.',
      motto: 'Numbers do not lie. But accountants can make them confess.',
      order: 'The Order of the Ledger',
      passive: '+2 XP all rewards  |  +5% XP all sources  |  +10 XP full daily chain',
      color: '#f59e0b',
      specs: [
        { name: 'Ledger Overseer', desc: '+10% XP from Note Creation' },
        { name: 'Audit Judge', desc: '+10% XP from Task Completion' },
        { name: 'Asset Growth Specialist', desc: '+10% XP from Project Completion' }
      ],
      powers: [
        'Receipt Acknowledgment — First entry in the ledger',
        'Spreadsheet Initiate — Simple SUM formulas',
        'Ledger of Truth — Perfect financial tracking',
        'Budget Mastery — Control every expense',
        'ROI Vision — See true value of investments',
        'Audit Trail — Track every change',
        'Resource Optimization — Maximize every asset',
        'Financial Forecasting — Predict with accuracy',
        'Cost-Benefit Analysis — Weigh every decision',
        'Deficit Elimination — Balance any budget',
        'Revenue Tracking — Monitor all income streams',
        'Expense Management — Categorize automatically',
        'Tax Optimization — Maximize efficiency legally',
        'Cash Flow Mastery — Manage timing perfectly',
        'Investment Analysis — Identify best opportunities',
        'Risk Quantification — Put numbers on uncertainty',
        'Variance Analysis — Spot deviations instantly',
        'Margin Optimization — Improve profitability',
        'Compliance Shield — Stay within all rules',
        'Grand Accountant — Master of all resources'
      ]
    }
  };

  $: stats = $userStats;
  $: streak = $streaks;
  $: tree = $zenTree;
  $: unlockedAchievements = [...$achievements.values()];

  // Devices & Local Reflections & Dynamic Log
  let devices = [];
  let reflections = [];
  let selectedReflectionIdx = 0;
  let showSpecModal = false;
  let showReflectionModal = false;

  // New Daily Reflection fields
  let newWentWell = '';
  let newCanImprove = '';

  function xpForLevel(level) {
    return 200 + level * level * 12;
  }

  $: xpNeeded = stats ? xpForLevel(stats.level ?? 1) : 200;
  $: xpProgress = stats ? Math.min(100, Math.round(((stats.xp ?? 0) / xpNeeded) * 100)) : 0;

  $: cleanClass = stats?.class?.replace(/\s+/g, '') ?? 'CodeWarlock';
  $: classInfo = classDetails[cleanClass] ?? classDetails.CodeWarlock;
  $: classColor = classInfo.color;

  $: powers = classInfo.powers ?? [];
  $: unlockedPowers = powers.filter((_, i) => {
    const unlockLevel = (Math.floor(i / 4) + 1) * 5;
    return (stats?.level ?? 1) >= unlockLevel;
  });

  // Calculate Title dynamically based on level & class
  $: currentTitle = getTitle(stats?.class, stats?.level ?? 1);

  function getTitle(cls, lvl) {
    const cleanCls = cls?.replace(/\s+/g, '') ?? 'CodeWarlock';
    const cInfo = classDetails[cleanCls];
    if (!cInfo) return 'Adventurer';
    const titlesList = cleanCls === 'CodeWarlock'
      ? ['Novice Coder', 'Script Adept', 'Terminal Magus', 'Daemon Lord', 'Master of Automation', 'Architect of Simulations']
      : cleanCls === 'TaskPaladin'
      ? ['Squire of Order', 'Keeper of Tasks', 'Knight of Completion', 'Champion of Discipline', 'Guardian of Momentum', 'The Unfinished Finisher']
      : cleanCls === 'MindSage'
      ? ['Apprentice Thinker', 'Mapmaker of Nodes', 'Mind Explorer', 'Keeper of Knowledge', 'Sage of Connections', 'Omniscient Mind Architect']
      : cleanCls === 'SystemsArchitect'
      ? ['Framework Apprentice', 'Blueprint Drafter', 'Builder of Structure', 'Order Designer', 'Architect of Flow', 'Cosmic System Designer']
      : cleanCls === 'TimeChronomancer'
      ? ['Watcher of Seconds', 'Minute Weaver', 'Hour Shaper', 'Deadline Shield', 'Master of Schedules', 'Master of Time']
      : ['Ledger Apprentice', 'Formula Initiate', 'Expense Judge', 'Golden Balancer', 'Portfolio Alchemist', 'Omniscient Ledger Lord'];

    if (lvl < 10) return titlesList[0];
    if (lvl < 25) return titlesList[1];
    if (lvl < 50) return titlesList[2];
    if (lvl < 75) return titlesList[3];
    if (lvl < 100) return titlesList[4];
    return titlesList[5];
  }

  // Calculate Best Realm / Most Productive Project based on tasks completed
  $: bestProjectName = (() => {
    const completedCounts = {};
    for (const t of $tasks.values()) {
      if (t.completed && t.project_id) {
        completedCounts[t.project_id] = (completedCounts[t.project_id] || 0) + 1;
      }
    }
    let max = 0;
    let bestId = null;
    for (const [pid, count] of Object.entries(completedCounts)) {
      if (count > max) {
        max = count;
        bestId = pid;
      }
    }
    if (bestId) {
      return $projects.get(bestId)?.name || 'None yet';
    }
    return 'None yet';
  })();

  // Generate Adventure Log dynamically from stores
  $: adventureLog = (() => {
    const events = [];

    // Tasks completed
    for (const t of $tasks.values()) {
      if (t.completed) {
        events.push({
          text: `Completed Quest: ${t.title}`,
          timestamp: t.updated_at || t.created_at || new Date().toISOString(),
          type: 'Quest'
        });
      }
    }

    // Notes written
    for (const n of $notes.values()) {
      events.push({
        text: `Penned Scroll: ${n.title}`,
        timestamp: n.created_at || new Date().toISOString(),
        type: 'Scroll'
      });
    }

    // Journal entries
    for (const j of $journalEntries.values()) {
      events.push({
        text: `Recorded Journal Entry`,
        timestamp: j.created_at || new Date().toISOString(),
        type: 'Journal'
      });
    }

    // Focus sessions completed
    for (const f of $focusSessions.values()) {
      events.push({
        text: `Focused for ${f.duration_mins} mins (${f.soundscape})`,
        timestamp: f.completed_at || new Date().toISOString(),
        type: 'Focus'
      });
    }

    // Sort descending by timestamp
    events.sort((a, b) => b.timestamp.localeCompare(a.timestamp));
    return events.slice(0, 100); // show up to 100 entries
  })();

  // Generate mock/local XP history
  $: xpHistory = (() => {
    const history = [];
    for (const f of $focusSessions.values()) {
      history.push({
        gain: f.xp_gained || 25,
        type: `Focus Session (${f.soundscape})`,
        timestamp: f.completed_at
      });
    }
    for (const t of $tasks.values()) {
      if (t.completed) {
        history.push({
          gain: t.priority === 'High' ? 50 : 25,
          type: `Completed Quest: ${t.title}`,
          timestamp: t.updated_at
        });
      }
    }
    history.sort((a, b) => (b.timestamp || '').localeCompare(a.timestamp || ''));
    return history.slice(0, 5); // recent 5 events
  })();

  onMount(async () => {
    // Load devices from API if logged in
    if ($apiClient) {
      try {
        devices = await $apiClient.get('devices');
      } catch (err) {
        console.error('Failed to load devices:', err);
      }
    }
    // Load reflections from localStorage
    reflections = JSON.parse(localStorage.getItem('questline_reflections') || '[]');
  });

  async function selectSpecialization(specName) {
    if (!stats) return;
    const updated = { ...stats, specialization: specName, updated_at: new Date().toISOString() };
    userStats.set(updated);
    try {
      await pushEvent($apiClient, 'user', stats.id || stats.user_uuid, 'upsert', updated);
      addToast(`Specialization unlocked: ${specName}`, 'success');
      showSpecModal = false;
    } catch (err) {
      addToast('Failed to save specialization: ' + err.message, 'error');
    }
  }

  function saveReflection() {
    if (!newWentWell.trim() || !newCanImprove.trim()) return;
    const today = new Date().toISOString().slice(0, 10);
    const newRef = {
      created_date: today,
      what_went_well: newWentWell.trim(),
      what_can_improve: newCanImprove.trim()
    };
    const stored = JSON.parse(localStorage.getItem('questline_reflections') || '[]');
    const filtered = stored.filter(r => r.created_date !== today);
    filtered.unshift(newRef);
    localStorage.setItem('questline_reflections', JSON.stringify(filtered));
    reflections = filtered;
    selectedReflectionIdx = 0;
    showReflectionModal = false;
    newWentWell = '';
    newCanImprove = '';
    addToast('Daily reflection recorded', 'success');
  }
</script>

<div class="character-page">
  <div class="page-header">
    <h1 class="page-title">Hero Sheet</h1>
    <button class="btn-primary" on:click={() => showReflectionModal = true}>Write Reflection</button>
  </div>

  <div class="hero-grid">
    <!-- Left Column: Specs, Summary, XP history, Adventure Log -->
    <div class="left-panel">
      <!-- Character Specs Card -->
      <div class="card specs-card">
        <h2 class="card-title">Character Specs</h2>
        <div class="specs-list">
          <div class="spec-row">
            <span class="label">Name:</span>
            <span class="val bold white">{stats?.username ?? '—'}</span>
          </div>
          <div class="spec-row">
            <span class="label">Class:</span>
            <span class="val bold" style="color: {classColor}">{classInfo.name}</span>
          </div>
          <div class="spec-row">
            <span class="label">Title:</span>
            <span class="val bold warning">{currentTitle}</span>
          </div>
          <div class="spec-row">
            <span class="label">Level:</span>
            <span class="val bold white">{stats?.level ?? 1}</span>
          </div>
          <div class="spec-row">
            <span class="label">Special:</span>
            {#if stats?.specialization}
              <span class="val bold cyan">{stats.specialization}</span>
            {:else if (stats?.level ?? 1) >= 10}
              <button class="btn-action-inline" on:click={() => showSpecModal = true}>Choose Specialization!</button>
            {:else}
              <span class="val dim">Locked (Unlocks at Lvl 10)</span>
            {/if}
          </div>
          <div class="spec-row">
            <span class="label">Created:</span>
            <span class="val text">{stats?.created_at?.slice(0, 10) ?? '—'}</span>
          </div>
          <div class="spec-row">
            <span class="label">Devices:</span>
            <span class="val devices-list">
              {devices.map(d => d.device_name).join(', ') || 'This device'}
            </span>
          </div>
        </div>

        <div class="motto-box">
          <span class="motto-label">Motto:</span>
          <p class="motto-text" style="color: {classColor}">"{classInfo.motto}"</p>
        </div>

        <div class="motto-box">
          <span class="motto-label">Description:</span>
          <p class="desc-text">{classInfo.description}</p>
        </div>
      </div>

      <!-- Progression Summary Card -->
      <div class="card summary-card">
        <h2 class="card-title">Progression Summary</h2>
        <div class="specs-list">
          <div class="spec-row">
            <span class="label">Achievements:</span>
            <span class="val bold cyan">{unlockedAchievements.length} / 14 Unlocked</span>
          </div>
          <div class="spec-row">
            <span class="label">Zen Tree:</span>
            {#if tree}
              <span class="val bold success">Stage {tree.stage ?? 1} ({tree.growth ?? 0} Growth, {tree.health ?? 100}% Health)</span>
            {:else}
              <span class="val bold success">Acorn (0 Growth, 100% Health)</span>
            {/if}
          </div>
          <div class="spec-row">
            <span class="label">Streak:</span>
            <span class="val orange">{streak?.current_streak ?? stats?.streak ?? 0} Days Active (Best: {streak?.best_streak ?? 0} Days)</span>
          </div>
          <div class="spec-row">
            <span class="label">Best Realm:</span>
            <span class="val bold magenta">{bestProjectName}</span>
          </div>
        </div>
      </div>

      <!-- XP Progression Gauge -->
      <div class="card xp-card">
        <h2 class="card-title">XP Progression</h2>
        <div class="xp-bar-container">
          <div class="xp-bar-bg">
            <div class="xp-bar-fill" style="width: {xpProgress}%; background-color: {classColor}"></div>
          </div>
          <div class="xp-labels">
            <span>{stats?.xp ?? 0} / {xpNeeded} XP</span>
            <span>{xpProgress}%</span>
          </div>
        </div>
      </div>

      <!-- Recent XP History Card -->
      <div class="card xp-history-card">
        <h2 class="card-title">Recent XP History</h2>
        <div class="xp-events-list">
          {#each xpHistory as event}
            <div class="xp-event-item">
              <span class="xp-gain">+{event.gain} XP</span>
              <span class="xp-event-desc">— {event.type}</span>
            </div>
          {:else}
            <div class="empty">No recent XP history events recorded.</div>
          {/each}
        </div>
      </div>

      <!-- Adventure Log Book -->
      <div class="card log-card">
        <h2 class="card-title">Adventure Log Book</h2>
        <div class="adventure-log-list">
          {#each adventureLog as entry}
            <div class="log-item">
              <span class="log-bullet" style="color: {classColor}">▶</span>
              <span class="log-text">{entry.text}</span>
              <span class="log-time">{entry.timestamp.slice(5, 16).replace('T', ' ')}</span>
            </div>
          {:else}
            <div class="empty">The chronicle is empty. Embark on quests and focus to write your history.</div>
          {/each}
        </div>
      </div>
    </div>

    <!-- Right Column: Class Progression Tree, Daily Reflections -->
    <div class="right-panel">
      <!-- Powers / Class Progression Tree Card -->
      <div class="card powers-card">
        <h2 class="card-title">{classInfo.name} Class Progression Tree</h2>
        <p class="passive-lbl">Passive: <span class="white">{classInfo.passive}</span></p>
        <div class="powers-list">
          {#each powers as power, i}
            {@const unlockLevel = (Math.floor(i / 4) + 1) * 5}
            {@const unlocked = (stats?.level ?? 1) >= unlockLevel}
            <div class="power-item" class:locked={!unlocked}>
              <span class="power-level" style="color: {unlocked ? classColor : '#333'}">
                {unlocked ? '✦' : '○'} Lv{unlockLevel}
              </span>
              <span class="power-name">{power}</span>
            </div>
          {/each}
        </div>
      </div>

      <!-- Daily Reflections Card -->
      <div class="card reflections-card">
        <h2 class="card-title">Daily Reflections History</h2>
        {#if reflections.length > 0}
          <div class="reflections-layout">
            <div class="reflections-sidebar">
              {#each reflections as ref, idx}
                <button
                  class="ref-date-btn"
                  class:active={selectedReflectionIdx === idx}
                  on:click={() => selectedReflectionIdx = idx}
                >
                  {ref.created_date}
                </button>
              {/each}
            </div>
            <div class="ref-detail">
              {#if reflections[selectedReflectionIdx]}
                <div class="ref-section">
                  <h4 class="ref-sub-title success">What Went Well:</h4>
                  <p class="ref-body">{reflections[selectedReflectionIdx].what_went_well}</p>
                </div>
                <div class="ref-section">
                  <h4 class="ref-sub-title warning">What Can Improve:</h4>
                  <p class="ref-body">{reflections[selectedReflectionIdx].what_can_improve}</p>
                </div>
              {/if}
            </div>
          </div>
        {:else}
          <div class="empty-center">
            <p>No daily reflections recorded yet.</p>
            <button class="btn-secondary" on:click={() => showReflectionModal = true}>Write Your First Reflection</button>
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>

<!-- Specialization Modal -->
<Modal open={showSpecModal} title="Unlock Class Specialization" onClose={() => showSpecModal = false}>
  <div class="spec-modal-content">
    <p class="spec-desc">Choose your subclass specialization path. Once chosen, it will enhance your journey:</p>
    <div class="specs-options">
      {#each classInfo.specs as spec}
        <button class="spec-option-card" on:click={() => selectSpecialization(spec.name)}>
          <div class="spec-opt-name bold white">{spec.name}</div>
          <div class="spec-opt-desc cyan">{spec.desc}</div>
        </button>
      {/each}
    </div>
  </div>
</Modal>

<!-- Write Reflection Modal -->
<Modal open={showReflectionModal} title="Daily Reflection" onClose={() => showReflectionModal = false}>
  <div class="reflection-form">
    <div class="field">
      <label for="went-well">What went well today?</label>
      <textarea id="went-well" bind:value={newWentWell} placeholder="Describe your victories..." rows="3" required></textarea>
    </div>
    <div class="field">
      <label for="can-improve">What can improve tomorrow?</label>
      <textarea id="can-improve" bind:value={newCanImprove} placeholder="Describe adjustments for tomorrow..." rows="3" required></textarea>
    </div>
    <button class="btn-primary full-width" on:click={saveReflection} disabled={!newWentWell.trim() || !newCanImprove.trim()}>
      Record Daily Reflection
    </button>
  </div>
</Modal>

<style>
  .character-page {
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

  .hero-grid {
    display: grid;
    grid-template-columns: 480px 1fr;
    gap: 1.5rem;
  }

  @media (max-width: 1024px) {
    .hero-grid {
      grid-template-columns: 1fr;
    }
  }

  .left-panel, .right-panel {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
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

  .specs-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .spec-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.82rem;
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
  .orange { color: #f97316; }
  .magenta { color: #ec4899; }
  .warning { color: #f59e0b; }
  .success { color: #22c55e; }
  .dim { color: #444; }

  .motto-box {
    margin-top: 1.25rem;
    border-top: 1px solid #111;
    padding-top: 0.75rem;
  }

  .motto-label {
    display: block;
    font-size: 0.68rem;
    color: #444;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    margin-bottom: 0.25rem;
  }

  .motto-text {
    font-size: 0.85rem;
    font-style: italic;
    line-height: 1.4;
  }

  .desc-text {
    font-size: 0.8rem;
    color: #888;
    line-height: 1.5;
  }

  .xp-bar-container {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .xp-bar-bg {
    height: 8px;
    background: #1c1c1c;
    border-radius: 4px;
    overflow: hidden;
  }

  .xp-bar-fill {
    height: 100%;
    border-radius: 4px;
    transition: width 0.5s ease-in-out;
  }

  .xp-labels {
    display: flex;
    justify-content: space-between;
    font-size: 0.72rem;
    color: #555;
  }

  .xp-events-list {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .xp-event-item {
    font-size: 0.8rem;
  }

  .xp-gain {
    color: #22c55e;
    font-weight: 600;
  }

  .xp-event-desc {
    color: #777;
  }

  .adventure-log-list {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    max-height: 300px;
    overflow-y: auto;
    padding-right: 0.25rem;
  }

  .log-item {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    font-size: 0.78rem;
    line-height: 1.4;
  }

  .log-bullet {
    font-size: 0.6rem;
    margin-top: 0.25rem;
  }

  .log-text {
    color: #aaa;
    flex: 1;
  }

  .log-time {
    color: #444;
    font-size: 0.7rem;
    white-space: nowrap;
  }

  .passive-lbl {
    font-size: 0.75rem;
    color: #666;
    margin-top: -0.75rem;
    margin-bottom: 1.25rem;
  }

  .powers-list {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    max-height: 520px;
    overflow-y: auto;
  }

  .power-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.4rem 0;
    border-bottom: 1px solid rgba(255,255,255,0.02);
  }

  .power-item.locked {
    opacity: 0.25;
  }

  .power-level {
    font-size: 0.7rem;
    font-weight: 600;
    min-width: 45px;
  }

  .power-name {
    font-size: 0.8rem;
    color: #aaa;
  }

  .power-item:not(.locked) .power-name {
    color: #d4d4d4;
  }

  .reflections-layout {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: 1.25rem;
    min-height: 250px;
  }

  .reflections-sidebar {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    max-height: 300px;
    overflow-y: auto;
    border-right: 1px solid #111;
    padding-right: 0.5rem;
  }

  .ref-date-btn {
    background: none;
    border: none;
    color: #555;
    font-family: inherit;
    font-size: 0.78rem;
    padding: 0.4rem 0.5rem;
    text-align: left;
    cursor: pointer;
    border-radius: 4px;
    transition: background 0.15s, color 0.15s;
  }

  .ref-date-btn:hover {
    background: rgba(255,255,255,0.02);
    color: #888;
  }

  .ref-date-btn.active {
    background: rgba(168, 85, 247, 0.12);
    color: #a855f7;
    font-weight: 600;
  }

  .ref-detail {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    max-height: 300px;
    overflow-y: auto;
  }

  .ref-section {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .ref-sub-title {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .ref-body {
    font-size: 0.82rem;
    color: #aaa;
    line-height: 1.5;
    white-space: pre-wrap;
  }

  .empty, .empty-center {
    font-size: 0.8rem;
    color: #444;
  }

  .empty-center {
    text-align: center;
    padding: 3rem 1rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
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

  .btn-primary:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: none;
    border: 1px solid #2a2a2a;
    border-radius: 5px;
    color: #666;
    font-family: inherit;
    font-size: 0.75rem;
    padding: 0.35rem 0.75rem;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s;
  }

  .btn-secondary:hover {
    border-color: #a855f7;
    color: #a855f7;
  }

  .btn-action-inline {
    background: none;
    border: 1px solid #f59e0b;
    border-radius: 3px;
    color: #f59e0b;
    font-family: inherit;
    font-size: 0.72rem;
    padding: 0.15rem 0.4rem;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 600;
  }

  .btn-action-inline:hover {
    background: rgba(245, 158, 11, 0.15);
  }

  .devices-list {
    font-style: italic;
    color: #06b6d4;
  }

  .spec-modal-content {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 0.5rem 0;
  }

  .spec-desc {
    font-size: 0.85rem;
    color: #888;
    line-height: 1.5;
  }

  .specs-options {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .spec-option-card {
    background: rgba(255,255,255,0.02);
    border: 1px solid #2a2a2a;
    border-radius: 6px;
    padding: 1rem;
    text-align: left;
    cursor: pointer;
    font-family: inherit;
    transition: border-color 0.15s, background 0.15s;
  }

  .spec-option-card:hover {
    border-color: #a855f7;
    background: rgba(168, 85, 247, 0.05);
  }

  .spec-opt-name {
    font-size: 0.9rem;
    margin-bottom: 0.25rem;
  }

  .spec-opt-desc {
    font-size: 0.78rem;
  }

  .reflection-form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .full-width {
    width: 100%;
  }
</style>
