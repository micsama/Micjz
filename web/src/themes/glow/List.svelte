<script lang="ts">
  import EntryDays from "../../ui/EntryDays.svelte";
  import { fly } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import Icon from "../../ui/Icon.svelte";
  import SpendingCard from "../../ui/SpendingCard.svelte";
  import { app, periodStatus, settings, want } from "../../lib/store.svelte";
  import { byMonth, earning, periodEntries, spending } from "../../lib/query";
  import { isYear, money, periodLabel } from "../../lib/format";
  import PeriodHeader from "./PeriodHeader.svelte";

  let collapsed = $state(false);
  let dir = $state(1);
  let page = $state<HTMLElement>();

  const list = $derived(periodEntries(app.period));
  const year = $derived(isYear(app.period));
  const label = $derived(periodLabel(app.period, settings.cycle_day));

  // 按年看时不列每一笔，按月汇总；点一个月进到那个月。
  const months = $derived(year ? [...byMonth(list)].map(([key, es]) => ({ key, out: spending(es), income: earning(es).cents })) : []);
  const monthBudget = $derived(settings.budget_cents);
  const monthScale = $derived(Math.max(1, monthBudget ?? 0, ...months.map((m) => m.out.cents)));

  function drill(key: string) {
    dir = 1;
    app.period = key;
    page?.scrollTo({ top: 0, behavior: "smooth" });
  }
</script>

<div class="page" bind:this={page} onscroll={(e) => (collapsed = e.currentTarget.scrollTop > 46)}>
  <header class="bar" class:collapsed><span>{label.title}</span></header>
  <PeriodHeader onshift={(d) => (dir = d)} />

  {#if periodStatus(app.period) === "ready"}
    {#key app.period}
      <div in:fly={{ x: 40 * dir, duration: 420, easing: quintOut }}>
        <SpendingCard />

        {#if year}
          {#if months.length}<h3 class="day"><span>按月</span><span>点开看每一笔</span></h3>{/if}
          <div class="group" class:hidden={!months.length}>
            {#each months as m (m.key)}
              <button class="row month" onclick={() => drill(m.key)}>
                <span class="mo"><b class="num">{Number(m.key.slice(5))}</b>月</span>
                <span class="main">
                  <span class="title num"
                    >{money(m.out.cents)}<small
                      >{m.out.count} 笔{#if m.income}
                        · 收入 {money(m.income)}{/if}</small
                    ></span
                  >
                  <span class="meter">
                    <i class:over={monthBudget !== null && m.out.cents > monthBudget} style:width="{(m.out.cents / monthScale) * 100}%"></i>
                    {#if monthBudget !== null}<b style:left="{(monthBudget / monthScale) * 100}%"></b>{/if}
                  </span>
                </span>
                <span class="chev"><Icon name="chevron-right" size={14} weight={2.6} /></span>
              </button>
            {/each}
          </div>
        {/if}

        {#if !year}<EntryDays {list} />{/if}
        {#if !list.length}<div class="empty">
            <p>这一期还没有账目</p>
            <small>轻点手机背面截图记账，或点下方「＋」记一笔</small>
          </div>{/if}
      </div>
    {/key}
  {:else if periodStatus(app.period) === "failed"}
    <div class="empty"><button onclick={() => void want(app.period)}>账目没取到，重试</button></div>
  {:else}
    <div class="empty" role="status">正在加载账目…</div>
  {/if}
  <div class="tail"></div>
</div>

<style>
  .page {
    position: absolute;
    inset: 0;
    overflow-y: auto;
    overflow-x: hidden;
    touch-action: pan-y;
    -webkit-overflow-scrolling: touch;
    padding-top: env(safe-area-inset-top);
  }
  .bar {
    position: sticky;
    top: calc(-1 * env(safe-area-inset-top));
    z-index: 5;
    height: 44px;
    margin-top: calc(-1 * env(safe-area-inset-top));
    padding-top: env(safe-area-inset-top);
    display: grid;
    place-items: center;
    font-weight: 600;
    transition:
      background-color 0.2s,
      box-shadow 0.2s;
  }
  .bar span {
    opacity: 0;
    transform: translateY(6px);
    transition:
      opacity 0.2s,
      transform 0.2s;
  }
  .bar.collapsed {
    background: var(--bar-bg);
    backdrop-filter: saturate(180%) blur(24px);
    -webkit-backdrop-filter: saturate(180%) blur(24px);
    box-shadow: 0 0.5px 0 var(--sep);
  }
  .bar.collapsed span {
    opacity: 1;
    transform: none;
  }

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
  .group.hidden {
    display: none;
  }
  .month + .month {
    box-shadow: inset 0 0.5px 0 var(--sep);
  }
  .mo {
    flex: none;
    width: 40px;
    display: flex;
    align-items: baseline;
    justify-content: center;
    font-size: 12px;
    font-weight: 600;
    color: var(--label2);
  }
  .mo b {
    font-size: 24px;
    font-weight: 700;
    color: var(--label);
  }
  .title small {
    margin-left: 8px;
    font-size: 12px;
    font-weight: 400;
    color: var(--label2);
  }
  .meter {
    position: relative;
    height: 6px;
    margin: 4px 0 2px;
    border-radius: 3px;
    background: var(--fill);
  }
  .meter i {
    display: block;
    height: 100%;
    border-radius: 3px;
    background: var(--tint);
    transition: width 0.5s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .meter i.over {
    background: var(--red);
  }
  .meter b {
    position: absolute;
    top: -3px;
    bottom: -3px;
    width: 2px;
    margin-left: -1px;
    border-radius: 1px;
    background: var(--label3);
  }
  .chev {
    flex: none;
    display: flex;
    color: var(--label3);
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
  .empty {
    display: grid;
    justify-items: center;
    gap: 6px;
    padding: 60px 30px;
    text-align: center;
  }

  .empty p {
    margin: 6px 0 0;
    font-size: 17px;
    font-weight: 600;
  }
  .empty small {
    font-size: 14px;
    color: var(--label2);
  }
  .tail {
    height: calc(env(safe-area-inset-bottom) + 100px);
  }
</style>
