<script lang="ts">
  // 全屏看截图：双指缩放、拖动，双击放大 / 还原，未放大时下滑关闭。
  import { fade, scale as zoomIn } from "svelte/transition";
  import { quintOut } from "svelte/easing";

  let { src, onclose }: { src: string; onclose: () => void } = $props();

  let s = $state(1);
  let tx = $state(0);
  let ty = $state(0);
  let animate = $state(false);
  let dim = $derived(s === 1 ? Math.max(0, 1 - Math.abs(ty) / 400) : 1);

  const pts = new Map<number, { x: number; y: number }>();
  let start: { s: number; tx: number; ty: number; d: number; mx: number; my: number } | null = null;
  let lastTap = 0;

  const mid = () => {
    const [a, b] = [...pts.values()];
    return b ? { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2, d: Math.hypot(a.x - b.x, a.y - b.y) } : { x: a.x, y: a.y, d: 1 };
  };

  function begin() {
    const m = mid();
    start = { s, tx, ty, d: m.d, mx: m.x, my: m.y };
  }

  function down(e: PointerEvent) {
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
    pts.set(e.pointerId, { x: e.clientX, y: e.clientY });
    animate = false;
    begin();
    if (pts.size === 1) {
      if (e.timeStamp - lastTap < 280) {
        animate = true;
        if (s > 1) [s, tx, ty] = [1, 0, 0];
        else [s, tx, ty] = [2.5, (innerWidth / 2 - e.clientX) * 1.5, (innerHeight / 2 - e.clientY) * 1.5];
        lastTap = 0;
      } else lastTap = e.timeStamp;
    }
  }

  function move(e: PointerEvent) {
    if (!pts.has(e.pointerId) || !start) return;
    pts.set(e.pointerId, { x: e.clientX, y: e.clientY });
    const m = mid();
    if (pts.size === 2) s = Math.min(5, Math.max(0.6, (start.s * m.d) / start.d));
    tx = start.tx + m.x - start.mx;
    ty = start.ty + m.y - start.my;
    if (start.s === 1 && pts.size === 1) tx = tx / 3;
  }

  function up(e: PointerEvent) {
    pts.delete(e.pointerId);
    if (pts.size) return begin();
    animate = true;
    if (s <= 1.02) {
      if (Math.abs(ty) > 120) return onclose();
      [s, tx, ty] = [1, 0, 0];
    }
  }
</script>

<div class="viewer" style:background="rgba(0,0,0,{dim})" transition:fade={{ duration: 220 }}>
  <button class="close" onclick={onclose} aria-label="关闭">完成</button>
  <div class="stage" onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up} role="presentation">
    <img
      {src}
      alt="支付截图"
      class:animate
      style:transform="translate({tx}px, {ty}px) scale({s})"
      transition:zoomIn={{ start: 0.85, duration: 380, easing: quintOut }}
      draggable="false"
    />
  </div>
</div>

<style>
  .viewer {
    position: absolute;
    inset: 0;
    z-index: 70;
  }
  .stage {
    position: absolute;
    inset: 0;
    touch-action: none;
    padding: calc(env(safe-area-inset-top) + 50px) 0 calc(env(safe-area-inset-bottom) + 20px);
  }
  /* 网格里百分比 max-height 在 Safari 不生效，长截图会撑出屏幕；改成铺满舞台按比例缩放。 */
  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
    user-select: none;
    -webkit-user-drag: none;
  }
  img.animate {
    transition: transform 0.35s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .close {
    position: absolute;
    z-index: 1;
    top: calc(env(safe-area-inset-top) + 8px);
    right: 12px;
    padding: 6px 14px;
    border: 0;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.18);
    color: #fff;
    font: inherit;
    font-weight: 600;
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
  }
</style>
