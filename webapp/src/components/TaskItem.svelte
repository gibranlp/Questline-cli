<script>
  import { pushEvent } from '../lib/sync.js';
  import { tasks, taskAssignments, identity, userStats, addToast, applySyncEvent } from '../lib/store.js';
  import { assignmentId, assignmentsForTask } from '../lib/collaboration.js';

  export let task;
  export let api;
  export let onDelete = null;
  export let members = [];
  export let canAssign = false;

  let expanded = false;
  let assigning = false;

  $: subtasks = [...$tasks.values()].filter(t => t.parent_task_id === task.id);
  $: assignees = assignmentsForTask($taskAssignments, task.id);

  const prioColor = { High: '#ef4444', Medium: '#f59e0b', Low: '#22c55e' };

  async function toggleComplete() {
    const updated = { ...task, completed: !task.completed, updated_at: new Date().toISOString() };
    tasks.update(m => { const n = new Map(m); n.set(task.id, updated); return n; });
    await pushEvent(api, 'task', task.id, 'upsert', updated);
    if (updated.completed) addToast(`Quest complete: ${task.title}`, 'success');
  }

  async function toggleAssignee(member) {
    const userIdentity = String(member.identity || '').toLowerCase();
    if (!userIdentity || !task.project_id) return;
    const id = assignmentId(task.id, userIdentity);
    const existing = $taskAssignments.has(id);
    const payload = {
      task_id: task.id,
      project_id: task.project_id,
      user_identity: userIdentity,
      user_username: member.username || `${userIdentity.slice(0, 8)}…`,
      assigned_by_identity: $identity?.public_key || null,
      assigned_by_username: $userStats?.username || 'Companion',
      assigned_at: new Date().toISOString(),
    };
    const operation = existing ? 'delete' : 'upsert';
    applySyncEvent({ entity_type: 'task_assignment', entity_id: id, operation, content: JSON.stringify(payload) });
    try {
      const result = await pushEvent(api, 'task_assignment', id, operation, payload);
      if (!result?.queued) addToast(existing ? 'Companion unassigned' : `Quest assigned to ${payload.user_username}`, 'success');
    } catch (err) {
      // The sync engine keeps optimistic offline changes in IndexedDB.
      addToast('Assignment saved locally and will retry when sync is available', 'warning');
    }
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
      {#if assignees.length > 0}
        <span class="assignees" aria-label="Assigned companions">
          {#each assignees as assignee (assignee.id)}
            <span class="assignee-chip" title={assignee.user_identity}>{assignee.user_username || 'Companion'}</span>
          {/each}
        </span>
      {/if}
    </div>

    <span class="prio-dot" style="background: {prioColor[task.priority] ?? '#555'}"></span>

    {#if subtasks.length > 0}
      <button class="expand-btn" on:click={() => expanded = !expanded}>
        {expanded ? '▼' : '▶'} {subtasks.length}
      </button>
    {/if}

    {#if canAssign && members.length > 0}
      <div class="assign-wrap">
        <button class="assign-btn" aria-expanded={assigning} on:click={() => assigning = !assigning}>⚜ Assign</button>
        {#if assigning}
          <div class="assign-menu">
            <div class="assign-label">Assign companions</div>
            {#each members.filter(m => m.role !== 'Observer') as member (member.identity)}
              <label class="member-option">
                <input
                  type="checkbox"
                  checked={$taskAssignments.has(assignmentId(task.id, member.identity))}
                  on:change={() => toggleAssignee(member)}
                />
                <span>{member.username || `${member.identity.slice(0, 8)}…`}</span>
                <small>{member.role}</small>
              </label>
            {/each}
          </div>
        {/if}
      </div>
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

  .assignees { display: flex; flex-wrap: wrap; gap: 0.25rem; }
  .assignee-chip {
    border: 1px solid color-mix(in srgb, var(--accent, #a855f7) 35%, transparent);
    border-radius: 999px;
    color: var(--accent, #a855f7);
    font-size: 0.62rem;
    padding: 0.08rem 0.4rem;
    white-space: nowrap;
  }

  .assign-wrap { position: relative; }
  .assign-btn {
    background: none; border: 1px solid #2a2a2a; border-radius: 4px; color: #777;
    cursor: pointer; font: inherit; font-size: 0.68rem; padding: 0.2rem 0.45rem;
  }
  .assign-btn:hover { border-color: var(--accent, #a855f7); color: var(--accent, #a855f7); }
  .assign-menu {
    position: absolute; right: 0; top: calc(100% + 0.4rem); z-index: 30; width: 240px;
    background: #0b0b0b; border: 1px solid #2a2a2a; border-radius: 6px;
    box-shadow: 0 12px 30px rgba(0,0,0,0.55); padding: 0.5rem;
  }
  .assign-label { color: #555; font-size: 0.65rem; letter-spacing: 0.1em; padding: 0.25rem; text-transform: uppercase; }
  .member-option { display: grid; grid-template-columns: 20px 1fr auto; align-items: center; gap: 0.35rem; padding: 0.45rem 0.25rem; cursor: pointer; }
  .member-option:hover { background: rgba(168,85,247,0.08); }
  .member-option span { color: #d4d4d4; font-size: 0.75rem; overflow: hidden; text-overflow: ellipsis; }
  .member-option small { color: #555; font-size: 0.6rem; }

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
