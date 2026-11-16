<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    open,
    title,
    onclose,
    above = false,
    onclear,
    children,
  }: { open: boolean; title: string; onclose: () => void; above?: boolean; onclear?: () => void; children: Snippet } = $props();
</script>

{#if open}
  <button class="dismiss" aria-label="收起选择" onclick={onclose}></button>
  <div class="popover" class:above role="dialog" aria-modal="true" aria-label={title}>
    <header>
      <b>{title}</b>
      <div class="actions">
        {#if onclear}<button class="clear" onclick={onclear}>清空</button>{/if}<button onclick={onclose}>完成</button>
      </div>
    </header>
    <div class="content">{@render children()}</div>
  </div>
{/if}

<style>
  .dismiss {
    position: fixed;
    inset: 0;
    z-index: 40;
    border: 0;
    background: #00000012;
    cursor: default;
  }
  .popover {
    position: absolute;
    top: calc(100% + 8px);
    left: 0;
    right: 0;
    z-index: 41;
    max-height: min(340px, 50dvh);
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    background: var(--card);
    color: var(--label);
    border-radius: 18px;
    box-shadow: 0 8px 28px #0002;
    padding: 12px;
  }
  .popover.above {
    top: auto;
    bottom: calc(100% + 8px);
    left: 12px;
    right: 12px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
    font-size: 15px;
    flex: none;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  header button.clear {
    color: var(--label2);
  }
  header button {
    min-height: 36px;
    padding: 6px 8px;
    border: 0;
    background: none;
    color: var(--tint);
    font: inherit;
    cursor: pointer;
  }
  .content {
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
</style>
