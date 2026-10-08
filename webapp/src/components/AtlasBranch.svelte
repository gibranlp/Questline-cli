<script>
  export let node;
  export let children;
  export let readOnly = false;
  export let root = false;
  export let onSave;
  export let onAdd;
  export let onRemove;
  export let onColor;

  let editing = false;
  let editTitle = '';

  $: childNodes = children.get(node.id) || [];
  const colors = ['#dc4b4b','#e17837','#e1a537','#dcc341','#9bc341','#4bb969','#37b496','#41b9be','#4ba5d7','#557ddc','#6e69d2','#9b64cd','#b95fc3','#d769a0','#dc647d','#d7d7d7'];

  function beginEdit() {
    if (readOnly) return;
    editTitle = node.title;
    editing = true;
  }

  function handleEditKey(event) {
    if (event.key === 'Escape') editing = false;
    if (event.key === 'Enter' && editTitle.trim()) {
      onSave(node, { title: editTitle.trim() });
      editing = false;
    }
  }
</script>

<div class:root class="branch" style={`--wire: ${node.color == null ? '#68565a' : colors[node.color]}; --node-color: ${node.color == null ? (root ? '#30210a' : '#101012') : colors[node.color]}`}>
  <div class:marked={node.marked} class:central={root} class:connected={childNodes.length} class:colored={node.color != null} class="thought">
    <button
      class="fold"
      class:empty={!childNodes.length}
      title={childNodes.length ? (node.collapsed ? 'Unfold branches' : 'Fold branches') : 'No child branches'}
      disabled={!childNodes.length}
      on:click={() => childNodes.length && onSave(node, { collapsed: !node.collapsed })}
    >{node.collapsed && childNodes.length ? '[+]' : childNodes.length ? '[−]' : '·'}</button>
    {#if editing}
      <input bind:value={editTitle} on:keydown={handleEditKey} on:blur={() => editing = false} autofocus>
    {:else}
      <button class="label" on:dblclick={beginEdit}>{node.marked ? '✓ ' : ''}{node.title}</button>
    {/if}
    {#if !readOnly}
      <div class="actions">
        <button title="Mark" on:click={() => onSave(node, { marked: !node.marked })}>✓</button>
        <button title="Add child" on:click={() => onAdd(node.id)}>＋</button>
        <button title="Rename" on:click={beginEdit}>✎</button>
        <button title="Color branch and descendants" on:click={() => onColor(node)}>◈</button>
        <button title={node.hidden ? 'Unhide branch' : 'Hide branch'} on:click={() => onSave(node, { hidden: !node.hidden })}>H</button>
        <button class="remove" title="Remove" on:click={() => onRemove(node)}>×</button>
      </div>
    {/if}
  </div>

  {#if !node.collapsed && childNodes.length}
    <div class="children">
      {#each childNodes as child (child.id)}
        <svelte:self
          node={child}
          {children}
          {readOnly}
          {onSave}
          {onAdd}
          {onRemove}
          {onColor}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .branch{--wire:#68565a;position:relative;display:flex;align-items:center;min-width:max-content}
  .branch:not(.root)::before{content:"";position:absolute;left:-.75rem;top:50%;width:.75rem;border-top:1px solid var(--wire)}
  .thought{position:relative;z-index:1;display:flex;align-items:center;gap:.25rem;max-width:22rem;margin:.55rem .75rem .55rem 0;padding:.25rem .4rem;background:var(--node-color);color:#e4e4e7;font-family:ui-monospace,SFMono-Regular,monospace}
  .thought.connected::after{content:"";position:absolute;left:100%;top:50%;width:.75rem;border-top:1px solid var(--wire)}
  .thought.central{color:#f59e0b;font-weight:700}
  .thought.colored{color:#111}
  .thought.colored .label{color:#111}
  .thought.marked{opacity:.62}
  .thought.marked .label{text-decoration:line-through}
  .children{position:relative;display:flex;flex-direction:column;justify-content:center;border-left:1px solid var(--wire);padding:.25rem 0 .25rem .75rem}
  .fold,.label,.actions button{border:0;background:transparent;padding:.15rem;color:#f59e0b;font:inherit}
  .fold.empty{color:#71717a}.fold:disabled{cursor:default}
  .label{min-width:0;max-width:16rem;overflow:hidden;color:inherit;text-align:left;text-overflow:ellipsis;white-space:nowrap;cursor:text}
  input{min-width:7rem;max-width:12rem;border:0;border-bottom:1px solid #a855f7;background:#09090b;color:#eee;font:inherit;outline:none}
  .actions{display:none;align-items:center;gap:.05rem;margin-left:auto}
  .thought:hover .actions,.thought:focus-within .actions{display:flex}
  .actions button{color:#a855f7;cursor:pointer}.actions .remove{color:#ef4444}
</style>
