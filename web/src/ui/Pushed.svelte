<script lang="ts">
  // 推入的页面：从右侧滑入；从左边缘右滑返回（主屏幕模式下系统不提供）。
  import type { Snippet } from "svelte";
  import { motion, nav } from "./nav.svelte";

  let { onback, children }: { onback: () => void; children: Snippet } = $props();

  let el: HTMLDivElement;
  let swiped = false;
  let drag: { id: number; x0: number; x: number; t: number; v: number } | null = null;

  function slide(_: Element) {
    const { easing, reduce } = motion();
    if (swiped) return { duration: 0 };
    if (reduce) return { duration: 200, css: (t: number) => `opacity: ${t}` };
    return {
      duration: 460,
      easing,
      css: (t: number) => `transform: translateX(${(1 - t) * 100}%); box-shadow: 0 0 ${24 * t}px rgba(0,0,0,${0.12 * t})`,
    };
  }

  function down(e: PointerEvent) {
    if (e.clientX - el.getBoundingClientRect().left > 28) return;
    drag = { id: e.pointerId, x0: e.clientX, x: e.clientX, t: e.timeStamp, v: 0 };
    el.setPointerCapture(e.pointerId);
    nav.dragging = true;
  }

  function move(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    const dt = Math.max(1, e.timeStamp - drag.t);
    drag.v = (e.clientX - drag.x) / dt;
    drag.x = e.clientX;
    drag.t = e.timeStamp;
    const dx = Math.max(0, e.clientX - drag.x0);
    el.style.transform = `translateX(${dx}px)`;
    nav.drag = dx / el.offsetWidth;
  }

  function up(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    const done = nav.drag > 0.35 || drag.v > 0.5;
    drag = null;
    nav.dragging = false;
    el.style.transition = "transform 0.3s cubic-bezier(0.2, 0.8, 0.2, 1)";
    el.style.transform = done ? "translateX(100%)" : "";
    nav.drag = done ? 1 : 0;
    el.addEventListener(
      "transitionend",
      () => {
        el.style.transition = "";
        if (done) {
          swiped = true;
          onback();
        }
        nav.drag = 0;
      },
      { once: true },
    );
  }
</script>

<div class="pushed" role="presentation" bind:this={el} transition:slide onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up}>
  {@render children()}
</div>

<style>
  .pushed {
    position: absolute;
    inset: 0;
    background: var(--bg);
    touch-action: pan-y;
    z-index: 10;
    overflow: hidden;
  }
</style>
