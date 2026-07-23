<script>
  import { toasts } from '../lib/store.js';
</script>

<div class="toast-container">
  {#each $toasts as toast (toast.id)}
    <div class="toast toast-{toast.type}">
      {toast.msg}
    </div>
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    bottom: 1.5rem;
    right: 1.5rem;
    z-index: 999;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    pointer-events: none;
  }

  .toast {
    padding: 0.75rem 1.25rem;
    border-radius: 6px;
    font-size: 0.85rem;
    letter-spacing: 0.03em;
    border: 1px solid;
    animation: slideIn 0.2s ease;
    max-width: 340px;
  }

  @keyframes slideIn {
    from { opacity: 0; transform: translateX(20px); }
    to   { opacity: 1; transform: translateX(0); }
  }

  .toast-info    { background: #0d1117; border-color: #30363d; color: #d4d4d4; }
  .toast-success { background: #0d2118; border-color: #22c55e; color: #4ade80; }
  .toast-warning { background: #1a1200; border-color: #f59e0b; color: #fbbf24; }
  .toast-error   { background: #1a0a0a; border-color: #ef4444; color: #f87171; }
</style>
