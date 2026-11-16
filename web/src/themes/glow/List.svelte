<script lang="ts">
  import { fly } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import Icon from "../../ui/Icon.svelte";
  import Donut from "../../ui/Donut.svelte";
  import SwipeRow from "../../ui/SwipeRow.svelte";
  import { accountOf, app, back, breakdown, budgetOf, byDay, byMonth, earning, fallbackOf, open, openStats, periodEntries, prefs, remove, spendIndex, spendBars, spending, todayIndex } from "../../lib/store.svelte";
  import { dayLabel, isYear, money, periodLabel, time } from "../../lib/format";
  import { cardLabel, color, slices } from "../../lib/look";
  import { view } from "../../lib/view";
  import AccountLogo from "../../ui/AccountLogo.svelte";
  import PeriodHeader from "./PeriodHeader.svelte";
  import Mark from "../../ui/Mark.svelte";

  // 传了账户就是推入的账户明细页：只看这个账户的账目，不显示预算。
  let { account }: { account?: number } = $props();
  const acct = $derived(account === undefined ? undefined : accountOf(account));

  let collapsed = $state(false);
  let dir = $state(1);
  let page = $state<HTMLElement>();

  const list = $derived(periodEntries(app.period, account));
  const days = $derived(byDay(list));
  const out = $derived(spending(list));
  const spent = $derived(out.cents);
  const income = $derived(earning(list).cents);
  const parts = $derived(breakdown(list));
  const ring = $derived(slices(parts, spendIndex, fallbackOf("expense").name));
  const year = $derived(isYear(app.period));
  const bars = $derived(spendBars(app.period, list));
  const peak = $derived(Math.max(1, ...bars));
  const today = $derived(todayIndex(app.period));
  const label = $derived(periodLabel(app.period, prefs.cycleDay));
  const budget = $derived(account === undefined ? budgetOf(app.period) : null);
  const left = $derived(budget === null ? 0 : budget - spent);
  // 剩余天数或月数（含今天、本月），只对当前周期有意义。
  const unitsLeft = $derived(today < 0 ? 0 : bars.length - today);

  // 按年看时不列每一笔，按月汇总；点一个月进到那个月。
  const months = $derived(
    year ? [...byMonth(list)].map(([key, es]) => ({ key, out: spending(es), income: earning(es).cents })) : [],
  );
  const monthBudget = $derived(account === undefined ? prefs.budget : null);
  const monthScale = $derived(Math.max(1, monthBudget ?? 0, ...months.map((m) => m.out.cents)));

  function drill(key: string) {
    dir = 1;
    app.period = key;
    page?.scrollTo({ top: 0, behavior: "smooth" });
  }

  const glow = $derived([0, 1, 2].map((i) => color(spendIndex(parts[i]?.name ?? "") + (parts[i] ? 0 : i))));

  const [yuan, cents] = $derived(money(spent).split("."));
</script>

<div class="page" bind:this={page} onscroll={(e) => (collapsed = e.currentTarget.scrollTop > 46)}>
  <header class="bar" class:collapsed class:pushed={acct}>
    {#if acct}<button class="back" onclick={back}><Icon name="chevron-left" size={22} weight={2.6} />财富</button>{/if}
    <span>{acct?.name ?? label.title}</span>
  </header>

  {#if acct}
    <div class="acct">
      <AccountLogo account={acct} size={46} />
      <span><b>{acct.name}</b><small class="num">{acct?.card ? cardLabel(acct.card) : "无卡号"}</small></span>
    </div>
  {/if}
  <PeriodHeader {account} onshift={(d) => (dir = d)} />

  {#key app.period}
    <div in:fly={{ x: 40 * dir, duration: 420, easing: quintOut }}>
      <button class="hero" style:--g1={glow[0]} style:--g2={glow[1]} style:--g3={glow[2]} onclick={openStats}>
        <span class="glass"></span>
        <span class="cap">{acct ? "这个账户" : ""}{year ? "全年支出" : prefs.cycleDay === 1 ? "本月支出" : "本期支出"}<Icon name="chevron-right" size={13} weight={2.8} /></span>
        <span class="big num">{yuan}<span>.{cents}</span></span>
        {#if budget !== null}
          <span class="budget">
            <span class="track"><span class="used" class:over={left < 0} style:width="{Math.min(100, (spent / budget) * 100)}%"></span></span>
            <span class="sub num">
              {#if left >= 0}预算还剩 {money(left)}{#if unitsLeft} · {year ? "每月" : "每天"}可花 {money(Math.floor(left / unitsLeft))}{/if}{:else}超出预算 {money(-left)}{/if}
            </span>
          </span>
        {:else}
          <span class="sub num">{out.count} 笔{#if income} · 收入 {money(income)}{/if}</span>
        {/if}
        <span class="bars" aria-hidden="true">
          {#each bars as v, i (i)}
            <i class:today={i === today} class:future={today >= 0 && i > today} style:height="{Math.max(6, (v / peak) * 100)}%"></i>
          {/each}
        </span>
      </button>

      {#if ring.length}
        <button class="mix" onclick={openStats}>
          <Donut slices={ring} size={112} thickness={14}>
            <small>{parts.length} 类</small>
            <b class="num">{out.count} 笔</b>
          </Donut>
          <ul>
            {#each ring as p (p.name)}
              <li><b style:background={p.color}></b><span class="n">{p.name}</span><span class="num">{money(p.cents)}</span><small class="num">{Math.round((p.cents / spent) * 100)}%</small></li>
            {/each}
          </ul>
        </button>
      {/if}

      {#if year}
        {#if months.length}<h3 class="day"><span>按月</span><span>点开看每一笔</span></h3>{/if}
        <div class="group" class:hidden={!months.length}>
          {#each months as m (m.key)}
            <button class="row month" onclick={() => drill(m.key)}>
              <span class="mo"><b class="num">{Number(m.key.slice(5))}</b>月</span>
              <span class="main">
                <span class="title num">{money(m.out.cents)}<small>{m.out.count} 笔{#if m.income} · 收入 {money(m.income)}{/if}</small></span>
                <span class="meter">
                  <i class:over={monthBudget !== null && m.out.cents > monthBudget} style:width="{(m.out.cents / monthScale) * 100}%"></i>
                  {#if monthBudget !== null}<b style:left="{(monthBudget / monthScale) * 100}%"></b>{/if}
                </span>
              </span>
              <span class="chev"><Icon name="chevron-right" size={14} weight={2.6} /></span>
            </button>
          {/each}
        </div>
        {#if monthBudget !== null && months.length}<p class="note">竖线是每月预算 {money(monthBudget)}</p>{/if}
      {/if}

      {#each year ? [] : days as g (g.day)}
        <h3 class="day"><span>{dayLabel(g.at)}</span>{#if g.spent}<span class="num">支出 {money(g.spent)}</span>{/if}</h3>
        <div class="group">
          {#each g.entries as e (e.id)}
            {@const v = view(e, account)}
            <SwipeRow actions={[{ icon: "trash", label: "删除", danger: true, run: () => remove(e.id) }]}>
              <button class="row" onclick={() => open(e.id)}>
                <span class="icon" style:--c={v.color}><Mark mark={v.mark} /></span>
                <span class="main">
                  <span class="title">{v.title}</span>
                  <span class="meta">{v.meta.join(" · ")}{#if v.tags.length}{" · "}<em>{v.tags.join(" ")}</em>{/if}</span>
                </span>
                <span class="right">
                  <span class="amt num" class:income={v.tone === "in"} class:plain={v.tone === "plain"}>{v.amount}</span>
                  <span class="time num">{time(e.occurred_at)}</span>
                </span>
              </button>
            </SwipeRow>
          {/each}
        </div>
      {:else}
        {#if !months.length}
        <div class="empty">
          <div class="em"><Icon name="receipt" size={54} weight={1.6} /></div>
          <p>{year ? "这一年" : "这个月"}还没有账目</p>
          <small>支付完成后轻点手机背面，截图就会记下来</small>
        </div>
        {/if}
      {/each}
    </div>
  {/key}
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
    transition: background-color 0.2s, box-shadow 0.2s;
  }
  .bar span {
    opacity: 0;
    transform: translateY(6px);
    transition: opacity 0.2s, transform 0.2s;
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
  .bar.pushed {
    grid-template-columns: 1fr auto 1fr;
    padding-left: 8px;
    padding-right: 8px;
  }
  .bar.pushed::after {
    content: "";
  }
  .back {
    justify-self: start;
    display: flex;
    align-items: center;
    border: 0;
    background: none;
    color: var(--tint);
    font: inherit;
    font-size: 17px;
    font-weight: 400;
    padding: 6px 4px;
  }
  .acct {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 4px 20px 12px;
  }
  .acct > span:last-child {
    display: grid;
  }
  .acct b {
    font-size: 20px;
  }
  .acct small {
    font-size: 13px;
    color: var(--label2);
  }

  .hero {
    position: relative;
    display: grid;
    width: calc(100% - 32px);
    transition: transform 0.25s;
    border: 0;
    font: inherit;
    text-align: left;
    overflow: hidden;
    margin: 4px 16px 0;
    padding: 18px 20px 14px;
    border-radius: 24px;
    color: #fff;
    background:
      radial-gradient(120% 90% at 0% 0%, var(--g1) 0%, transparent 60%),
      radial-gradient(90% 80% at 100% 10%, var(--g2) 0%, transparent 62%),
      radial-gradient(100% 90% at 70% 120%, var(--g3) 0%, transparent 60%),
      #241a3a;
    background-size: 160% 160%;
    animation: drift 14s ease-in-out infinite alternate;
    box-shadow: 0 14px 34px -14px color-mix(in oklab, var(--g1) 70%, transparent);
  }
  @keyframes drift {
    to {
      background-position: 100% 60%;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .hero {
      animation: none;
    }
  }
  .glass {
    position: absolute;
    inset: 0;
    background: linear-gradient(160deg, rgba(255, 255, 255, 0.22), transparent 40%);
    pointer-events: none;
  }
  .hero:active,
  .mix:active {
    transform: scale(0.98);
  }
  .hero > span {
    position: relative;
  }
  .cap {
    display: flex;
    align-items: center;
    gap: 3px;
    font-size: 13px;
    font-weight: 600;
    opacity: 0.85;
    letter-spacing: 0.04em;
  }
  .big {
    margin-top: 2px;
    font-size: 40px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }
  .big span {
    font-size: 24px;
    opacity: 0.8;
  }
  .sub {
    font-size: 14px;
    opacity: 0.85;
  }
  .budget {
    display: grid;
    gap: 6px;
    margin-top: 4px;
  }
  .track {
    height: 6px;
    border-radius: 3px;
    background: rgba(255, 255, 255, 0.22);
    overflow: hidden;
  }
  .used {
    display: block;
    height: 100%;
    border-radius: 3px;
    background: #fff;
    transition: width 0.6s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .used.over {
    background: #ffb4a8;
  }
  .bars {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 46px;
    margin-top: 14px;
  }
  .bars i {
    flex: 1;
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.38);
  }
  .bars i.future {
    background: rgba(255, 255, 255, 0.14);
  }
  .bars i.today {
    background: #fff;
    box-shadow: 0 0 10px rgba(255, 255, 255, 0.8);
  }

  .mix {
    display: flex;
    align-items: center;
    gap: 20px;
    width: calc(100% - 32px);
    margin: 12px 16px 0;
    padding: 16px 18px;
    border: 0;
    border-radius: var(--radius-group);
    background: var(--card);
    color: inherit;
    font: inherit;
    text-align: left;
    box-shadow: var(--shadow-card);
    transition: transform 0.25s;
  }
  .mix small {
    font-size: 12px;
    color: var(--label2);
  }
  .mix b.num {
    font-size: 17px;
  }
  .mix ul {
    flex: 1;
    display: grid;
    gap: 7px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 14px;
  }
  .mix li {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .mix li b {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 3px;
  }
  .mix li .n {
    flex: 1;
  }
  .mix li .n {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .mix li small {
    flex: none;
    width: 4ch;
    text-align: right;
    font-size: 12px;
    color: var(--label3);
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
  .note {
    margin: 8px 32px 0;
    font-size: 12px;
    color: var(--label3);
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
    box-shadow: 0 3px 8px -3px color-mix(in oklab, var(--c) 80%, transparent), inset 0 0.5px 0 rgba(255, 255, 255, 0.5);
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
  .empty {
    display: grid;
    justify-items: center;
    gap: 6px;
    padding: 60px 30px;
    text-align: center;
  }
  .empty .em {
    display: flex;
    justify-content: center;
    color: var(--label3);
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
