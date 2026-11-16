<script lang="ts">
  import type { Entry } from "../lib/types";
  import { open, remove } from "../lib/store.svelte";
  import { byDay } from "../lib/query";
  import { dayLabel, money, time } from "../lib/format";
  import { view } from "../lib/view";
  import SwipeRow from "./SwipeRow.svelte";
  import Mark from "./Mark.svelte";
  let { list, account }: { list: Entry[]; account?: number } = $props();
  const days = $derived(byDay(list));
</script>

<div class="entry-days">
  {#each days as g (g.day)}
    <h3 class="day">
      <span>{dayLabel(g.at)}</span>{#if g.spent}<span class="num">支出 {money(g.spent)}</span>{/if}
    </h3>
    <div class="group">
      {#each g.entries as e (e.id)}
        {@const v = view(e, account)}
        <SwipeRow actions={[{ icon: "trash", label: "删除", danger: true, run: () => remove(e.id) }]}>
          <button class="row" onclick={() => open(e.id)}>
            <span class="icon" style:--c={v.color}><Mark mark={v.mark} /></span>
            <span class="main">
              <span class="title">{v.title}</span>
              <span class="meta"
                >{v.meta.join(" · ")}{#if v.tags.length}{" · "}<em>{v.tags.join(" ")}</em>{/if}</span
              >
            </span>
            <span class="right">
              <span class="amt num" class:income={v.tone === "in"} class:plain={v.tone === "plain"}>{v.amount}</span>
              <span class="time num">{time(e.occurred_at)}</span>
            </span>
          </button>
        </SwipeRow>
      {/each}
    </div>
  {/each}
</div>

<style>
  .day {
    display: flex;
    justify-content: space-between;
    margin: 22px 32px 7px;
    font-size: 13px;
    font-weight: 500;
    color: var(--label2);
  }
  .group {
    margin: 0 16px;
    border-radius: var(--radius-group);
    overflow: hidden;
    background: var(--card);
    box-shadow: var(--shadow-card);
  }
  .group > :global(.swipe + .swipe)::before {
    content: "";
    position: absolute;
    left: 62px;
    right: 0;
    top: 0;
    height: 0.5px;
    background: var(--sep);
    z-index: 2;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 62px;
    padding: 9px 16px 9px 12px;
    border: 0;
    background: var(--card);
    color: inherit;
    font: inherit;
    text-align: left;
    transition: background-color 0.35s;
  }
  .row:active {
    background: color-mix(in srgb, var(--card), var(--label) 8%);
    transition: none;
  }
  .icon {
    flex: none;
    width: 38px;
    height: 38px;
    border-radius: 11px;
    display: grid;
    place-items: center;
    font-size: 20px;
    color: white;
    background: linear-gradient(145deg, color-mix(in oklab, var(--c) 55%, white), var(--c));
    box-shadow:
      0 3px 8px -3px color-mix(in oklab, var(--c) 80%, transparent),
      inset 0 0.5px 0 rgba(255, 255, 255, 0.5);
  }
  .main {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 2px;
  }
  .title {
    font-size: 16px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta {
    font-size: 13px;
    color: var(--label2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta em {
    font-style: normal;
    color: var(--c2);
  }
  .right {
    display: grid;
    justify-items: end;
    gap: 3px;
  }
  .amt {
    font-size: 16px;
    font-weight: 600;
  }
  .amt.income {
    color: var(--green);
  }
  .amt.plain {
    color: var(--label2);
  }
  .time {
    font-size: 12px;
    color: var(--label3);
  }
</style>
