<script>
  import { pushEvent } from '../lib/sync.js';
  import { tasks, addToast } from '../lib/store.js';

  export let task;
  export let api;
  export let onDelete = null;

  let expanded = false;

  $: subtasks = [...$tasks.values()].filter(t => t.parent_task_id === task.id);

  const prioColor = { High: '#ef4444', Medium: '#f59e0b', Low: '#22c55e' };

  async function toggleComplete() {
    const updated = { ...task, completed: !task.completed, updated_at: new Date().toISOString() };
    tasks.update(m => { const n = new Map(m); n.set(task.id, updated); return n; });
    await pushEvent(api, 'task', task.id, 'upsert', updated);
    if (updated.completed) addToast(`Quest complete: ${task.title}`, 'success');
  }
</script>

<div class="task-item" class:completed={task.completed}>
  <div class="task-row">
    <button class="check-btn" on:click={toggleComplete} aria-label="Toggle complete">
      {task.completed ? '✓' : '○'}
    </button>

    <div class="task-content">
      <span class="task-title">{task.title}</span>
      {#if task.due_date}
        <span class="due-date">{task.due_date}</span>
      {/if}
    </div>

    <span class="prio-dot" style="background: {prioColor[task.priority] ?? '#555'}"></span>

    {#if subtasks.length > 0}
      <button class="expand-btn" on:click={() => expanded = !expanded}>
        {expanded ? '▼' : '▶'} {subtasks.length}
      </button>
    {/if}

    {#if onDelete}
      <button class="del-btn" on:click={() => onDelete(task)}>✕</button>
    {/if}
  </div>

  {#if expanded && subtasks.length > 0}
    <div class="subtasks">
      {#each subtasks as sub (sub.id)}
        <svelte:self task={sub} {api} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .task-item {
    border-bottom: 1px solid #111;
    transition: opacity 0.2s;
  }

  .task-item.completed { opacity: 0.4; }

  .task-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.6rem 0;
  }

  .check-btn {
    background: none;
    border: none;
    color: #a855f7;
    font-size: 0.9rem;
    cursor: pointer;
    font-family: inherit;
    width: 24px;
    text-align: center;
    flex-shrink: 0;
  }

  .task-content {
    flex: 1;
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
    min-width: 0;
  }

  .task-title {
    font-size: 0.88rem;
    color: #d4d4d4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .completed .task-title {
    text-decoration: line-through;
    color: #555;
  }

  .due-date {
    font-size: 0.72rem;
    color: #555;
    flex-shrink: 0;
  }

  .prio-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .expand-btn, .del-btn {
    background: none;
    border: none;
    color: #555;
    font-size: 0.75rem;
    cursor: pointer;
    font-family: inherit;
    padding: 0.1rem 0.3rem;
  }

  .expand-btn:hover, .del-btn:hover { color: #d4d4d4; }
  .del-btn:hover { color: #ef4444; }

  .subtasks {
    padding-left: 2rem;
    border-left: 1px solid #1c1c1c;
    margin-left: 1rem;
  }
</style>
