<script module lang="ts">
  export interface Action {
    label: string;
    danger?: boolean;
    run: () => void;
  }
</script>

<script lang="ts">
  // 底部操作菜单：一组按钮 + 单独的取消。选了就关，再执行。
  import { fade, fly } from "svelte/transition";
  import { quintOut } from "svelte/easing";

  let { open, title, actions, oncancel }: { open: boolean; title: string; actions: Action[]; oncancel: () => void } = $props();
</script>

{#if open}
  <div class="backdrop" transition:fade={{ duration: 220 }} onclick={oncancel} role="presentation"></div>
  <div class="as" transition:fly={{ y: 300, duration: 380, easing: quintOut, opacity: 1 }} role="alertdialog">
    <div class="group">
      <p>{title}</p>
      {#each actions as a (a.label)}
        <button class:danger={a.danger} onclick={() => { oncancel(); a.run(); }}>{a.label}</button>
      {/each}
    </div>
    <button class="cancel" onclick={oncancel}>取消</button>
  </div>
{/if}

<style>
  .backdrop {
    position: absolute;
    inset: 0;
    z-index: 50;
    background: var(--scrim);
  }
  .as {
    position: absolute;
    left: 8px;
    right: 8px;
    bottom: max(env(safe-area-inset-bottom), 8px);
    z-index: 51;
    display: grid;
    gap: 8px;
  }
  .group,
  .cancel {
    border-radius: var(--radius-alert);
    overflow: hidden;
    background: var(--alert-bg);
    backdrop-filter: saturate(180%) blur(30px);
    -webkit-backdrop-filter: saturate(180%) blur(30px);
  }
  button + button {
    border-top: 0.5px solid var(--sep);
  }
  p {
    margin: 0;
    padding: 14px 16px;
    text-align: center;
    font-size: 13px;
    color: var(--label2);
    border-bottom: 0.5px solid var(--sep);
  }
  button {
    width: 100%;
    height: 57px;
    border: 0;
    background: transparent;
    font: inherit;
    font-size: 20px;
    color: var(--tint);
  }
  button:active {
    background: var(--press);
  }
  .danger {
    color: var(--red);
  }
  .cancel {
    font-weight: 600;
  }
</style>
