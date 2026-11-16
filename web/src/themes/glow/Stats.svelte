<script lang="ts">
  import Icon from "../../ui/Icon.svelte";
  import Donut from "../../ui/Donut.svelte";
  import { app, back, breakdown, budgetOf, elapsedDays, isLatest, open, periodEntries, prefs, spendBars, sumBy, todayIndex, total } from "../../lib/store.svelte";
  import { isYear, leaf, money, periodLabel, periodRange, shiftPeriod } from "../../lib/format";
  import { color, emoji, slices } from "../../lib/look";

  let scrolled = $state(false);
  let picked = $state<string | null>(null);
  let day = $state<number | null>(null);

  const list = $derived(periodEntries(app.period));
  const spent = $derived(total(list, "expense"));
  const income = $derived(total(list, "income"));
  const prev = $derived(total(periodEntries(shiftPeriod(app.period, -1)), "expense"));
  const label = $derived(periodLabel(app.period, prefs.cycleDay));
  const latest = $derived(isLatest(app.period));
  const year = $derived(isYear(app.period));
  /** 柱子的单位：按月看是天，按年看是月。 */
  const unit = $derived(year ? "月" : "日");

  const bars = $derived(spendBars(app.period, list));
  const today = $derived(todayIndex(app.period));
  // 已经过去的柱数：当前周期算到今天（本月），过去的周期算整期。
  const elapsed = $derived(today >= 0 ? today + 1 : bars.length);
  const avg = $derived(Math.round(spent / elapsed));
  const peak = $derived(Math.max(1, ...bars));
  const budget = $derived(budgetOf(app.period));
  const pace = $derived(elapsedDays(app.period));
  const start = $derived(periodRange(app.period, prefs.cycleDay).start);
  const dateOf = (i: number) => {
    if (year) return `${i + 1}月`;
    const d = new Date(start);
    d.setDate(d.getDate() + i);
    return `${d.getMonth() + 1}月${d.getDate()}日`;
  };

  const idx = (name: string) => Math.max(0, app.catalog.expense.findIndex((c) => c.name === name));
  const parts = $derived(breakdown(list));
  const ring = $derived(slices(parts, idx, 6));
  const subs = (name: string) => sumBy(list.filter((e) => e.category === name), (e) => e.subcategory);
  const accounts = $derived(sumBy(list, (e) => e.account));
  const merchants = $derived(sumBy(list, (e) => e.merchant).slice(0, 5));
  const tags = $derived(sumBy(list, (e) => e.tags));
  const biggest = $derived(list.filter((e) => e.kind === "expense").sort((a, b) => b.amount_cents - a.amount_cents).slice(0, 3));
  const pickedSlice = $derived(ring.find((s) => s.name === picked));
  const pct = (cents: number) => `${Math.round((cents / (spent || 1)) * 100)}%`;
  const change = $derived(prev ? Math.round(((spent - prev) / prev) * 100) : null);

  function go(delta: number) {
    picked = null;
    day = null;
    app.period = shiftPeriod(app.period, delta);
  }

  /** 按年看时从某个月的柱子进到那个月。 */
  function drill(i: number) {
    day = null;
    picked = null;
    app.period = `${app.period}-${String(i + 1).padStart(2, "0")}`;
  }
</script>

{#snippet bar(name: string, cents: number, count: number, c: string, max: number)}
  <div class="bar-row">
    <div class="bar-head"><span>{name}</span><span class="num">{money(cents)}<small>{" · "}{count} 笔</small></span></div>
    <div class="bar-track"><i style:width="{(cents / max) * 100}%" style:background={c}></i></div>
  </div>
{/snippet}

<div class="page" onscroll={(e) => (scrolled = e.currentTarget.scrollTop > 40)}>
  <header class="nav" class:scrolled>
    <button class="back" onclick={back}><Icon name="chevron-left" size={22} weight={2.6} />账目</button>
    <span class="t">{label.title}统计</span>
    <span></span>
  </header>

  <div class="period">
    <button onclick={() => go(-1)} aria-label="上一期"><Icon name="chevron-left" size={18} weight={2.6} /></button>
    <div><h1>{label.title}</h1><small>{label.range || (year ? "全年" : label.year)}</small></div>
    <button onclick={() => go(1)} disabled={latest} aria-label="下一期"><Icon name="chevron-right" size={18} weight={2.6} /></button>
  </div>

  <section class="tiles">
    <div class="tile">
      <small>支出</small><b class="num">{money(spent)}</b>
      {#if change !== null}<em class:up={change > 0}>{change > 0 ? "↑" : "↓"} 比{year ? "去年" : "上期"} {Math.abs(change)}%</em>{/if}
    </div>
    <div class="tile"><small>收入</small><b class="num green">{money(income)}</b><em>{list.filter((e) => e.kind === "income").length} 笔</em></div>
    <div class="tile"><small>结余</small><b class="num" class:red={income < spent}>{money(income - spent, true)}</b><em>收入 − 支出</em></div>
    <div class="tile"><small>{unit}均支出</small><b class="num">{money(avg)}</b><em>按 {elapsed} {year ? "个月" : "天"}算</em></div>
    {#if budget !== null}
      {@const fast = spent / budget > pace.elapsed / pace.days}
      <div class="tile wide">
        <div class="budget-head">
          <small>{year ? "全年" : ""}预算 {money(budget)}</small>
          <span class="num">{spent <= budget ? `剩 ${money(budget - spent)}` : `超 ${money(spent - budget)}`}</span>
        </div>
        <div class="bar-track big"><i class:over={spent > budget} class:fast style:width="{Math.min(100, (spent / budget) * 100)}%"></i><b style:left="{Math.min(100, (pace.elapsed / pace.days) * 100)}%"></b></div>
        <em>{year ? `每月 ${money(budget / 12)} × 12。` : ""}竖线是时间进度：已过 {pace.elapsed}/{pace.days} 天，花了预算的 {Math.round((spent / budget) * 100)}%{fast ? "，花得比时间快" : ""}</em>
      </div>
    {/if}
  </section>

  <h3 class="cap">每{unit}支出</h3>
  <section class="card">
    <div class="readout">
      {#if day !== null}<span>{dateOf(day)}</span><b class="num">{money(bars[day])}</b>
        {#if year}<button class="drill" onclick={() => drill(day!)}>看这个月<Icon name="chevron-right" size={12} weight={2.8} /></button>{/if}
      {:else}<span>最高一{year ? "个月" : "天"}</span><b class="num">{money(peak === 1 ? 0 : peak)}</b>{/if}
    </div>
    <div class="chart" role="presentation" onpointerleave={() => !year && (day = null)}>
      <i class="avg" style:bottom="{(avg / peak) * 100}%"><span class="num">{unit}均 {money(avg)}</span></i>
      {#each bars as v, i (i)}
        <button class="col" class:on={day === i} class:future={today >= 0 && i > today} onpointerenter={() => (day = i)} onclick={() => (day = i)} aria-label={dateOf(i)}>
          <i style:height="{Math.max(2, (v / peak) * 100)}%"></i>
        </button>
      {/each}
    </div>
    <div class="axis num"><span>{dateOf(0)}</span><span>{dateOf(Math.floor(bars.length / 2))}</span><span>{dateOf(bars.length - 1)}</span></div>
  </section>

  <h3 class="cap">分类</h3>
  <section class="card">
    {#if ring.length}
      <div class="ring">
        <Donut slices={ring} size={200} thickness={24} selected={picked} onselect={(n) => (picked = n)}>
          {#if pickedSlice}
            <small>{pickedSlice.name}</small><b class="num">{money(pickedSlice.cents)}</b><small class="num">{pct(pickedSlice.cents)}</small>
          {:else}
            <small>总支出</small><b class="num">{money(spent)}</b><small>点圆环看分类</small>
          {/if}
        </Donut>
      </div>
      {#each parts as p (p.name)}
        <button class="cat" class:open={picked === p.name} onclick={() => (picked = picked === p.name ? null : p.name)}>
          <span class="ce" style:--c={color(idx(p.name))}>{emoji({ category: p.name, subcategory: null })}</span>
          <span class="cn">{p.name}<small>{p.count} 笔</small></span>
          <span class="cv num">{money(p.cents)}<small>{pct(p.cents)}</small></span>
        </button>
        {#if picked === p.name}
          <div class="subs">
            {#each subs(p.name) as s (s.name)}
              {@render bar(s.name, s.cents, s.count, color(idx(p.name)), p.cents)}
            {:else}
              <p class="none">没有二级分类</p>
            {/each}
          </div>
        {/if}
      {/each}
    {:else}
      <p class="none">这一期没有支出</p>
    {/if}
  </section>

  {#if accounts.length}
    <h3 class="cap">账户</h3>
    <section class="card pad">
      {#each accounts as a, i (a.name)}{@render bar(a.name, a.cents, a.count, `var(--c${[1, 5, 4, 3][i % 4]})`, accounts[0].cents)}{/each}
    </section>
  {/if}

  {#if merchants.length}
    <h3 class="cap">常去的商家</h3>
    <section class="card pad">
      {#each merchants as m (m.name)}{@render bar(m.name, m.cents, m.count, "var(--c0)", merchants[0].cents)}{/each}
    </section>
  {/if}

  {#if tags.length}
    <h3 class="cap">标签</h3>
    <section class="card pad">
      {#each tags as t (t.name)}{@render bar(`🏷️ ${t.name}`, t.cents, t.count, "var(--c2)", Math.max(...tags.map((x) => x.cents)))}{/each}
    </section>
  {/if}

  {#if biggest.length}
    <h3 class="cap">最大的几笔</h3>
    <section class="card">
      {#each biggest as e, i (e.id)}
        <button class="cat" onclick={() => open(e.id)}>
          <span class="rank num">{i + 1}</span>
          <span class="cn">{e.merchant ?? leaf(e)}<small>{leaf(e)} · {dateOf(Math.floor((new Date(e.occurred_at).getTime() - start.getTime()) / 86400000))}</small></span>
          <span class="cv num">{money(e.amount_cents)}<small>{pct(e.amount_cents)}</small></span>
        </button>
      {/each}
    </section>
  {/if}
  <div class="tail"></div>
</div>

<style>
  .page {
    position: absolute;
    inset: 0;
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
    padding-top: env(safe-area-inset-top);
  }
  .nav {
    position: sticky;
    top: calc(-1 * env(safe-area-inset-top));
    z-index: 5;
    margin-top: calc(-1 * env(safe-area-inset-top));
    padding: env(safe-area-inset-top) 8px 0;
    height: 44px;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    transition: background-color 0.2s;
  }
  .nav.scrolled {
    background: var(--bar-bg);
    backdrop-filter: saturate(180%) blur(24px);
    -webkit-backdrop-filter: saturate(180%) blur(24px);
    box-shadow: 0 0.5px 0 var(--sep);
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
    padding: 6px 4px;
  }
  .t {
    font-weight: 600;
    opacity: 0;
    transition: opacity 0.2s;
  }
  .nav.scrolled .t {
    opacity: 1;
  }
  .period {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px 6px;
    text-align: center;
  }
  .period h1 {
    margin: 0;
    font-size: 28px;
    font-weight: 700;
  }
  .period small {
    font-size: 13px;
    color: var(--label2);
  }
  .period button {
    width: 34px;
    height: 34px;
    border: 0;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--fill);
    color: var(--tint);
  }
  .period button:disabled {
    color: var(--label3);
  }
  .tiles {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    margin: 10px 16px 0;
  }
  .tile {
    display: grid;
    gap: 3px;
    padding: 12px 14px;
    border-radius: var(--radius-group);
    background: var(--card);
    box-shadow: var(--shadow-card);
  }
  .tile.wide {
    grid-column: span 2;
    gap: 8px;
  }
  .tile small {
    font-size: 13px;
    color: var(--label2);
  }
  .tile b {
    font-size: 21px;
    font-weight: 700;
  }
  .tile em {
    font-style: normal;
    font-size: 12px;
    color: var(--label2);
  }
  .tile em.up {
    color: var(--red);
  }
  .green {
    color: var(--green);
  }
  .red {
    color: var(--red);
  }
  .budget-head {
    display: flex;
    justify-content: space-between;
    font-size: 15px;
    font-weight: 600;
  }
  .cap {
    margin: 24px 32px 7px;
    font-size: 13px;
    font-weight: 500;
    color: var(--label2);
  }
  .card {
    margin: 0 16px;
    border-radius: var(--radius-group);
    background: var(--card);
    box-shadow: var(--shadow-card);
    overflow: hidden;
  }
  .card.pad {
    display: grid;
    gap: 14px;
    padding: 14px 16px;
  }
  .readout {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    padding: 14px 16px 0;
    font-size: 13px;
    color: var(--label2);
  }
  .readout b {
    font-size: 20px;
    color: var(--label);
  }
  .readout span {
    flex: 1;
  }
  .drill {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: 10px;
    padding: 4px 8px 4px 10px;
    border: 0;
    border-radius: 12px;
    background: color-mix(in srgb, var(--tint) 14%, transparent);
    color: var(--tint);
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    align-self: center;
  }
  .chart {
    position: relative;
    display: flex;
    align-items: stretch;
    height: 120px;
    margin: 14px 12px 0;
  }
  .col {
    flex: 1;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding: 0 1px;
    border: 0;
    background: none;
  }
  .col i {
    width: 100%;
    max-width: 10px;
    border-radius: 3px 3px 1px 1px;
    background: var(--tint);
    opacity: 0.75;
    transition: opacity 0.15s;
  }
  .col.future i {
    opacity: 0.15;
  }
  .col.on i {
    opacity: 1;
    background: var(--c2);
  }
  .avg {
    position: absolute;
    left: 0;
    right: 0;
    border-top: 1px dashed var(--label3);
    pointer-events: none;
    z-index: 1;
  }
  .avg span {
    position: absolute;
    right: 0;
    bottom: 2px;
    padding: 0 4px;
    border-radius: 4px;
    font-size: 11px;
    color: var(--label2);
    background: var(--card);
  }
  .axis {
    display: flex;
    justify-content: space-between;
    padding: 6px 12px 12px;
    font-size: 11px;
    color: var(--label3);
  }
  .ring {
    display: grid;
    place-items: center;
    padding: 20px 0 12px;
  }
  .ring small {
    font-size: 12px;
    color: var(--label2);
  }
  .ring b {
    font-size: 22px;
    font-weight: 700;
  }
  .cat {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 54px;
    padding: 8px 16px 8px 14px;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    transition: background-color 0.3s;
  }
  .cat:active,
  .cat.open {
    background: var(--press);
  }
  .cat + .cat::before,
  .subs + .cat::before {
    content: "";
    position: absolute;
    left: 58px;
    right: 0;
    top: 0;
    height: 0.5px;
    background: var(--sep);
  }
  .ce {
    flex: none;
    width: 32px;
    height: 32px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    font-size: 17px;
    background: linear-gradient(145deg, color-mix(in oklab, var(--c) 50%, white), var(--c));
  }
  .rank {
    flex: none;
    width: 32px;
    text-align: center;
    font-size: 20px;
    font-weight: 700;
    color: var(--label3);
  }
  .cn,
  .cv {
    display: grid;
    gap: 1px;
  }
  .cn {
    flex: 1;
    min-width: 0;
    font-size: 16px;
  }
  .cv {
    justify-items: end;
    font-size: 16px;
    font-weight: 600;
  }
  .cn small,
  .cv small {
    font-size: 12px;
    font-weight: 400;
    color: var(--label2);
  }
  .subs {
    display: grid;
    gap: 12px;
    padding: 6px 16px 14px 58px;
    background: var(--press);
  }
  .bar-row {
    display: grid;
    gap: 5px;
  }
  .bar-head {
    display: flex;
    justify-content: space-between;
    font-size: 14px;
  }
  .bar-head small {
    color: var(--label2);
  }
  .bar-track {
    position: relative;
    height: 6px;
    border-radius: 3px;
    background: var(--fill);
  }
  .bar-track.big {
    height: 10px;
    border-radius: 5px;
  }
  .bar-track i {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--tint);
    transition: width 0.5s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .bar-track i.fast {
    background: var(--c0);
  }
  .bar-track i.over {
    background: var(--red);
  }
  .bar-track b {
    position: absolute;
    top: -3px;
    bottom: -3px;
    width: 2px;
    margin-left: -1px;
    border-radius: 1px;
    background: var(--label);
  }
  .none {
    margin: 0;
    padding: 16px;
    text-align: center;
    color: var(--label2);
    font-size: 14px;
  }
  .tail {
    height: calc(env(safe-area-inset-bottom) + 40px);
  }
</style>
