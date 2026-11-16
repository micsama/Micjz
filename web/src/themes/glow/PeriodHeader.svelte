<script lang="ts">
  // 周期标题：左边是正在看的月或年，点开跳转面板；右边切月/年、翻前后。
  // 账目、账户明细、统计共用 `app.period`，在一页切了别处跟着变。
  import { scale } from "svelte/transition";
  import { quintOut } from "svelte/easing";
  import Icon from "../../ui/Icon.svelte";
  import { app, currentPeriod, isLatest, monthTotals, prefs, setUnit } from "../../lib/store.svelte";
  import { isYear, periodLabel, shiftPeriod } from "../../lib/format";

  /**
   * `onshift` 拿到切换方向（-1 往前、1 往后、0 换单位），页面用它决定内容怎么进场。
   * `account` 让面板里的金额只算这个账户。
   */
  let { onshift, account }: { onshift?: (delta: number) => void; account?: string } = $props();

  const year = $derived(isYear(app.period));
  const label = $derived(periodLabel(app.period, prefs.cycleDay));
  const latest = $derived(isLatest(app.period));

  let open = $state(false);
  /** 面板里正在看的年份，只在按月时用。 */
  let shown = $state(0);
  const totals = $derived(open ? monthTotals(account) : new Map<string, { spent: number; count: number }>());
  const thisYear = $derived(Number(currentPeriod(true)));
  const firstYear = $derived(Math.min(thisYear, ...[...totals.keys()].map((k) => Number(k.slice(0, 4)))));
  const years = $derived(Array.from({ length: thisYear - firstYear + 1 }, (_, i) => String(thisYear - i)));

  const yearTotal = (y: string) =>
    [...totals].filter(([k]) => k.startsWith(`${y}-`)).reduce((s, [, t]) => ({ spent: s.spent + t.spent, count: s.count + t.count }), { spent: 0, count: 0 });

  /** 面板里的金额只要量级，万以上用「万」。 */
  const brief = (cents: number) => {
    const y = cents / 100;
    return y >= 10000 ? `${(y / 10000).toFixed(1)}万` : `¥${Math.round(y).toLocaleString("en-US")}`;
  };

  function go(delta: number) {
    onshift?.(delta);
    app.period = shiftPeriod(app.period, delta);
  }

  function unit(y: boolean) {
    if (y === year) return;
    onshift?.(0);
    setUnit(y);
    open = false;
  }

  function toggle() {
    shown = Number(app.period.slice(0, 4));
    open = !open;
  }

  function jump(key: string) {
    open = false;
    if (key === app.period) return;
    onshift?.(key > app.period ? 1 : -1);
    app.period = key;
  }
</script>

<div class="large">
  <button class="title" class:open onclick={toggle} aria-expanded={open} aria-label="跳到其他{year ? '年份' : '月份'}">
    <h1>{label.title}</h1>
    {#if label.range || !year}<small class="num">{label.range || label.year}</small>{/if}
    <span class="caret"><Icon name="chevron-down" size={15} weight={2.8} /></span>
  </button>
  <div class="ctrl">
    <div class="seg" class:right={year}>
      <button class:on={!year} onclick={() => unit(false)}>月</button>
      <button class:on={year} onclick={() => unit(true)}>年</button>
    </div>
    <button class="arrow" onclick={() => go(-1)} aria-label={year ? "上一年" : "上个周期"}><Icon name="chevron-left" size={17} weight={2.6} /></button>
    <button class="arrow" onclick={() => go(1)} disabled={latest} aria-label={year ? "下一年" : "下个周期"}><Icon name="chevron-right" size={17} weight={2.6} /></button>
  </div>

  {#if open}
    <button class="scrim" onclick={() => (open = false)} aria-label="关闭"></button>
    <div class="picker" transition:scale={{ start: 0.9, duration: 260, easing: quintOut }}>
      {#if year}
        <div class="grid years">
          {#each years as y (y)}
            {@const t = yearTotal(y)}
            <button class:on={y === app.period} class:now={y === String(thisYear)} onclick={() => jump(y)}>
              <b>{y}年</b><small class="num">{t.count ? brief(t.spent) : "—"}</small>
            </button>
          {/each}
        </div>
      {:else}
        <div class="yr">
          <button onclick={() => shown--} disabled={shown <= firstYear} aria-label="上一年"><Icon name="chevron-left" size={15} weight={2.6} /></button>
          <b class="num">{shown}年</b>
          <button onclick={() => shown++} disabled={shown >= thisYear} aria-label="下一年"><Icon name="chevron-right" size={15} weight={2.6} /></button>
        </div>
        <div class="grid">
          {#each Array.from({ length: 12 }, (_, i) => `${shown}-${String(i + 1).padStart(2, "0")}`) as k, i (k)}
            {@const t = totals.get(k)}
            <button class:on={k === app.period} class:now={k === currentPeriod()} disabled={k > currentPeriod()} onclick={() => jump(k)}>
              <b>{i + 1}月</b><small class="num">{t ? brief(t.spent) : "—"}</small>
            </button>
          {/each}
        </div>
      {/if}
      <p>{year ? "下面是每年的支出" : "下面是每月的支出"}{prefs.cycleDay === 1 ? "" : `，每期从 ${prefs.cycleDay} 号起`}</p>
    </div>
  {/if}
</div>

<style>
  .large {
    position: relative;
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 8px;
    padding: 0 16px 10px 20px;
    margin-top: -6px;
  }
  .title {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
    padding: 0;
    border: 0;
    background: none;
    color: var(--label);
    font: inherit;
    text-align: left;
    transition: opacity 0.2s;
  }
  .title:active {
    opacity: 0.6;
  }
  h1 {
    margin: 0;
    font-size: 34px;
    font-weight: 700;
    letter-spacing: 0.01em;
    white-space: nowrap;
  }
  .title small {
    font-size: 17px;
    font-weight: 500;
    color: var(--label2);
    white-space: nowrap;
  }
  .caret {
    display: flex;
    align-self: center;
    margin-top: 4px;
    color: var(--tint);
    transition: transform 0.3s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .open .caret {
    transform: rotate(180deg);
  }

  .ctrl {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    padding-bottom: 4px;
  }
  .seg {
    position: relative;
    display: grid;
    grid-template-columns: 1fr 1fr;
    width: 72px;
    height: 32px;
    padding: 2px;
    margin-right: 4px;
    border-radius: 16px;
    background: var(--fill);
  }
  /* 滑块跟着选中项移动。 */
  .seg::before {
    content: "";
    position: absolute;
    top: 2px;
    bottom: 2px;
    left: 2px;
    width: calc(50% - 2px);
    border-radius: 14px;
    background: var(--card);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.14);
    transition: transform 0.3s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .seg.right::before {
    transform: translateX(100%);
  }
  .seg button {
    position: relative;
    border: 0;
    background: none;
    color: var(--label2);
    font: inherit;
    font-size: 14px;
    font-weight: 600;
    transition: color 0.2s;
  }
  .seg button.on {
    color: var(--label);
  }
  .arrow {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border: 0;
    display: grid;
    place-items: center;
    background: var(--fill);
    color: var(--tint);
    transition: transform 0.2s;
  }
  .arrow:disabled {
    color: var(--label3);
  }
  .arrow:active {
    transform: scale(0.88);
  }

  .scrim {
    position: fixed;
    inset: 0;
    z-index: 30;
    border: 0;
    background: rgba(0, 0, 0, 0.12);
  }
  .picker {
    position: absolute;
    top: calc(100% - 2px);
    left: 12px;
    z-index: 31;
    width: min(320px, calc(100% - 24px));
    padding: 10px;
    border-radius: 20px;
    background: var(--card);
    box-shadow: 0 18px 50px -12px rgba(0, 0, 0, 0.35), 0 0 0 0.5px var(--sep);
    transform-origin: 24px 0;
  }
  .yr {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 2px 8px;
  }
  .yr b {
    font-size: 15px;
  }
  .yr button {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 50%;
    background: none;
    color: var(--tint);
  }
  .yr button:disabled {
    color: var(--label3);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 4px;
  }
  .grid.years {
    grid-template-columns: repeat(3, 1fr);
  }
  .grid button {
    display: grid;
    gap: 1px;
    padding: 8px 0 7px;
    border: 0;
    border-radius: 12px;
    background: none;
    color: var(--label);
    font: inherit;
    transition: background-color 0.2s, transform 0.2s;
  }
  .grid button:active {
    transform: scale(0.94);
    background: var(--press);
  }
  .grid b {
    font-size: 15px;
    font-weight: 600;
  }
  .grid small {
    font-size: 11px;
    color: var(--label2);
  }
  .grid button.now b {
    color: var(--tint);
  }
  .grid button.on {
    background: var(--tint);
  }
  .grid button.on b,
  .grid button.on small {
    color: #fff;
  }
  .grid button:disabled {
    opacity: 0.3;
  }
  .picker p {
    margin: 8px 4px 0;
    font-size: 12px;
    color: var(--label3);
  }
</style>
