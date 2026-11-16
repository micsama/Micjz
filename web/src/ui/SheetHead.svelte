<script lang="ts">
  // 底部面板的标题栏：左边关闭，中间图标和标题，右边可选的一个动作（完成、清除）。
  import Icon, { type IconName } from "./Icon.svelte";

  let { icon, title, onclose, action }: { icon: IconName; title: string; onclose: () => void; action?: { label: string; run: () => void; quiet?: boolean } } =
    $props();
</script>

<header>
  <button class="round" onclick={onclose} aria-label="关闭"><Icon name="xmark" size={14} weight={2.6} /></button>
  <h2><span class="glyph"><Icon name={icon} size={19} weight={2.2} /></span>{title}</h2>
  {#if action}
    <button class="link" class:quiet={action.quiet} onclick={action.run}>{action.label}</button>
  {:else}
    <span></span>
  {/if}
</header>

<style>
  header {
    display: grid;
    grid-template-columns: 80px 1fr 80px;
    align-items: center;
    padding: 0 16px 10px;
  }
  h2 {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    margin: 0;
    font-size: 17px;
    font-weight: 600;
  }
  .glyph {
    display: flex;
    color: var(--tint);
  }
  .link {
    justify-self: end;
    border: 0;
    background: none;
    padding: 6px 0;
    color: var(--tint);
    font: inherit;
    font-size: 17px;
    font-weight: 600;
  }
  .link.quiet {
    color: var(--label2);
    font-weight: 400;
  }
  .round {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    border: 0;
    display: grid;
    place-items: center;
    background: var(--fill);
    color: var(--label2);
  }
</style>
