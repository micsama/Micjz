<script lang="ts">
  import { app } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";
  let { selected, onselect }: { selected: number[]; onselect: (id: number) => void } = $props();
</script>

<div class="tags">
  {#each app.catalog.tags as t (t.id)}<button class:on={selected.includes(t.id)} aria-pressed={selected.includes(t.id)} onclick={() => onselect(t.id)}
      >{t.name}{#if selected.includes(t.id)}<Icon name="check" size={14} />{/if}</button
    >{:else}<span>还没有标签</span>{/each}
</div>

<style>
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  button {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 44px;
    padding: 8px 12px;
    border: 0;
    border-radius: 12px;
    background: var(--fill);
    color: var(--label);
    font: inherit;
  }
  button.on {
    color: var(--tint);
    background: color-mix(in srgb, var(--tint) 12%, var(--card));
  }
  span {
    font-size: 14px;
    color: var(--label2);
  }
</style>
