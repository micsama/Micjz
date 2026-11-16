<script lang="ts">
  // 饼环图：段间留 2px 缝；点一段选中，再点取消。中间内容由调用方给。
  import type { Snippet } from "svelte";
  import type { Slice } from "../lib/look";

  let {
    slices,
    size = 140,
    thickness = 16,
    selected = null,
    onselect,
    children,
  }: {
    slices: Slice[];
    size?: number;
    thickness?: number;
    selected?: string | null;
    onselect?: (name: string | null) => void;
    children?: Snippet;
  } = $props();

  const GAP = 2;
  const r = $derived((size - thickness) / 2);
  const c = $derived(2 * Math.PI * r);
  const sum = $derived(slices.reduce((s, x) => s + x.cents, 0) || 1);
  const arcs = $derived.by(() => {
    let at = 0;
    return slices.map((s) => {
      const len = (s.cents / sum) * c;
      const arc = { ...s, dash: Math.max(0.5, len - (slices.length > 1 ? GAP : 0)), offset: -at };
      at += len;
      return arc;
    });
  });
</script>

<div class="donut" style:width="{size}px" style:height="{size}px">
  <svg viewBox="0 0 {size} {size}" role="img" aria-label="分类占比">
    <g transform="rotate(-90 {size / 2} {size / 2})">
      {#each arcs as a (a.name)}
        <circle
          class:dim={selected && selected !== a.name}
          cx={size / 2}
          cy={size / 2}
          {r}
          fill="none"
          stroke={a.color}
          stroke-width={thickness}
          stroke-dasharray="{a.dash} {c}"
          stroke-dashoffset={a.offset}
          role="presentation"
          onclick={onselect ? () => onselect(selected === a.name ? null : a.name) : undefined}
        />
      {/each}
    </g>
  </svg>
  {#if children}<div class="center">{@render children()}</div>{/if}
</div>

<style>
  .donut {
    position: relative;
    flex: none;
  }
  svg {
    display: block;
    overflow: visible;
  }
  circle {
    transition: opacity 0.25s;
  }
  circle.dim {
    opacity: 0.25;
  }
  svg {
    animation: spin-in 0.7s cubic-bezier(0.2, 0.8, 0.2, 1) both;
  }
  @keyframes spin-in {
    from {
      opacity: 0;
      transform: rotate(-60deg) scale(0.85);
    }
  }
  .center {
    position: absolute;
    inset: 0;
    display: grid;
    place-content: center;
    justify-items: center;
    text-align: center;
    pointer-events: none;
  }
</style>
