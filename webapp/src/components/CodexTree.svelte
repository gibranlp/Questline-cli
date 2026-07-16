<script>
  export let codices = [];   // all codices for the project
  export let selectedId = null;
  export let onSelect = () => {};

  function children(parentId) {
    return codices.filter(c => c.parent_codex_id === (parentId ?? null));
  }

  let collapsed = new Set();

  function toggle(id) {
    collapsed = collapsed.has(id)
      ? new Set([...collapsed].filter(x => x !== id))
      : new Set([...collapsed, id]);
  }
</script>

<div class="codex-tree">
  {#each children(null) as codex (codex.id)}
    <svelte:self
      codices={codices.filter(c => c.id !== codex.id)}
      {selectedId}
      {onSelect}
      _node={codex}
      _children={children(codex.id)}
      {collapsed}
      {toggle}
    />
  {/each}
</div>

<!-- This component self-recurses via a private interface for nested nodes -->
{#if $$props._node}
  {@const node = $$props._node}
  {@const kids = $$props._children ?? []}
  <div class="codex-node">
    <button
      class="codex-label"
      class:selected={selectedId === node.id}
      on:click={() => onSelect(node.id)}
    >
      {#if kids.length > 0}
        <button class="toggle" on:click|stopPropagation={() => $$props.toggle(node.id)} aria-label="Toggle codex">
          {$$props.collapsed?.has(node.id) ? '▶' : '▼'}
        </button>
      {:else}
        <span class="leaf">·</span>
      {/if}
      {node.name}
    </button>

    {#if kids.length > 0 && !$$props.collapsed?.has(node.id)}
      <div class="codex-children">
        {#each kids as child (child.id)}
          <svelte:self
            codices={codices}
            {selectedId}
            {onSelect}
            _node={child}
            _children={codices.filter(c => c.parent_codex_id === child.id)}
            collapsed={$$props.collapsed}
            toggle={$$props.toggle}
          />
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .codex-tree { padding: 0.5rem 0; }

  .codex-node { }

  .codex-label {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    width: 100%;
    background: none;
    border: none;
    color: #888;
    font-family: inherit;
    font-size: 0.82rem;
    padding: 0.3rem 0.5rem;
    cursor: pointer;
    text-align: left;
    border-radius: 4px;
    letter-spacing: 0.03em;
  }

  .codex-label:hover { color: #d4d4d4; background: rgba(168,85,247,0.05); }
  .codex-label.selected { color: #a855f7; background: rgba(168,85,247,0.1); }

  .toggle { color: #555; font-size: 0.65rem; width: 12px; }
  .leaf { color: #333; width: 12px; display: inline-block; text-align: center; }

  .codex-children { padding-left: 1rem; border-left: 1px solid #1c1c1c; margin-left: 0.5rem; }
</style>
