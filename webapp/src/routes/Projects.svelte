<script>
  import { sortedProjects, addToast, projects } from '../lib/store.js';
  import { pushEvent } from '../lib/sync.js';
  import { navigate } from '../lib/router.js';
  import Modal from '../components/Modal.svelte';

  export let api;

  let showNew = false;
  let newName = '';
  let newDesc = '';
  let saving = false;

  async function createProject() {
    if (!newName.trim()) return;
    saving = true;
    const id = crypto.randomUUID();
    const now = new Date().toISOString();
    const project = {
      id,
      name: newName.trim(),
      description: newDesc.trim(),
      created_at: now,
      updated_at: now,
      archived: false,
      completed: false,
      is_shared: false,
    };

    try {
      await pushEvent(api, 'project', id, 'upsert', project);
      projects.update(m => { const n = new Map(m); n.set(id, project); return n; });
      addToast(`Campaign created: ${project.name}`, 'success');
      showNew = false;
      newName = '';
      newDesc = '';
      navigate(`/projects/${id}`);
    } catch (err) {
      addToast('Failed to create project: ' + err.message, 'error');
    } finally {
      saving = false;
    }
  }

  async function archiveProject(p) {
    const updated = { ...p, archived: true, updated_at: new Date().toISOString() };
    projects.update(m => { const n = new Map(m); n.set(p.id, updated); return n; });
    await pushEvent(api, 'project', p.id, 'upsert', updated);
    addToast(`Archived: ${p.name}`, 'info');
  }
</script>

<div class="projects-page">
  <div class="page-header">
    <h1 class="page-title">Campaigns</h1>
    <button class="btn-primary" on:click={() => showNew = true}>+ New Campaign</button>
  </div>

  <div class="project-grid">
    {#each $sortedProjects.filter(p => !p.archived && !p.completed) as project (project.id)}
      <div class="project-card">
        <div class="project-card-header">
          <h3 class="project-name">{project.name}</h3>
          {#if project.is_shared}
            <span class="badge shared">⚜ Shared</span>
          {/if}
        </div>
        {#if project.description}
          <p class="project-desc">{project.description}</p>
        {/if}
        <div class="project-actions">
          <button class="btn-link" on:click={() => navigate(`/projects/${project.id}`)}>
            Open →
          </button>
          <button class="btn-danger" on:click={() => archiveProject(project)}>
            Archive
          </button>
        </div>
      </div>
    {/each}

    {#if $sortedProjects.filter(p => !p.archived).length === 0}
      <div class="empty-state">
        <div class="empty-icon">🗺️</div>
        <p>No campaigns yet. Create your first one.</p>
      </div>
    {/if}
  </div>
</div>

<Modal open={showNew} title="New Campaign" onClose={() => showNew = false}>
  <form on:submit|preventDefault={createProject}>
    <div class="field">
      <label for="proj-name">Name</label>
      <input id="proj-name" type="text" bind:value={newName} placeholder="My Campaign" required />
    </div>
    <div class="field">
      <label for="proj-desc">Description</label>
      <textarea id="proj-desc" bind:value={newDesc} placeholder="What is this campaign about?" rows="3"></textarea>
    </div>
    <button type="submit" class="btn-primary" disabled={!newName.trim() || saving}>
      {saving ? 'Creating…' : 'Create Campaign'}
    </button>
  </form>
</Modal>

<style>
  .projects-page { padding: 2rem; }

  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 2rem;
  }

  .page-title {
    font-size: 1rem;
    font-weight: 700;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: #a855f7;
  }

  .project-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1rem;
  }

  .project-card {
    background: rgba(0,0,0,0.6);
    border: 1px solid #1c1c1c;
    border-radius: 8px;
    padding: 1.25rem;
    transition: border-color 0.15s;
  }

  .project-card:hover { border-color: #2a2a2a; }

  .project-card-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
  }

  .project-name {
    font-size: 0.9rem;
    font-weight: 600;
    color: #d4d4d4;
    flex: 1;
  }

  .badge {
    font-size: 0.7rem;
    padding: 0.15rem 0.5rem;
    border-radius: 3px;
    letter-spacing: 0.05em;
  }

  .badge.shared { background: rgba(6,182,212,0.1); border: 1px solid #06b6d4; color: #06b6d4; }

  .project-desc {
    font-size: 0.82rem;
    color: #666;
    line-height: 1.5;
    margin-bottom: 1rem;
  }

  .project-actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
  }

  .btn-primary {
    background: rgba(168,85,247,0.15);
    border: 1px solid #a855f7;
    border-radius: 5px;
    color: #a855f7;
    font-family: inherit;
    font-size: 0.8rem;
    padding: 0.45rem 1rem;
    cursor: pointer;
    letter-spacing: 0.05em;
    font-weight: 600;
    text-transform: uppercase;
    transition: background 0.15s;
  }

  .btn-primary:hover:not(:disabled) { background: rgba(168,85,247,0.25); }
  .btn-primary:disabled { opacity: 0.4; cursor: not-allowed; }

  .btn-link {
    background: none;
    border: none;
    color: #06b6d4;
    font-family: inherit;
    font-size: 0.8rem;
    cursor: pointer;
    letter-spacing: 0.03em;
  }

  .btn-danger {
    background: none;
    border: 1px solid #2a2a2a;
    border-radius: 5px;
    color: #555;
    font-family: inherit;
    font-size: 0.78rem;
    padding: 0.35rem 0.6rem;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
  }

  .btn-danger:hover { color: #ef4444; border-color: #ef4444; }

  .empty-state {
    grid-column: 1 / -1;
    text-align: center;
    padding: 3rem;
    color: #444;
  }

  .empty-icon { font-size: 2.5rem; margin-bottom: 0.75rem; }

  /* Modal form styles */
  :global(.field) { margin-bottom: 1.25rem; }
  :global(.field label) {
    display: block;
    font-size: 0.75rem;
    color: #666;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    margin-bottom: 0.4rem;
  }
  :global(.field input, .field textarea) {
    width: 100%;
    background: #050505;
    border: 1px solid #2a2a2a;
    border-radius: 6px;
    color: #d4d4d4;
    font-family: inherit;
    font-size: 0.88rem;
    padding: 0.6rem 0.85rem;
    outline: none;
    transition: border-color 0.15s;
  }
  :global(.field input:focus, .field textarea:focus) { border-color: #a855f7; }
  :global(.field textarea) { resize: vertical; }
</style>
