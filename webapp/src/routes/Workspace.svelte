<script>
  import { onMount } from 'svelte';
  import { projects, tasks, notes, codices, journalEntries, milestones, focusSessions, currentProjectId, addToast } from '../lib/store.js';
  import { pushEvent } from '../lib/sync.js';
  import { navigate } from '../lib/router.js';
  import TaskItem from '../components/TaskItem.svelte';
  import Modal from '../components/Modal.svelte';
  import MarkdownEditor from '../components/MarkdownEditor.svelte';
  import CodexTree from '../components/CodexTree.svelte';

  export let projectId;
  export let api;

  $: currentProjectId.set(projectId);
  $: project = $projects.get(projectId);

  $: projectTasks = [...$tasks.values()]
    .filter(t => t.project_id === projectId && !t.parent_task_id)
    .sort((a, b) => {
      const p = { High: 0, Medium: 1, Low: 2 };
      return (p[a.priority] ?? 1) - (p[b.priority] ?? 1);
    });

  $: projectNotes = [...$notes.values()].filter(n => n.project_id === projectId);
  $: projectCodex = [...$codices.values()].filter(c => c.project_id === projectId);
  $: projectJournals = [...$journalEntries.values()]
    .filter(e => e.project_id === projectId)
    .sort((a, b) => (b.entry_date ?? '').localeCompare(a.entry_date ?? ''));

  $: projectMilestones = [...$milestones.values()]
    .filter(m => m.project_id === projectId)
    .sort((a, b) => (a.tier ?? 1) - (b.tier ?? 1));

  // Compute Project Stats
  $: statsCompletedQuests = [...$tasks.values()].filter(t => t.project_id === projectId && t.completed).length;
  $: statsTotalQuests = [...$tasks.values()].filter(t => t.project_id === projectId).length;
  $: statsScrolls = projectNotes.length;
  $: statsJournalEntries = projectJournals.length;
  $: statsCompletedMilestones = projectMilestones.filter(m => m.completed).length;

  // Project Chronicle dynamic events list
  $: projectChronicle = (() => {
    const events = [];
    
    // Tasks completed in this project
    const pTasks = [...$tasks.values()].filter(t => t.project_id === projectId);
    for (const t of pTasks) {
      if (t.completed) {
        events.push({
          text: `Completed Quest: ${t.title}`,
          timestamp: t.updated_at || t.created_at || new Date().toISOString()
        });
      }
    }
    
    // Notes created in this project
    for (const n of projectNotes) {
      events.push({
        text: `Penned Scroll: ${n.title}`,
        timestamp: n.created_at || new Date().toISOString()
      });
    }

    // Journal entries in this project
    for (const j of projectJournals) {
      events.push({
        text: `Recorded Journal: "${(j.content || '').slice(0, 40)}..."`,
        timestamp: j.created_at || new Date().toISOString()
      });
    }

    // Focus sessions completed in this project
    const pFocus = [...$focusSessions.values()].filter(f => f.project_id === projectId);
    for (const f of pFocus) {
      events.push({
        text: `Focused for ${f.duration_mins} mins (${f.soundscape})`,
        timestamp: f.completed_at || new Date().toISOString()
      });
    }

    // Sort descending by timestamp
    events.sort((a, b) => b.timestamp.localeCompare(a.timestamp));
    return events;
  })();

  let tab = 'tasks'; // tasks | notes | journal | overview | chronicle
  let showNewTask = false;
  let showNewNote = false;
  let showNewJournal = false;
  let showNewMilestone = false;
  let selectedCodexId = null;
  let newJournalContent = '';

  // New task form
  let taskTitle = '';
  let taskPriority = 'Medium';
  let taskDue = '';

  // New note form
  let noteTitle = '';
  let noteContent = '';
  let editingNoteId = null;
  let editingContent = '';
  let editingNote = null;

  // New milestone form
  let milestoneName = '';
  let milestoneDesc = '';
  let milestoneTier = '1'; // 1 = Initiate, 2 = Veteran, 3 = Legendary
  let milestoneXp = 100;

  $: editingNote = editingNoteId ? $notes.get(editingNoteId) : null;

  async function createTask() {
    if (!taskTitle.trim()) return;
    const id = crypto.randomUUID();
    const now = new Date().toISOString();
    const task = {
      id,
      project_id: projectId,
      title: taskTitle.trim(),
      priority: taskPriority,
      due_date: taskDue || null,
      completed: false,
      created_at: now,
      updated_at: now,
    };
    tasks.update(m => { const n = new Map(m); n.set(id, task); return n; });
    await pushEvent(api, 'task', id, 'upsert', task);
    addToast('Quest added', 'success');
    taskTitle = '';
    taskDue = '';
    showNewTask = false;
  }

  async function createNote() {
    if (!noteTitle.trim()) return;
    const id = crypto.randomUUID();
    const now = new Date().toISOString();
    const note = {
      id,
      project_id: projectId,
      codex_id: selectedCodexId,
      title: noteTitle.trim(),
      markdown_content: noteContent,
      created_at: now,
      updated_at: now,
    };
    notes.update(m => { const n = new Map(m); n.set(id, note); return n; });
    await pushEvent(api, 'note', id, 'upsert', note);
    addToast('Scroll created', 'success');
    noteTitle = '';
    noteContent = '';
    showNewNote = false;
    editingNoteId = id;
    editingContent = '';
    tab = 'notes';
  }

  async function saveNote(content) {
    if (!editingNoteId) return;
    const note = $notes.get(editingNoteId);
    if (!note) return;
    const updated = { ...note, markdown_content: content, updated_at: new Date().toISOString() };
    notes.update(m => { const n = new Map(m); n.set(editingNoteId, updated); return n; });
    await pushEvent(api, 'note', editingNoteId, 'upsert', updated);
    addToast('Saved', 'success');
  }

  async function createJournalEntry() {
    if (!newJournalContent.trim()) return;
    const id = crypto.randomUUID();
    const now = new Date().toISOString();
    const entry = {
      id,
      project_id: projectId,
      entry_date: now.slice(0, 10),
      content: newJournalContent.trim(),
      created_at: now,
      visibility: 'Private',
      author_username: 'Adventurer',
    };
    journalEntries.update(m => { const n = new Map(m); n.set(id, entry); return n; });
    await pushEvent(api, 'journal_entry', id, 'create', entry);
    addToast('Entry recorded', 'success');
    newJournalContent = '';
    showNewJournal = false;
  }

  async function createMilestone() {
    if (!milestoneName.trim()) return;
    const id = crypto.randomUUID();
    const now = new Date().toISOString();
    const milestone = {
      id,
      project_id: projectId,
      name: milestoneName.trim(),
      description: milestoneDesc.trim() || null,
      completed: false,
      xp_reward: parseInt(milestoneXp, 10) || 100,
      created_at: now,
      tier: parseInt(milestoneTier, 10) || 1,
      template_id: ''
    };
    milestones.update(m => { const n = new Map(m); n.set(id, milestone); return n; });
    await pushEvent(api, 'milestone', id, 'upsert', milestone);
    addToast('Milestone created', 'success');
    milestoneName = '';
    milestoneDesc = '';
    showNewMilestone = false;
  }

  async function toggleMilestone(m) {
    const updated = { ...m, completed: !m.completed };
    milestones.update(map => { const n = new Map(map); n.set(m.id, updated); return n; });
    await pushEvent(api, 'milestone', m.id, 'upsert', updated);
    if (updated.completed) addToast(`Milestone completed: ${m.name}`, 'success');
  }

  $: filteredNotes = selectedCodexId
    ? projectNotes.filter(n => n.codex_id === selectedCodexId)
    : projectNotes;
</script>

<div class="workspace">
  {#if !project}
    <div class="not-found">
      <p>Campaign not found.</p>
      <button on:click={() => navigate('/projects')}>← Back to Campaigns</button>
    </div>
  {:else}
    <div class="workspace-header">
      <button class="back-btn" on:click={() => navigate('/projects')}>← Campaigns</button>
      <h1 class="proj-title">{project.name}</h1>
      {#if project.is_shared}
        <span class="shared-badge">⚜ Shared</span>
      {/if}
    </div>

    <div class="tab-bar">
      <button class="tab" class:active={tab === 'tasks'} on:click={() => tab = 'tasks'}>Quests</button>
      <button class="tab" class:active={tab === 'notes'} on:click={() => tab = 'notes'}>Scrolls</button>
      <button class="tab" class:active={tab === 'journal'} on:click={() => tab = 'journal'}>Journal</button>
      <button class="tab" class:active={tab === 'overview'} on:click={() => tab = 'overview'}>Overview</button>
      <button class="tab" class:active={tab === 'chronicle'} on:click={() => tab = 'chronicle'}>Chronicle</button>
    </div>

    {#if tab === 'tasks'}
      <div class="task-section">
        <div class="section-header">
          <span>{projectTasks.length} quests</span>
          <button class="btn-add" on:click={() => showNewTask = true}>+ Add Quest</button>
        </div>
        {#each projectTasks as task (task.id)}
          <TaskItem {task} {api} />
        {/each}
        {#if projectTasks.length === 0}
          <div class="empty">No quests yet. Add your first one.</div>
        {/if}
      </div>

    {:else if tab === 'notes'}
      <div class="notes-layout">
        <!-- Codex sidebar -->
        <div class="codex-sidebar">
          <div class="sidebar-header">
            <span class="sidebar-label">Codices</span>
          </div>
          <button
            class="all-notes-btn"
            class:selected={!selectedCodexId}
            on:click={() => { selectedCodexId = null; editingNoteId = null; }}
          >All scrolls</button>
          <CodexTree
            codices={projectCodex}
            selectedId={selectedCodexId}
            onSelect={id => { selectedCodexId = id; editingNoteId = null; }}
          />
        </div>

        <!-- Note list -->
        <div class="note-list">
          <div class="section-header">
            <span>{filteredNotes.length} scrolls</span>
            <button class="btn-add" on:click={() => showNewNote = true}>+ New Scroll</button>
          </div>
          {#each filteredNotes as note (note.id)}
            <button
              class="note-item"
              class:active={editingNoteId === note.id}
              on:click={() => { editingNoteId = note.id; editingContent = note.markdown_content || note.content || ''; }}
            >
              <span class="note-title">{note.title || 'Untitled'}</span>
              <span class="note-date">{note.updated_at?.slice(0, 10) ?? ''}</span>
            </button>
          {/each}
          {#if filteredNotes.length === 0}
            <div class="empty">No scrolls here.</div>
          {/if}
        </div>

        <!-- Editor -->
        <div class="note-editor">
          {#if editingNote}
            <h3 class="note-edit-title">{editingNote.title}</h3>
            <MarkdownEditor
              bind:value={editingContent}
              onSave={saveNote}
            />
          {:else}
            <div class="editor-empty">Select a scroll to edit</div>
          {/if}
        </div>
      </div>
    {:else if tab === 'journal'}
      <div class="journal-section">
        <div class="section-header">
          <span>{projectJournals.length} entries</span>
          <button class="btn-add" on:click={() => showNewJournal = true}>+ New Entry</button>
        </div>
        {#if showNewJournal}
          <div class="journal-compose">
            <textarea
              class="journal-textarea"
              bind:value={newJournalContent}
              placeholder="Write today's entry…"
              rows="5"
            ></textarea>
            <div class="journal-compose-footer">
              <button class="btn-primary" disabled={!newJournalContent.trim()} on:click={createJournalEntry}>
                Record Entry
              </button>
              <button class="btn-cancel" on:click={() => { showNewJournal = false; newJournalContent = ''; }}>
                Cancel
              </button>
            </div>
          </div>
        {/if}
        {#each projectJournals as entry (entry.id)}
          <div class="journal-entry">
            <div class="entry-header">
              <span class="entry-date">{entry.entry_date ?? entry.created_at?.slice(0, 10) ?? '—'}</span>
              <span class="entry-author">{entry.author_username ?? ''}</span>
            </div>
            <div class="entry-content">{entry.content}</div>
          </div>
        {/each}
        {#if projectJournals.length === 0 && !showNewJournal}
          <div class="empty">No journal entries yet. Record your first day.</div>
        {/if}
      </div>
    {:else if tab === 'overview'}
      <div class="overview-section">
        <div class="overview-grid">
          <!-- Project Stats Card -->
          <div class="card stats-card">
            <h3 class="section-lbl">Campaign Analytics</h3>
            <div class="stats-list">
              <div class="stat-row">
                <span class="label">Quests Completed:</span>
                <span class="val bold white">{statsCompletedQuests} / {statsTotalQuests}</span>
              </div>
              <div class="stat-row">
                <span class="label">Scrolls Written:</span>
                <span class="val bold white">{statsScrolls}</span>
              </div>
              <div class="stat-row">
                <span class="label">Journal Log Entries:</span>
                <span class="val bold white">{statsJournalEntries}</span>
              </div>
              <div class="stat-row">
                <span class="label">Milestones Achieved:</span>
                <span class="val bold success">{statsCompletedMilestones} / {projectMilestones.length}</span>
              </div>
            </div>
          </div>

          <!-- Milestones Card -->
          <div class="card milestones-card">
            <div class="section-header">
              <span class="section-lbl">Campaign Milestones</span>
              <button class="btn-add" on:click={() => showNewMilestone = true}>+ New Milestone</button>
            </div>
            <div class="milestones-list">
              {#each projectMilestones as m}
                <div class="milestone-item" class:completed={m.completed}>
                  <button class="check-btn" on:click={() => toggleMilestone(m)}>
                    {m.completed ? '✓' : '○'}
                  </button>
                  <div class="milestone-body">
                    <span class="milestone-name bold text">{m.name}</span>
                    {#if m.description}
                      <p class="milestone-desc dim">{m.description}</p>
                    {/if}
                  </div>
                  <div class="milestone-reward">
                    <span class="badge" class:completed={m.completed}>+{m.xp_reward} XP</span>
                  </div>
                </div>
              {:else}
                <div class="empty">No milestones established for this campaign. Formulate one to set milestones.</div>
              {/each}
            </div>
          </div>
        </div>
      </div>
    {:else if tab === 'chronicle'}
      <div class="project-chronicle-section">
        <h3 class="section-lbl">Campaign History & Chronicle</h3>
        <div class="chronicle-feed">
          {#each projectChronicle as entry}
            <div class="chronicle-item">
              <span class="bullet">📜</span>
              <span class="text">{entry.text}</span>
              <span class="time dim">{entry.timestamp.slice(0, 16).replace('T', ' ')}</span>
            </div>
          {:else}
            <div class="empty">No campaign history recorded. Complete quests or write scrolls to start the chronicle.</div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>

<Modal open={showNewTask} title="New Quest" onClose={() => showNewTask = false}>
  <form on:submit|preventDefault={createTask}>
    <div class="field">
      <label for="t-title">Title</label>
      <input id="t-title" type="text" bind:value={taskTitle} placeholder="Quest title" required />
    </div>
    <div class="field">
      <label for="t-prio">Priority</label>
      <select id="t-prio" bind:value={taskPriority}>
        <option>High</option>
        <option>Medium</option>
        <option>Low</option>
      </select>
    </div>
    <div class="field">
      <label for="t-due">Due Date</label>
      <input id="t-due" type="date" bind:value={taskDue} />
    </div>
    <button type="submit" class="btn-primary" disabled={!taskTitle.trim()}>Add Quest</button>
  </form>
</Modal>

<Modal open={showNewNote} title="New Scroll" onClose={() => showNewNote = false}>
  <form on:submit|preventDefault={createNote}>
    <div class="field">
      <label for="n-title">Title</label>
      <input id="n-title" type="text" bind:value={noteTitle} placeholder="Scroll title" required />
    </div>
    <button type="submit" class="btn-primary" disabled={!noteTitle.trim()}>Create Scroll</button>
  </form>
</Modal>

<Modal open={showNewMilestone} title="New Milestone" onClose={() => showNewMilestone = false}>
  <form on:submit|preventDefault={createMilestone}>
    <div class="field">
      <label for="m-name">Name</label>
      <input id="m-name" type="text" bind:value={milestoneName} placeholder="Milestone name" required />
    </div>
    <div class="field">
      <label for="m-desc">Description</label>
      <textarea id="m-desc" bind:value={milestoneDesc} placeholder="What is required to unlock this?" rows="2"></textarea>
    </div>
    <div class="field">
      <label for="m-tier">Tier</label>
      <select id="m-tier" bind:value={milestoneTier}>
        <option value="1">Initiate</option>
        <option value="2">Veteran</option>
        <option value="3">Legendary</option>
      </select>
    </div>
    <div class="field">
      <label for="m-xp">XP Reward</label>
      <input id="m-xp" type="number" bind:value={milestoneXp} min="10" max="1000" />
    </div>
    <button type="submit" class="btn-primary" disabled={!milestoneName.trim()}>Formulate Milestone</button>
  </form>
</Modal>

<style>
  .workspace { padding: 2rem; }

  .not-found {
    padding: 3rem;
    text-align: center;
    color: #555;
  }

  .workspace-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1.5rem;
    flex-wrap: wrap;
  }

  .back-btn {
    background: none;
    border: none;
    color: #555;
    font-family: inherit;
    font-size: 0.8rem;
    cursor: pointer;
    letter-spacing: 0.05em;
  }
  .back-btn:hover { color: #a855f7; }

  .proj-title {
    font-size: 1.1rem;
    font-weight: 700;
    color: #d4d4d4;
    letter-spacing: 0.05em;
  }

  .shared-badge {
    font-size: 0.72rem;
    color: #06b6d4;
    border: 1px solid #06b6d4;
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
  }

  .tab-bar {
    display: flex;
    gap: 0.25rem;
    margin-bottom: 1.5rem;
    border-bottom: 1px solid #1c1c1c;
  }

  .tab {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: #555;
    font-family: inherit;
    font-size: 0.82rem;
    padding: 0.5rem 1rem;
    cursor: pointer;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    margin-bottom: -1px;
    transition: color 0.15s, border-color 0.15s;
  }

  .tab.active { color: #a855f7; border-bottom-color: #a855f7; }

  .section-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
    font-size: 0.8rem;
    color: #555;
  }

  .btn-add {
    background: none;
    border: 1px solid #2a2a2a;
    border-radius: 4px;
    color: #888;
    font-family: inherit;
    font-size: 0.78rem;
    padding: 0.3rem 0.6rem;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
  }
  .btn-add:hover { color: #a855f7; border-color: #a855f7; }

  .empty { color: #444; font-size: 0.85rem; padding: 1rem 0; }

  .notes-layout {
    display: grid;
    grid-template-columns: 180px 220px 1fr;
    gap: 1rem;
    min-height: calc(100vh - 200px);
  }

  .codex-sidebar {
    border-right: 1px solid #1c1c1c;
    padding-right: 1rem;
  }

  .sidebar-header { margin-bottom: 0.5rem; }
  .sidebar-label {
    font-size: 0.7rem;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    color: #444;
  }

  .all-notes-btn {
    display: block;
    width: 100%;
    background: none;
    border: none;
    color: #666;
    font-family: inherit;
    font-size: 0.82rem;
    padding: 0.3rem 0.5rem;
    cursor: pointer;
    text-align: left;
    border-radius: 4px;
    margin-bottom: 0.25rem;
  }

  .all-notes-btn.selected { color: #a855f7; background: rgba(168,85,247,0.1); }
  .all-notes-btn:hover { color: #d4d4d4; }

  .note-list { border-right: 1px solid #1c1c1c; padding-right: 1rem; overflow-y: auto; }

  .note-item {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    width: 100%;
    background: none;
    border: none;
    padding: 0.5rem;
    cursor: pointer;
    border-radius: 5px;
    text-align: left;
    transition: background 0.1s;
  }

  .note-item:hover { background: rgba(168,85,247,0.05); }
  .note-item.active { background: rgba(168,85,247,0.1); }

  .note-title { font-size: 0.85rem; color: #d4d4d4; }
  .note-date { font-size: 0.7rem; color: #444; }

  .note-editor { overflow: hidden; }

  .note-edit-title {
    font-size: 0.9rem;
    color: #a855f7;
    margin-bottom: 0.75rem;
    letter-spacing: 0.05em;
  }

  .editor-empty {
    color: #333;
    font-size: 0.85rem;
    padding: 2rem;
    text-align: center;
  }

  .btn-primary {
    background: rgba(168,85,247,0.15);
    border: 1px solid #a855f7;
    border-radius: 5px;
    color: #a855f7;
    font-family: inherit;
    font-size: 0.8rem;
    padding: 0.5rem 1rem;
    cursor: pointer;
    letter-spacing: 0.05em;
    font-weight: 600;
    text-transform: uppercase;
    transition: background 0.15s;
  }
  .btn-primary:disabled { opacity: 0.4; cursor: not-allowed; }

  .journal-section { max-width: 720px; }

  .journal-compose {
    background: rgba(0,0,0,0.3);
    border: 1px solid #2a2a2a;
    border-radius: 8px;
    padding: 1rem;
    margin-bottom: 1.25rem;
  }

  .journal-textarea {
    width: 100%;
    background: #050505;
    border: 1px solid #2a2a2a;
    border-radius: 6px;
    color: #d4d4d4;
    font-family: inherit;
    font-size: 0.88rem;
    padding: 0.65rem 0.9rem;
    resize: vertical;
    outline: none;
    line-height: 1.6;
  }

  .journal-textarea:focus { border-color: #a855f7; }

  .journal-compose-footer {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.75rem;
  }

  .btn-cancel {
    background: none;
    border: 1px solid #2a2a2a;
    border-radius: 5px;
    color: #555;
    font-family: inherit;
    font-size: 0.78rem;
    padding: 0.4rem 0.8rem;
    cursor: pointer;
    transition: color 0.15s;
  }
  .btn-cancel:hover { color: #888; }

  .journal-entry {
    border-left: 2px solid #2a2a2a;
    padding: 0.75rem 1rem;
    margin-bottom: 1rem;
  }

  .entry-header {
    display: flex;
    gap: 1rem;
    margin-bottom: 0.4rem;
  }

  .entry-date {
    font-size: 0.75rem;
    color: #a855f7;
    letter-spacing: 0.05em;
  }

  .entry-author {
    font-size: 0.72rem;
    color: #444;
  }

  .entry-content {
    font-size: 0.88rem;
    color: #aaa;
    line-height: 1.6;
    white-space: pre-wrap;
  }

  :global(select) {
    width: 100%;
    background: #050505;
    border: 1px solid #2a2a2a;
    border-radius: 6px;
    color: #d4d4d4;
    font-family: inherit;
    font-size: 0.88rem;
    padding: 0.6rem 0.85rem;
    outline: none;
  }

  /* Overview & Milestones & Chronicle Styling */
  .overview-section {
    max-width: 1000px;
  }

  .overview-grid {
    display: grid;
    grid-template-columns: 350px 1fr;
    gap: 1.5rem;
  }

  @media (max-width: 768px) {
    .overview-grid {
      grid-template-columns: 1fr;
    }
  }

  .card {
    background: rgba(0, 0, 0, 0.6);
    border: 1px solid #1c1c1c;
    border-radius: 8px;
    padding: 1.25rem;
  }

  .section-lbl {
    font-size: 0.72rem;
    font-weight: 600;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #555;
    margin-bottom: 1rem;
  }

  .stats-list {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .stat-row {
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

  .val { color: #888; }
  .bold { font-weight: 600; }
  .white { color: #d4d4d4; }
  .success { color: #22c55e; }

  .milestones-list {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    max-height: 450px;
    overflow-y: auto;
  }

  .milestone-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6rem;
    border: 1px solid #111;
    border-radius: 6px;
    background: rgba(255,255,255,0.01);
  }

  .milestone-item.completed {
    opacity: 0.5;
  }

  .check-btn {
    background: none;
    border: none;
    color: #a855f7;
    font-size: 0.95rem;
    cursor: pointer;
    font-family: inherit;
    flex-shrink: 0;
    width: 24px;
    text-align: center;
  }

  .milestone-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }

  .milestone-name {
    font-size: 0.85rem;
  }

  .completed .milestone-name {
    text-decoration: line-through;
    color: #555;
  }

  .milestone-desc {
    font-size: 0.72rem;
    line-height: 1.3;
  }

  .milestone-reward {
    flex-shrink: 0;
  }

  .badge {
    font-size: 0.7rem;
    font-weight: 600;
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    background: rgba(168,85,247,0.12);
    border: 1px solid #a855f7;
    color: #a855f7;
  }

  .badge.completed {
    background: rgba(34,197,94,0.1);
    border-color: #22c55e;
    color: #22c55e;
  }

  .project-chronicle-section {
    max-width: 800px;
  }

  .chronicle-feed {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: 500px;
    overflow-y: auto;
    border-left: 1px solid #1c1c1c;
    padding-left: 1rem;
  }

  .chronicle-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-size: 0.8rem;
    padding: 0.25rem 0;
  }

  .bullet {
    font-size: 0.8rem;
  }

  .chronicle-item .text {
    flex: 1;
    color: #bbb;
  }

  .time {
    font-size: 0.7rem;
    color: #555;
  }
</style>
