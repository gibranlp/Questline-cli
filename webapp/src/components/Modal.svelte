<script>
  export let open = false;
  export let title = '';
  export let onClose = () => {};

  function handleKeydown(e) {
    if (e.key === 'Escape') onClose();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="overlay" on:click={onClose} on:keydown={handleKeydown}>
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="modal" on:click|stopPropagation on:keydown|stopPropagation>
      <div class="modal-header">
        <h3>{title}</h3>
        <button class="close-btn" on:click={onClose}>✕</button>
      </div>
      <div class="modal-body">
        <slot />
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.7);
    z-index: 200;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal {
    background: #0d0d0d;
    border: 1px solid #2a2a2a;
    border-radius: 8px;
    width: 100%;
    max-width: 520px;
    max-height: 85vh;
    overflow-y: auto;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.5rem;
    border-bottom: 1px solid #1c1c1c;
  }

  h3 {
    font-size: 0.9rem;
    font-weight: 600;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: #a855f7;
  }

  .close-btn {
    background: none;
    border: none;
    color: #555;
    cursor: pointer;
    font-size: 0.9rem;
    padding: 0.25rem 0.5rem;
    font-family: inherit;
  }
  .close-btn:hover { color: #d4d4d4; }

  .modal-body { padding: 1.5rem; }
</style>
