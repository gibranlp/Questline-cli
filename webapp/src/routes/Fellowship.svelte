<script>
  import { onMount, onDestroy } from 'svelte';
  import { projects, addToast } from '../lib/store.js';
  import { navigate } from '../lib/router.js';

  export let api;

  let sharedProjects = [];
  let selectedProjectId = null;
  let messages = [];
  let presence = [];
  let newMsg = '';
  let pollInterval = null;
  let lastTimestamp = null;
  let sending = false;

  $: sharedProjects = [...$projects.values()].filter(p => p.is_shared);
  $: selectedProject = sharedProjects.find(p => p.id === selectedProjectId);

  onMount(() => {
    if (sharedProjects.length > 0) {
      selectProject(sharedProjects[0].id);
    }
  });

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval);
  });

  async function selectProject(id) {
    selectedProjectId = id;
    messages = [];
    lastTimestamp = null;
    if (pollInterval) clearInterval(pollInterval);

    await loadMessages();
    await loadPresence();

    pollInterval = setInterval(async () => {
      if (!document.hidden) {
        await loadMessages();
        await loadPresence();
      }
    }, 5000);
  }

  async function loadMessages() {
    if (!selectedProjectId) return;
    try {
      const params = { project_id: selectedProjectId };
      if (lastTimestamp) params.since = lastTimestamp;
      const data = await api.get('chronicle/messages', params);
      if (Array.isArray(data) && data.length > 0) {
        if (lastTimestamp) {
          messages = [...messages, ...data];
        } else {
          messages = data;
        }
        lastTimestamp = data[data.length - 1].timestamp;
      }
    } catch (err) {
      console.error('[fellowship] load messages:', err);
    }
  }

  async function loadPresence() {
    if (!selectedProjectId) return;
    try {
      presence = await api.get('chronicle/presence', { project_id: selectedProjectId });
    } catch {}
  }

  async function sendMessage() {
    if (!newMsg.trim() || !selectedProjectId) return;
    sending = true;
    try {
      await api.post('chronicle/message', {
        project_id: selectedProjectId,
        content: newMsg.trim(),
        message_type: 'text',
      });
      newMsg = '';
      await loadMessages();
    } catch (err) {
      addToast('Failed to send: ' + err.message, 'error');
    } finally {
      sending = false;
    }
  }

  function handleKeydown(e) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  }

  function formatTime(ts) {
    if (!ts) return '';
    return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }
</script>

<div class="fellowship-page">
  {#if sharedProjects.length === 0}
    <div class="empty-state">
      <div class="icon">⚜️</div>
      <h2>No Shared Campaigns</h2>
      <p>Share a campaign with companions to start collaborating.</p>
      <button on:click={() => navigate('/projects')}>Go to Campaigns →</button>
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
            <span class="chat-title">{selectedProject.name}</span>
            <span class="online-count">{presence.filter(p => p.is_online == 1).length} online</span>
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

      <!-- Presence sidebar -->
      <div class="presence-panel">
        <div class="panel-label">Companions</div>
        {#each presence as p (p.user_identity)}
          <div class="companion">
            <span class="online-dot" class:online={p.is_online == 1}></span>
            <span class="companion-name">{p.user_username ?? 'Companion'}</span>
            <span class="companion-role dim">{p.role}</span>
          </div>
        {/each}
        {#if presence.length === 0}
          <div class="empty dim">No companions yet</div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .fellowship-page { padding: 2rem; height: calc(100vh - 4rem); }

  .empty-state {
    text-align: center;
    padding: 4rem;
    color: #555;
  }
  .empty-state .icon { font-size: 2.5rem; margin-bottom: 1rem; }
  .empty-state h2 { color: #d4d4d4; font-size: 1rem; margin-bottom: 0.5rem; }
  .empty-state button {
    background: none; border: 1px solid #a855f7; border-radius: 5px;
    color: #a855f7; font-family: inherit; font-size: 0.85rem;
    padding: 0.5rem 1rem; cursor: pointer; margin-top: 1rem;
  }

  .fellowship-layout {
    display: grid;
    grid-template-columns: 200px 1fr 180px;
    gap: 1rem;
    height: 100%;
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

  .msg { }

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

  .online-dot {
    width: 6px; height: 6px;
    border-radius: 50%;
    background: #333;
    flex-shrink: 0;
  }
  .online-dot.online { background: #22c55e; }

  .companion-name { color: #888; flex: 1; }
  .companion-role { font-size: 0.68rem; }
  .dim { color: #444; }
  .empty { font-size: 0.82rem; }
</style>
