<script lang="ts">
  import { app, canSelectPeriod, openStats, periodStatus, settings, want } from "../lib/store.svelte";
  import { breakdown, budgetOf, earning, periodEntries, recentSpend, spendIndex, spending } from "../lib/query";
  import Icon from "./Icon.svelte";
  import { badge, comparisonEnds, isYear, money, periodOf, periodRange, shiftPeriod } from "../lib/format";
  import { color } from "../lib/look";
  const list = $derived(periodEntries(app.period));
  const spent = $derived(spending(list).cents);
  const income = $derived(earning(list).cents);
  const budget = $derived(budgetOf(app.period));
  const parts = $derived(breakdown(list));
  const scale = $derived(Math.max(1, spent, budget ?? 0));
  const label = $derived(isYear(app.period) ? "全年" : settings.cycle_day === 1 ? "本月" : "本期");
  const previousPeriod = $derived(shiftPeriod(app.period, -1));
  const change = $derived.by(() => {
    if (periodStatus(previousPeriod) !== "ready") return null;
    const end = comparisonEnds(app.period, settings.cycle_day);
    const current = spending(list.filter((e) => new Date(e.occurred_at) < end.current)).cents;
    const previous = spending(periodEntries(previousPeriod).filter((e) => new Date(e.occurred_at) < end.previous)).cents;
    return previous ? Math.round(((current - previous) / previous) * 100) : null;
  });
  const comparisonLabel = $derived(isYear(app.period) ? "较去年同期" : "较上月同期");
  // 走势按最近 30 天取数；当期以外只补它覆盖的月周期。
  const trendEnd = $derived(new Date(Math.min(Date.now(), periodRange(app.period, settings.cycle_day).end.getTime() - 1)));
  const trendPeriods = $derived.by(() => {
    const first = new Date(trendEnd);
    first.setDate(first.getDate() - 29);
    const stop = periodOf(trendEnd.toISOString(), settings.cycle_day);
    const periods: string[] = [];
    for (let key = periodOf(first.toISOString(), settings.cycle_day); key <= stop; key = shiftPeriod(key, 1)) {
      if (canSelectPeriod(key)) periods.push(key);
    }
    return periods;
  });
  const trendReady = $derived(trendPeriods.every((period) => periodStatus(period) === "ready"));
  $effect(() => {
    const periods = [...new Set([previousPeriod, ...trendPeriods])].filter(canSelectPeriod);
    const timer = setTimeout(() => periods.forEach((period) => void want(period)), 150);
    return () => clearTimeout(timer);
  });

  const trend = $derived.by(() => {
    const days = recentSpend(trendEnd, 30);
    const peak = Math.max(1, ...days.map((d) => d.cents));
    const avg = days.reduce((sum, d) => sum + d.cents, 0) / days.length;
    return days.map((d, i) => ({ ...d, x: 4 + (i / 29) * 132, y: 65 - (d.cents / peak) * 54, high: d.cents > avg * 2 }));
  });
  const path = $derived(trend.map((d, i) => `${i ? "L" : "M"}${d.x},${d.y}`).join(" "));
  const md = (d: Date) => `${d.getMonth() + 1}.${d.getDate()}`;
</script>

<button class="summary" onclick={() => openStats()} aria-label="查看收支、预算及分类统计">
  <div class="overview">
    <div class="totals">
      <small class="go">{label}支出<Icon name="chevron-right" size={11} weight={2.6} /></small><b class="num">{money(spent)}</b><span
        class="change"
        class:positive={change !== null && change < 0}
        class:negative={change !== null && change > 0}
        >{comparisonLabel}{#if canSelectPeriod(previousPeriod) && periodStatus(previousPeriod) !== "ready"}
          —{:else if change === null}
          暂无对比{:else if change === 0}
          持平{:else}<Icon name={change > 0 ? "arrow-up" : "arrow-down"} size={11} weight={2.4} />{Math.abs(change)}%{/if}</span
      ><span class="surplus"
        >收支结余 <strong class="num" class:positive={income > spent} class:negative={income < spent}>{money(income - spent, true)}</strong></span
      >
    </div>
    <div class="trend">
      {#if trendReady}
        <small>近 30 天支出</small><svg viewBox="0 0 140 76" role="img" aria-label="每日支出趋势，超过日均两倍的高消费标红"
          ><path class="baseline" d="M4 65H136" /><path class="line" d={path} />{#each trend as d, i (i)}{#if i > 0 && (d.high || trend[i - 1].high)}<path
                class="high-line"
                d={`M${trend[i - 1].x},${trend[i - 1].y} L${d.x},${d.y}`}
              />{/if}{#if d.high}<circle cx={d.x} cy={d.y} r="2.8"><title>{md(d.date)} · {money(d.cents)}</title></circle>{/if}{/each}</svg
        ><span class="dates num"><span>{md(trend[0].date)}</span><span>{md(trend[29].date)}</span></span>
      {:else}<small>近 30 天支出</small>{/if}
    </div>
  </div>
  <div class="budget-head">
    <span>{budget === null ? "消费构成" : `预算 ${money(budget)}`}</span><span
      class="num"
      class:positive={budget !== null && spent < budget}
      class:negative={budget !== null && spent > budget}
      >{budget === null ? `${spending(list).count} 笔支出` : spent <= budget ? `还可花 ${money(budget - spent)}` : `超出 ${money(spent - budget)}`}</span
    >
  </div>
  <div class="composition" aria-label="分类消费占预算的比例">
    <div class="segments">
      {#each parts as p (p.name)}<i style:width={`${(p.cents / scale) * 100}%`} style:background={color(spendIndex(p.name))}></i>{/each}
    </div>
    {#if budget !== null && spent > budget}<span class="overflow" style:left={`${(budget / scale) * 100}%`}></span><b
        class="limit"
        style:left={`${(budget / scale) * 100}%`}
        aria-label="预算分界"
      ></b>{/if}
  </div>
  <div class="legend">
    {#each parts as p (p.name)}<span><i style:background={color(spendIndex(p.name))}></i>{badge(p.name).text}</span>{/each}{#if !parts.length}<span
        >还没有支出</span
      >{/if}<span class="details go">看详情<Icon name="chevron-right" size={10} weight={2.6} /></span>
  </div>
</button>

<style>
  .summary {
    display: block;
    width: calc(100% - 32px);
    margin: 4px 16px 0;
    padding: 18px;
    border: 0;
    border-radius: calc(var(--radius-group) + 10px);
    color: var(--hero-fg);
    background: var(--hero-bg);
    box-shadow: var(--shadow-card);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .overview {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr);
    gap: 14px;
    align-items: center;
    margin-bottom: 18px;
  }
  .totals {
    display: grid;
    gap: 4px;
    min-width: 0;
  }
  small {
    font-size: 12px;
    opacity: 0.75;
  }
  .totals > b {
    font-size: clamp(24px, 7vw, 32px);
    font-weight: 700;
    letter-spacing: -0.03em;
    overflow-wrap: anywhere;
  }
  .totals > small:first-child {
    font-size: 13px;
  }
  .surplus {
    font-size: 12px;
    margin-top: 6px;
    display: flex;
    flex-wrap: wrap;
    gap: 4px 8px;
  }
  .surplus strong {
    font-weight: 600;
  }
  .change {
    font-size: 11px;
    margin-top: 3px;
  }
  .positive {
    color: var(--hero-good);
  }
  .negative {
    color: var(--hero-bad);
  }
  .go,
  .change {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  .trend {
    min-width: 0;
  }
  .trend > small {
    display: block;
  }
  svg {
    width: 100%;
    display: block;
    overflow: visible;
  }
  .line,
  .baseline,
  .high-line {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .baseline {
    opacity: 0.15;
    stroke-width: 0.5;
  }
  .high-line {
    stroke: var(--hero-alert);
    stroke-width: 2.2;
  }
  circle {
    fill: var(--hero-alert);
    stroke: none;
  }
  .dates {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    opacity: 0.6;
  }
  .budget-head {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 11px;
    opacity: 0.85;
    margin-bottom: 8px;
  }
  .composition {
    position: relative;
    height: 10px;
    border-radius: 5px;
    background: var(--hero-track);
    margin: 12px 0;
  }
  .segments {
    display: flex;
    height: 100%;
    border-radius: inherit;
    overflow: hidden;
  }
  .composition i {
    height: 100%;
    flex: none;
  }
  .composition .limit {
    position: absolute;
    top: -3px;
    bottom: -3px;
    width: 2px;
    transform: translateX(-1px);
    border-radius: 1px;
    background: color-mix(in srgb, var(--hero-alert) 45%, var(--hero-fg));
  }
  .overflow {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    border-radius: 0 5px 5px 0;
    background: repeating-linear-gradient(120deg, transparent 0 5px, color-mix(in srgb, var(--hero-alert) 22%, transparent) 5px 7px);
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 5px 10px;
    margin-top: 9px;
    font-size: 10px;
    opacity: 0.8;
    align-items: center;
  }
  .legend span {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .legend i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }
  .details {
    margin-left: auto;
  }
</style>
