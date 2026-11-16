<script lang="ts">
  import { categoryOf, subsOf, topOf, tops } from "../lib/store.svelte";
  import { badge } from "../lib/format";
  import { color } from "../lib/look";
  import type { FlowKind } from "../lib/types";
  let { kind, selected, onselect }: { kind?: FlowKind; selected: number | null; onselect: (id: number, final: boolean) => void } = $props();
  const categories = $derived(kind ? tops(kind) : [...tops("expense"), ...tops("income")]);
  const current = $derived(categoryOf(selected));
  const expanded = $derived(current ? topOf(current).id : null);
  function pickTop(id: number) {
    onselect(id, !subsOf(categoryOf(id)!).length);
  }
</script>

<div class="category-grid">
  {#each categories as c, i (c.id)}
    {@const children = subsOf(c)}
    <button
      class="category"
      class:active={expanded === c.id}
      class:selected={current && topOf(current).id === c.id}
      style:--category-color={color(i)}
      aria-expanded={children.length ? expanded === c.id : undefined}
      onclick={() => pickTop(c.id)}
    >
      <span class="category-icon"
        >{badge(c.name).icon}{#if children.length}<i></i>{/if}</span
      >
      <span>{badge(c.name).text}</span>
    </button>
    {#if expanded === c.id && children.length}
      <div class="subcategories" style:grid-row={Math.ceil(categories.length / 4) + 1}>
        {#each children as sub (sub.id)}
          <button class:on={selected === sub.id} aria-pressed={selected === sub.id} onclick={() => onselect(sub.id, true)}
            ><span>{badge(sub.name).icon}</span>{badge(sub.name).text}</button
          >
        {/each}
      </div>
    {/if}
  {/each}
</div>

<style>
  .category-grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    align-content: start;
    gap: 10px 8px;
  }
  .category {
    background: none;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    padding: 2px 0;
    font-size: 12px;
  }
  .category > span:last-child {
    max-width: 100%;
    overflow-wrap: anywhere;
  }
  .category-icon {
    position: relative;
    display: grid;
    place-items: center;
    width: 42px;
    height: 42px;
    border-radius: 14px;
    background: var(--fill);
    font-size: 23px;
    transition:
      background 0.2s,
      transform 0.2s;
  }
  .category.active .category-icon {
    background: color-mix(in srgb, var(--category-color) 20%, var(--card));
  }
  .category.selected {
    color: var(--tint);
    font-weight: 650;
  }
  .category.selected .category-icon {
    box-shadow: inset 0 0 0 1.5px var(--tint);
  }
  .category-icon i {
    position: absolute;
    bottom: 3px;
    right: 3px;
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--label3);
  }
  .subcategories {
    grid-column: 1/-1;
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 8px;
    border-radius: var(--radius-group);
    background: var(--fill);
  }
  .subcategories button {
    flex: 0 1 auto;
    min-width: 0;
    max-width: 100%;
    white-space: normal;
    overflow-wrap: anywhere;
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 7px 10px;
    min-height: 40px;
    background: var(--card);
    border-radius: 12px;
    font-size: 14px;
  }
  .subcategories button.on {
    background: var(--tint);
    color: white;
  }
  .subcategories button span {
    font-size: 18px;
  }
  button {
    font: inherit;
    color: inherit;
    border: 0;
    cursor: pointer;
  }
</style>
