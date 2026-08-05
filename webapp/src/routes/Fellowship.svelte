<script>
  import { onMount, onDestroy } from 'svelte';
  import { projects, chronicleMessages, identity, userStats, addToast, applySyncEvent } from '../lib/store.js';
  import { pushEvent, pullProjectSync } from '../lib/sync.js';
  import {
    listPendingInvitations, acceptInvitation, sendInvitation,
    listRotationMembers, removeMember,
  } from '../lib/fellowship.js';

  export let api;

  let selectedProjectId = null;
  let presence = [];
  let pending = [];
  let newMsg = '';
  let pollInterval = null;
  let sending = false;

  // Invite form (add a companion to the selected shared project)
  let showInvite = false;
  let inviteIdentity = '';
  let inviteRole = 'Companion';
  let inviting = false;

  // First-share form (share a not-yet-shared local campaign)
  let showShare = false;
  let shareProjectId = '';
  let shareIdentity = '';
  let shareRole = 'Companion';
  let sharing = false;
  $: unsharedProjects = [...$projects.values()].filter(p => !p.is_shared);

  $: sharedProjects = [...$projects.values()].filter(p => p.is_shared);
  $: selectedProject = sharedProjects.find(p => p.id === selectedProjectId);
  $: messages = selectedProjectId ? ($chronicleMessages.get(selectedProjectId) || []) : [];
  $: myIdentity = ($identity?.public_key || '').toLowerCase();
  $: isOwner = presence.some(m => (m.identity || '').toLowerCase() === myIdentity && m.role === 'Owner');

  onMount(async () => {
    await refreshPending();
    if (sharedProjects.length > 0) selectProject(sharedProjects[0].id);
  });

  onDestroy(() => { if (pollInterval) clearInterval(pollInterval); });

  async function refreshPending() {
    if (!$identity) return;
    pending = await listPendingInvitations($identity);
  }

  async function selectProject(id) {
    selectedProjectId = id;
    if (pollInterval) clearInterval(pollInterval);
    await loadPresence();
    pollInterval = setInterval(() => { if (!document.hidden) loadPresence(); }, 10000);
  }

  async function loadPresence() {
    if (!selectedProjectId || !$identity) return;
    presence = await listRotationMembers($identity, selectedProjectId);
  }

  async function acceptInvite(inv) {
    try {
      const projectId = await acceptInvitation($identity, inv, $userStats?.username || '');
      addToast(`Joined "${inv.project_name}"`, 'success');
      await refreshPending();
      await pullProjectSync($identity);   // fetch the shared campaign's content
      selectProject(projectId);
    } catch (err) {
      addToast('Could not accept invitation: ' + err.message, 'error');
    }
  }

  async function submitInvite() {
    if (!selectedProject) return;
    inviting = true;
    try {
      await sendInvitation($identity, selectedProject, inviteIdentity.trim(), inviteRole);
      addToast('Invitation sent', 'success');
      showInvite = false;
      inviteIdentity = '';
      await loadPresence();
    } catch (err) {
      addToast('Invite failed: ' + err.message, 'error');
    } finally {
      inviting = false;
    }
  }

  async function submitShare() {
    const proj = $projects.get(shareProjectId);
    if (!proj) { addToast('Pick a campaign to share', 'error'); return; }
    sharing = true;
    try {
      await sendInvitation($identity, proj, shareIdentity.trim(), shareRole);
      // Propagate is_shared=true as a project-scoped event so it reaches members
      // (including via accept-backfill). The key now exists, so this routes encrypted.
      await pushEvent(api, 'project', proj.id, 'upsert', { ...proj, is_shared: true });
      addToast(`Shared "${proj.name}" and invited companion`, 'success');
      showShare = false;
      shareIdentity = '';
      shareProjectId = '';
      selectProject(proj.id);
    } catch (err) {
      addToast('Share failed: ' + err.message, 'error');
    } finally {
      sharing = false;
    }
  }

  async function removeCompanion(memberIdentity) {
    if (!selectedProject) return;
    if (!confirm('Remove this companion? The project key will be rotated.')) return;
    try {
      await removeMember($identity, selectedProject.id, memberIdentity);
      addToast('Companion removed and key rotated', 'success');
      await loadPresence();
    } catch (err) {
      addToast('Removal failed: ' + err.message, 'error');
    }
  }

  async function sendMessage() {
    if (!newMsg.trim() || !selectedProjectId) return;
    sending = true;
    const id = crypto.randomUUID();
    const payload = {
      id,
      project_id: selectedProjectId,
      sender_identity: $identity.public_key,
      sender_username: $userStats?.username || 'Companion',
      content: newMsg.trim(),
      message_type: 'text',
      timestamp: new Date().toISOString(),
    };
    try {
      await pushEvent(api, 'chronicle_message', id, 'upsert', payload);
      applySyncEvent({ entity_type: 'chronicle_message', entity_id: id, operation: 'upsert', content: JSON.stringify(payload) });
      newMsg = '';
    } catch (err) {
      addToast('Failed to send: ' + err.message, 'error');
    } finally {
      sending = false;
    }
  }

  function handleKeydown(e) {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); sendMessage(); }
  }

  function formatTime(ts) {
    if (!ts) return '';
    return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  // rotation-members returns identity + role but no display name.
  function memberLabel(m) {
    if (m.username) return m.username;
    const id = m.identity || '';
    return id ? `${id.slice(0, 8)}…` : 'Companion';
  }
</script>

<div class="fellowship-page">
  {#if pending.length > 0}
    <div class="invites">
      <div class="panel-label">Pending Invitations</div>
      {#each pending as inv (inv.id)}
        <div class="invite-row">
          <span class="invite-name">{inv.project_name}</span>
          <span class="invite-from dim">from {inv.inviter_username ?? 'a companion'} · {inv.role}</span>
          <button class="accept-btn" on:click={() => acceptInvite(inv)}>Join</button>
        </div>
      {/each}
    </div>
  {/if}

  <div class="share-bar">
    {#if unsharedProjects.length > 0}
      <button class="share-toggle" on:click={() => showShare = !showShare}>
        {showShare ? 'Cancel' : '🔐 Share a campaign'}
      </button>
    {/if}
    {#if showShare}
      <div class="share-form">
        <select class="invite-input" bind:value={shareProjectId}>
          <option value="" disabled selected>Choose a campaign…</option>
          {#each unsharedProjects as p (p.id)}
            <option value={p.id}>{p.name}</option>
          {/each}
        </select>
        <input class="invite-input" bind:value={shareIdentity} placeholder="Companion identity (64 hex)" spellcheck="false" />
        <select class="invite-input" bind:value={shareRole}>
          <option>Companion</option>
          <option>Steward</option>
          <option>Observer</option>
        </select>
        <button class="accept-btn" on:click={submitShare} disabled={sharing || !shareProjectId || !shareIdentity.trim()}>
          {sharing ? 'Sharing…' : 'Share & invite'}
        </button>
      </div>
    {/if}
  </div>

  {#if sharedProjects.length === 0}
    <div class="empty-state">
      <div class="icon">⚜️</div>
      <h2>No Shared Campaigns</h2>
      <p>Share a campaign with companions to start an end-to-end encrypted Fellowship, or accept an invitation above.</p>
    </div>
  {:else}
    <div class="fellowship-layout">
      <!-- Project list -->
      <div class="project-list">
        <div class="panel-label">Shared Campaigns</div>
        {#each sharedProjects as p (p.id)}
          <button
            class="proj-btn"
            class:active={selectedProjectId === p.id}
            on:click={() => selectProject(p.id)}
          >{p.name}</button>
        {/each}
      </div>

      <!-- Chronicle chat -->
      <div class="chat-panel">
        {#if selectedProject}
          <div class="chat-header">
            <span class="chat-title">🔐 {selectedProject.name}</span>
            <span class="online-count">{presence.length} companion{presence.length === 1 ? '' : 's'}</span>
          </div>

          <div class="messages">
            {#each messages as msg (msg.id)}
              <div class="msg">
                <span class="msg-sender">{msg.sender_username ?? 'Companion'}</span>
                <span class="msg-time">{formatTime(msg.timestamp)}</span>
                <div class="msg-content">{msg.content}</div>
              </div>
            {/each}
            {#if messages.length === 0}
              <div class="empty-chat">The chronicle is empty. Say hello.</div>
            {/if}
          </div>

          <div class="chat-input-row">
            <textarea
              class="chat-input"
              bind:value={newMsg}
              placeholder="Message the chronicle…"
              rows="2"
              on:keydown={handleKeydown}
            ></textarea>
            <button class="send-btn" on:click={sendMessage} disabled={!newMsg.trim() || sending}>
              {sending ? '…' : 'Send'}
            </button>
          </div>
        {/if}
      </div>

      <!-- Companion sidebar -->
      <div class="presence-panel">
        <div class="panel-label">Companions</div>
        {#each presence as m (m.identity)}
          <div class="companion">
            <span class="companion-name">{memberLabel(m)}</span>
            <span class="companion-role dim">{m.role}</span>
            {#if isOwner && (m.identity || '').toLowerCase() !== myIdentity}
              <button class="remove-btn" title="Remove companion" on:click={() => removeCompanion(m.identity)}>✕</button>
            {/if}
          </div>
        {/each}
        {#if presence.length === 0}
          <div class="empty dim">No companions yet</div>
        {/if}

        <button class="invite-toggle" on:click={() => showInvite = !showInvite}>
          {showInvite ? 'Cancel' : '+ Invite companion'}
        </button>
        {#if showInvite}
          <div class="invite-form">
            <input
              class="invite-input"
              bind:value={inviteIdentity}
              placeholder="Companion identity (64 hex)"
              spellcheck="false"
            />
            <select class="invite-input" bind:value={inviteRole}>
              <option>Companion</option>
              <option>Steward</option>
              <option>Observer</option>
            </select>
            <button class="accept-btn" on:click={submitInvite} disabled={inviting || !inviteIdentity.trim()}>
              {inviting ? 'Sending…' : 'Send invite'}
            </button>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .fellowship-page { padding: 2rem; height: calc(100vh - 4rem); display: flex; flex-direction: column; gap: 1rem; }

  .empty-state {
    text-align: center;
    padding: 4rem;
    color: #555;
  }
  .empty-state .icon { font-size: 2.5rem; margin-bottom: 1rem; }
  .empty-state h2 { color: #d4d4d4; font-size: 1rem; margin-bottom: 0.5rem; }

  .invites {
    background: rgba(168,85,247,0.06);
    border: 1px solid rgba(168,85,247,0.25);
    border-radius: 8px;
    padding: 0.75rem 1rem;
  }
  .invite-row {
    display: flex; align-items: center; gap: 0.75rem;
    padding: 0.35rem 0;
  }
  .invite-name { color: #d4d4d4; font-size: 0.9rem; font-weight: 600; }
  .invite-from { font-size: 0.75rem; flex: 1; }
  .accept-btn {
    background: rgba(168,85,247,0.15); border: 1px solid #a855f7; border-radius: 5px;
    color: #a855f7; font-family: inherit; font-size: 0.78rem; padding: 0.35rem 0.9rem; cursor: pointer;
  }
  .accept-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .accept-btn:hover:not(:disabled) { background: rgba(168,85,247,0.25); }

  .fellowship-layout {
    display: grid;
    grid-template-columns: 200px 1fr 200px;
    gap: 1rem;
    flex: 1;
    min-height: 0;
  }

  .project-list, .presence-panel {
    background: rgba(0,0,0,0.4);
    border: 1px solid #1c1c1c;
    border-radius: 8px;
    padding: 1rem;
    overflow-y: auto;
  }

  .panel-label {
    font-size: 0.68rem;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #444;
    margin-bottom: 0.75rem;
  }

  .proj-btn {
    display: block;
    width: 100%;
    background: none;
    border: none;
    color: #666;
    font-family: inherit;
    font-size: 0.82rem;
    padding: 0.4rem 0.5rem;
    cursor: pointer;
    text-align: left;
    border-radius: 4px;
    margin-bottom: 2px;
  }
  .proj-btn:hover { color: #d4d4d4; background: rgba(255,255,255,0.03); }
  .proj-btn.active { color: #a855f7; background: rgba(168,85,247,0.1); }

  .chat-panel {
    display: flex;
    flex-direction: column;
    background: rgba(0,0,0,0.4);
    border: 1px solid #1c1c1c;
    border-radius: 8px;
    overflow: hidden;
    min-height: 0;
  }

  .chat-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #1c1c1c;
  }

  .chat-title {
    font-size: 0.85rem;
    font-weight: 600;
    color: #d4d4d4;
    letter-spacing: 0.05em;
  }

  .online-count { font-size: 0.72rem; color: #22c55e; }

  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .msg-sender {
    font-size: 0.75rem;
    color: #a855f7;
    font-weight: 600;
    letter-spacing: 0.05em;
  }

  .msg-time {
    font-size: 0.68rem;
    color: #444;
    margin-left: 0.5rem;
  }

  .msg-content {
    font-size: 0.88rem;
    color: #d4d4d4;
    line-height: 1.5;
    margin-top: 0.15rem;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .empty-chat { color: #333; font-size: 0.85rem; text-align: center; padding: 2rem; }

  .chat-input-row {
    display: flex;
    gap: 0.5rem;
    padding: 0.75rem;
    border-top: 1px solid #1c1c1c;
  }

  .chat-input {
    flex: 1;
    background: #050505;
    border: 1px solid #2a2a2a;
    border-radius: 6px;
    color: #d4d4d4;
    font-family: inherit;
    font-size: 0.85rem;
    padding: 0.5rem 0.75rem;
    outline: none;
    resize: none;
    line-height: 1.4;
  }

  .chat-input:focus { border-color: #a855f7; }

  .send-btn {
    background: rgba(168,85,247,0.15);
    border: 1px solid #a855f7;
    border-radius: 5px;
    color: #a855f7;
    font-family: inherit;
    font-size: 0.78rem;
    padding: 0.5rem 1rem;
    cursor: pointer;
    align-self: flex-end;
  }
  .send-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .send-btn:hover:not(:disabled) { background: rgba(168,85,247,0.25); }

  .companion {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.3rem 0;
    font-size: 0.8rem;
  }

  .companion-name { color: #888; flex: 1; }
  .companion-role { font-size: 0.68rem; }
  .remove-btn {
    background: none; border: none; color: #555; cursor: pointer;
    font-size: 0.75rem; padding: 0 0.2rem;
  }
  .remove-btn:hover { color: #ef4444; }
  .dim { color: #444; }
  .empty { font-size: 0.82rem; }

  .invite-toggle {
    display: block; width: 100%; margin-top: 1rem;
    background: none; border: 1px dashed #333; border-radius: 5px;
    color: #666; font-family: inherit; font-size: 0.76rem;
    padding: 0.4rem; cursor: pointer;
  }
  .invite-toggle:hover { color: #a855f7; border-color: #a855f7; }

  .invite-form { display: flex; flex-direction: column; gap: 0.4rem; margin-top: 0.5rem; }
  .invite-input {
    background: #050505; border: 1px solid #2a2a2a; border-radius: 5px;
    color: #d4d4d4; font-family: inherit; font-size: 0.76rem; padding: 0.4rem 0.5rem; outline: none;
  }
  .invite-input:focus { border-color: #a855f7; }

  .share-bar { display: flex; flex-direction: column; gap: 0.5rem; }
  .share-toggle {
    align-self: flex-start;
    background: rgba(168,85,247,0.08); border: 1px solid rgba(168,85,247,0.3); border-radius: 6px;
    color: #a855f7; font-family: inherit; font-size: 0.8rem; padding: 0.45rem 0.9rem; cursor: pointer;
  }
  .share-toggle:hover { background: rgba(168,85,247,0.18); }
  .share-form {
    display: flex; flex-wrap: wrap; gap: 0.5rem; align-items: center;
    background: rgba(0,0,0,0.3); border: 1px solid #1c1c1c; border-radius: 8px; padding: 0.75rem;
  }
  .share-form .invite-input { flex: 1; min-width: 160px; }
</style>
