<script>
  import { marked } from 'marked';
  import { onMount } from 'svelte';

  export let value = '';
  export let placeholder = 'Write in markdown...';
  export let onSave = null;

  let preview = false;
  let rendered = '';

  $: rendered = marked.parse(value || '');

  function handleKeydown(e) {
    // Ctrl+S or Cmd+S saves
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      e.preventDefault();
      if (onSave) onSave(value);
    }
    // Tab inserts 2 spaces
    if (e.key === 'Tab') {
      e.preventDefault();
      const el = e.target;
      const start = el.selectionStart;
      const end = el.selectionEnd;
      value = value.slice(0, start) + '  ' + value.slice(end);
      setTimeout(() => {
        el.selectionStart = el.selectionEnd = start + 2;
      });
    }
  }
</script>

<div class="editor-wrap">
  <div class="editor-toolbar">
    <button class:active={!preview} on:click={() => preview = false}>Edit</button>
    <button class:active={preview} on:click={() => preview = true}>Preview</button>
    {#if onSave}
      <button class="save-btn" on:click={() => onSave(value)}>Save ⌘S</button>
    {/if}
  </div>

  {#if preview}
    <div class="preview" >
      {@html rendered}
    </div>
  {:else}
    <textarea
      class="editor"
      bind:value
      {placeholder}
      on:keydown={handleKeydown}
      rows="20"
    ></textarea>
  {/if}
</div>

<style>
  .editor-wrap {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .editor-toolbar {
    display: flex;
    gap: 0.5rem;
    padding: 0.5rem 0;
    border-bottom: 1px solid #1c1c1c;
    margin-bottom: 0.75rem;
  }

  .editor-toolbar button {
    background: none;
    border: 1px solid transparent;
    color: #666;
    padding: 0.25rem 0.75rem;
    border-radius: 4px;
    font-family: inherit;
    font-size: 0.8rem;
    letter-spacing: 0.05em;
    cursor: pointer;
    text-transform: uppercase;
  }

  .editor-toolbar button.active {
    border-color: #a855f7;
    color: #a855f7;
  }

  .save-btn {
    margin-left: auto;
    border-color: #2a2a2a !important;
    color: #888 !important;
  }

  .save-btn:hover { color: #d4d4d4 !important; }

  .editor {
    width: 100%;
    flex: 1;
    background: #050505;
    border: 1px solid #1c1c1c;
    border-radius: 6px;
    color: #d4d4d4;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.88rem;
    line-height: 1.7;
    padding: 1rem;
    resize: none;
    outline: none;
    min-height: 300px;
  }

  .editor:focus { border-color: #a855f7; }

  .preview {
    background: #050505;
    border: 1px solid #1c1c1c;
    border-radius: 6px;
    padding: 1rem 1.5rem;
    min-height: 300px;
    color: #d4d4d4;
    line-height: 1.7;
    font-size: 0.9rem;
  }

  :global(.preview h1, .preview h2, .preview h3) { color: #a855f7; margin: 1rem 0 0.5rem; }
  :global(.preview code) { background: #1c1c1c; padding: 0.1rem 0.3rem; border-radius: 3px; font-size: 0.85em; }
  :global(.preview pre) { background: #0d0d0d; border: 1px solid #1c1c1c; padding: 1rem; border-radius: 6px; overflow-x: auto; }
  :global(.preview blockquote) { border-left: 3px solid #a855f7; padding-left: 1rem; color: #888; }
  :global(.preview ul, .preview ol) { padding-left: 1.5rem; }
  :global(.preview a) { color: #06b6d4; }
</style>
