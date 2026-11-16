<script module lang="ts">
  // 同一时刻只有一行是展开的。
  let closeOpen: (() => void) | null = null;
</script>

<script lang="ts">
  // 左滑露出这一行的操作按钮，点按钮才执行。全 app 行操作只用这一种手势：
  // 最多露两个按钮，多于两个时第二个换成「更多」，其余进底部菜单（title 作菜单标题）。
  // 危险操作（删除）点了先整行滑出再执行。
  import type { Snippet } from "svelte";
  import type { Action } from "./ActionSheet.svelte";
  import { showMenu } from "../lib/store.svelte";

  let { actions, title = "", children }: { actions: Action[]; title?: string; children: Snippet } = $props();

  const BUTTON = 76;
  const shown = $derived(
    actions.length <= 2 ? actions : [actions[0], { label: "更多", run: () => showMenu(title, actions.slice(1)) }],
  );
  const ACTION = $derived(BUTTON * shown.length);
  let x = $state(0);
  let animate = $state(true);
  let width = $state(1);
  let drag: { id: number; x0: number; y0: number; base: number; lock: "h" | "v" | null } | null = null;
  let moved = false;

  function close() {
    animate = true;
    x = 0;
  }

  function down(e: PointerEvent) {
    drag = { id: e.pointerId, x0: e.clientX, y0: e.clientY, base: x, lock: null };
    moved = false;
  }

  function move(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    const dx = e.clientX - drag.x0;
    const dy = e.clientY - drag.y0;
    if (!drag.lock) {
      if (Math.hypot(dx, dy) < 8) return;
      drag.lock = Math.abs(dx) > Math.abs(dy) ? "h" : "v";
      if (drag.lock === "h") {
        (e.currentTarget as Element).setPointerCapture(e.pointerId);
        if (closeOpen && closeOpen !== close) closeOpen();
      }
    }
    if (drag.lock !== "h") return;
    moved = true;
    animate = false;
    const next = drag.base + dx;
    // 超出按钮宽度后加阻尼，不再继续跟手。
    x = next > 0 ? next / 4 : next < -ACTION ? -ACTION + (next + ACTION) / 4 : next;
  }

  function up(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    const wasH = drag.lock === "h";
    drag = null;
    if (!wasH) return;
    animate = true;
    if (x < -ACTION / 2) {
      x = -ACTION;
      closeOpen = close;
    } else {
      x = 0;
    }
  }

  function run(a: Action) {
    if (a.danger) {
      animate = true;
      x = -width;
      setTimeout(a.run, 220);
    } else {
      close();
      a.run();
    }
  }

  // 拖动后的 click 不算点击；展开时点一下先收起。
  function click(e: MouseEvent) {
    if (moved || x !== 0) {
      e.stopPropagation();
      e.preventDefault();
      close();
    }
  }
</script>

<div class="swipe" bind:clientWidth={width}>
  <div class="actions" style:width="{Math.max(ACTION, -x)}px">
    {#each shown as a, i (a.label)}
      <button class:danger={a.danger} class:more={i > 0 && !a.danger} onclick={() => run(a)}>{a.label}</button>
    {/each}
  </div>
  <div
    class="content"
    class:animate
    style:transform="translateX({x}px)"
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onpointercancel={up}
    onclickcapture={click}
    role="presentation"
  >
    {@render children()}
  </div>
</div>

<style>
  .swipe {
    position: relative;
    overflow: hidden;
    background: var(--row-bg, var(--card));
  }
  .content {
    position: relative;
    z-index: 1;
    touch-action: pan-y;
    background: inherit;
  }
  .content.animate {
    transition: transform 0.32s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .actions {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    display: flex;
  }
  .actions button {
    flex: 1;
    min-width: 0;
    padding: 0 6px;
    border: 0;
    background: var(--tint);
    color: #fff;
    font: inherit;
    font-size: 15px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
  }
  .actions .more {
    background: #8e8e93;
  }
  .actions .danger {
    background: var(--red);
  }
</style>
