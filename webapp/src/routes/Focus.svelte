<script>
  import { onMount, onDestroy } from 'svelte';
  import { projects, addToast } from '../lib/store.js';
  import { pushEvent } from '../lib/sync.js';

  export let api;

  const durations = [
    { label: '15 min', seconds: 15 * 60 },
    { label: '25 min', seconds: 25 * 60 },
    { label: '45 min', seconds: 45 * 60 },
    { label: '90 min', seconds: 90 * 60 },
  ];

  let selectedDuration = durations[1];
  let selectedProjectId = '';
  let remaining = selectedDuration.seconds;
  let running = false;
  let completed = false;
  let timer = null;
  let startedAt = null;

  $: projectList = [...$projects.values()].filter(p => !p.archived);

  $: minutes = Math.floor(remaining / 60).toString().padStart(2, '0');
  $: seconds = (remaining % 60).toString().padStart(2, '0');
  $: progress = 1 - (remaining / selectedDuration.seconds);

  function selectDuration(d) {
    if (running) return;
    selectedDuration = d;
    remaining = d.seconds;
    completed = false;
  }

  function start() {
    running = true;
    startedAt = new Date().toISOString();
    timer = setInterval(() => {
      remaining--;
      if (remaining <= 0) {
        clearInterval(timer);
        running = false;
        completed = true;
        onComplete();
      }
    }, 1000);
  }

  function pause() {
    running = false;
    if (timer) clearInterval(timer);
  }

  function reset() {
    pause();
    completed = false;
    remaining = selectedDuration.seconds;
    startedAt = null;
  }

  async function onComplete() {
    addToast('Focus session complete! ⚔️', 'success', 6000);
    const id = crypto.randomUUID();
    const now = new Date().toISOString();
    const session = {
      id,
      project_id: selectedProjectId || null,
      duration_minutes: Math.round(selectedDuration.seconds / 60),
      started_at: startedAt,
      ended_at: now,
      completed: true,
    };
    try {
      await pushEvent(api, 'focus_session', id, 'upsert', session);
    } catch {}
  }

  onDestroy(() => {
    if (timer) clearInterval(timer);
  });

  // SVG circle
  $: circumference = 2 * Math.PI * 90;
  $: strokeDashoffset = circumference * (1 - progress);
</script>

<div class="focus-page">
  <h1 class="page-title">Focus Session</h1>

  <div class="focus-layout">
    <div class="timer-section">
      <!-- Duration picker -->
      <div class="duration-picker">
        {#each durations as d}
          <button
            class="dur-btn"
            class:active={selectedDuration === d}
            on:click={() => selectDuration(d)}
            disabled={running}
          >{d.label}</button>
        {/each}
      </div>

      <!-- Circle timer -->
      <div class="timer-circle-wrap">
        <svg class="timer-svg" viewBox="0 0 200 200">
          <circle cx="100" cy="100" r="90" class="track" />
          <circle
            cx="100" cy="100" r="90"
            class="progress-ring"
            stroke-dasharray={circumference}
            stroke-dashoffset={strokeDashoffset}
          />
        </svg>
        <div class="timer-display">
          <span class="time-value">{minutes}:{seconds}</span>
          {#if completed}
            <span class="completed-label">Complete!</span>
          {/if}
        </div>
      </div>

      <!-- Controls -->
      <div class="controls">
        {#if !running && !completed}
          <button class="ctrl-btn primary" on:click={start}>Start</button>
        {:else if running}
          <button class="ctrl-btn" on:click={pause}>Pause</button>
        {:else if completed}
          <button class="ctrl-btn primary" on:click={reset}>New Session</button>
        {/if}
        {#if running || (remaining < selectedDuration.seconds && !completed)}
          <button class="ctrl-btn" on:click={reset}>Reset</button>
        {/if}
      </div>

      <!-- Project selector -->
      {#if !running}
        <div class="project-select">
          <label for="focus-project">Campaign (optional)</label>
          <select id="focus-project" bind:value={selectedProjectId}>
            <option value="">None</option>
            {#each projectList as p (p.id)}
              <option value={p.id}>{p.name}</option>
            {/each}
          </select>
        </div>
      {/if}
    </div>

    <div class="focus-info">
      <div class="info-card">
        <h3>How Focus Works</h3>
        <p>Start a timed work session. Complete it to earn XP and water The Evergrowth.</p>
        <p>Use the 25-minute Pomodoro interval for deep work, or 90 minutes for flow state sessions.</p>
      </div>
    </div>
  </div>
</div>

<style>
  .focus-page { padding: 2rem; }

  .page-title {
    font-size: 1rem;
    font-weight: 700;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: #a855f7;
    margin-bottom: 2rem;
  }

  .focus-layout {
    display: grid;
    grid-template-columns: 1fr 300px;
    gap: 2rem;
    align-items: start;
  }

  .timer-section { display: flex; flex-direction: column; align-items: center; gap: 1.5rem; }

  .duration-picker { display: flex; gap: 0.5rem; }

  .dur-btn {
    background: none;
    border: 1px solid #2a2a2a;
    border-radius: 5px;
    color: #666;
    font-family: inherit;
    font-size: 0.8rem;
    padding: 0.4rem 0.85rem;
    cursor: pointer;
    transition: all 0.15s;
  }

  .dur-btn.active {
    border-color: #a855f7;
    color: #a855f7;
    background: rgba(168,85,247,0.1);
  }

  .dur-btn:hover:not(:disabled):not(.active) { border-color: #555; color: #d4d4d4; }
  .dur-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .timer-circle-wrap {
    position: relative;
    width: 220px;
    height: 220px;
  }

  .timer-svg {
    width: 100%;
    height: 100%;
    transform: rotate(-90deg);
  }

  .track {
    fill: none;
    stroke: #1c1c1c;
    stroke-width: 8;
  }

  .progress-ring {
    fill: none;
    stroke: #a855f7;
    stroke-width: 8;
    stroke-linecap: round;
    transition: stroke-dashoffset 0.5s linear;
  }

  .timer-display {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
  }

  .time-value {
    font-size: 2.5rem;
    font-weight: 700;
    color: #d4d4d4;
    letter-spacing: 0.05em;
    font-variant-numeric: tabular-nums;
  }

  .completed-label {
    font-size: 0.8rem;
    color: #22c55e;
    letter-spacing: 0.15em;
    text-transform: uppercase;
  }

  .controls { display: flex; gap: 0.75rem; }

  .ctrl-btn {
    background: none;
    border: 1px solid #2a2a2a;
    border-radius: 6px;
    color: #666;
    font-family: inherit;
    font-size: 0.85rem;
    padding: 0.55rem 1.5rem;
    cursor: pointer;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    transition: all 0.15s;
  }

  .ctrl-btn.primary {
    border-color: #a855f7;
    color: #a855f7;
    background: rgba(168,85,247,0.1);
  }

  .ctrl-btn:hover { border-color: #555; color: #d4d4d4; }
  .ctrl-btn.primary:hover { background: rgba(168,85,247,0.2); color: #c084fc; }

  .project-select {
    width: 100%;
    max-width: 320px;
  }

  .project-select label {
    display: block;
    font-size: 0.72rem;
    color: #555;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    margin-bottom: 0.4rem;
  }

  select {
    width: 100%;
    background: #050505;
    border: 1px solid #2a2a2a;
    border-radius: 6px;
    color: #d4d4d4;
    font-family: inherit;
    font-size: 0.85rem;
    padding: 0.5rem 0.75rem;
    outline: none;
  }

  .info-card {
    background: rgba(0,0,0,0.6);
    border: 1px solid #1c1c1c;
    border-radius: 8px;
    padding: 1.5rem;
  }

  .info-card h3 {
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    color: #555;
    margin-bottom: 0.75rem;
  }

  .info-card p {
    font-size: 0.82rem;
    color: #666;
    line-height: 1.6;
    margin-bottom: 0.75rem;
  }
</style>
