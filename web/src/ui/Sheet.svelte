<script lang="ts">
  // 底部弹出面板：带拖动条，下拉或点背景关闭；键盘弹出时跟着上移。
  import type { Snippet } from "svelte";
  import { fade } from "svelte/transition";
  import { keyboardInset, motion } from "./nav.svelte";

  let { open, onclose, children }: { open: boolean; onclose: () => void; children: Snippet } = $props();

  let panel = $state<HTMLDivElement>();
  let drag: { id: number; y0: number; y: number; t: number; v: number } | null = null;

  function rise(_: Element) {
    const { easing, reduce } = motion();
    if (reduce) return { duration: 200, css: (t: number) => `opacity: ${t}` };
    return { duration: 480, easing, css: (t: number) => `transform: translateY(${(1 - t) * 105}%)` };
  }

  function down(e: PointerEvent) {
    drag = { id: e.pointerId, y0: e.clientY, y: e.clientY, t: e.timeStamp, v: 0 };
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
  }

  function move(e: PointerEvent) {
    if (!drag || !panel || e.pointerId !== drag.id) return;
    drag.v = (e.clientY - drag.y) / Math.max(1, e.timeStamp - drag.t);
    drag.y = e.clientY;
    drag.t = e.timeStamp;
    const dy = e.clientY - drag.y0;
    // 往上拉有阻尼，往下跟手。
    panel.style.transform = `translateY(${dy > 0 ? dy : dy / 6}px)`;
  }

  function up(e: PointerEvent) {
    if (!drag || !panel || e.pointerId !== drag.id) return;
    const dy = drag.y - drag.y0;
    const close = dy > 110 || drag.v > 0.6;
    drag = null;
    if (close) {
      onclose();
      return;
    }
    panel.style.transition = "transform 0.35s cubic-bezier(0.2, 0.8, 0.2, 1)";
    panel.style.transform = "";
    panel.addEventListener("transitionend", () => panel && (panel.style.transition = ""), { once: true });
  }
</script>

{#if open}
  <div class="backdrop" transition:fade={{ duration: 260 }} onclick={onclose} role="presentation"></div>
  <div class="sheet" bind:this={panel} transition:rise use:keyboardInset role="dialog" aria-modal="true">
    <div class="grab" role="presentation" onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up}>
      <span></span>
    </div>
    {@render children()}
  </div>
{/if}

<style>
  .backdrop {
    position: absolute;
    inset: 0;
    z-index: 40;
    background: var(--scrim);
  }
  .sheet {
    position: absolute;
    left: 0;
    right: 0;
    bottom: var(--kb, 0px);
    z-index: 41;
    max-height: calc(100% - env(safe-area-inset-top) - 24px);
    display: flex;
    flex-direction: column;
    background: var(--sheet-bg);
    border-radius: var(--radius-sheet) var(--radius-sheet) 0 0;
    box-shadow: 0 -8px 40px rgba(0, 0, 0, 0.18);
    padding-bottom: max(env(safe-area-inset-bottom), 12px);
    transition: bottom 0.25s ease;
  }
  .grab {
    flex: none;
    height: 22px;
    display: grid;
    place-items: center;
    touch-action: none;
    cursor: grab;
  }
  .grab span {
    width: 36px;
    height: 5px;
    border-radius: 3px;
    background: var(--grabber);
  }
</style>
