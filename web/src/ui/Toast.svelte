<script lang="ts">
  import { fly } from "svelte/transition";
  import { backOut } from "svelte/easing";
  import { app } from "../lib/store.svelte";
</script>

{#if app.toast}
  {@const t = app.toast}
  <div class="toast" transition:fly={{ y: 40, duration: 380, easing: backOut }} role="status">
    <span>{t.text}</span>
    {#if t.undo}
      <button onclick={() => { t.undo?.(); app.toast = null; }}>撤销</button>
    {/if}
  </div>
{/if}

<style>
  .toast {
    position: absolute;
    left: 50%;
    bottom: calc(env(safe-area-inset-bottom) + var(--toast-offset, 96px));
    translate: -50% 0;
    z-index: 60;
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 12px 14px 12px 20px;
    border-radius: 999px;
    background: var(--toast-bg);
    color: var(--toast-fg);
    font-size: 15px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.25);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    white-space: nowrap;
  }
  button {
    border: 0;
    padding: 6px 14px;
    border-radius: 999px;
    background: var(--toast-btn);
    color: var(--toast-fg);
    font: inherit;
    font-weight: 600;
  }
</style>
