<script lang="ts">
  import { fly, scale } from "svelte/transition";
  import { backOut } from "svelte/easing";
  import SwipeRow from "../../ui/SwipeRow.svelte";
  import Donut from "../../ui/Donut.svelte";
  import { app, back, breakdown, byDay, categoryIndex, currentPeriod, open, openStats, periodEntries, prefs, remove, total } from "../../lib/store.svelte";
  import { leaf, money, periodLabel, shiftPeriod, time } from "../../lib/format";
  import { brand, color, emoji, slices } from "../../lib/look";

  // 传了账户就是推入的账户明细页：只看这个账户的账目，不显示预算。
  let { account }: { account?: string } = $props();

  let dir = $state(1);
  const list = $derived(periodEntries(app.period, account));
  const days = $derived(byDay(list));
  const spent = $derived(total(list, "expense"));
  const last = $derived(total(periodEntries(shiftPeriod(app.period, -1), account), "expense"));
  const parts = $derived(breakdown(list));
  const label = $derived(periodLabel(app.period, prefs.cycleDay));
  const latest = $derived(app.period >= currentPeriod());
  const budget = $derived(account ? null : prefs.budget);
  const left = $derived(budget === null ? 0 : budget - spent);
  const WEEK = ["日", "一", "二", "三", "四", "五", "六"];

  const idx = (name: string) => Math.max(0, app.catalog.expense.findIndex((c) => c.name === name));
  const ring = $derived(slices(parts, idx, 4));
  const mood = $derived(budget !== null ? (left >= 0 ? "🥰" : "🫣") : !last ? "🐷" : spent <= last ? "🥰" : "🫣");
  const compare = $derived(
    !last ? "第一个月，慢慢攒～" : spent <= last ? `比上个月少花 ${money(last - spent)}` : `比上个月多花 ${money(spent - last)}`,
  );

  function go(delta: number) {
    dir = delta;
    app.period = shiftPeriod(app.period, delta);
  }
</script>

<div class="page">
  <header class="top">
    {#if account}<button class="back" onclick={back} aria-label="返回">‹</button>{/if}
    <div class="hello">
      <small>{label.sub}</small>
      <h1>{#if account}<span class="logo" style:background={brand(account).bg}>{brand(account).mark}</span>{account}{:else}{label.title}的小账本{/if}</h1>
    </div>
    <div class="pill">
      <button onclick={() => go(-1)} aria-label="上个月">‹</button>
      <button onclick={() => go(1)} disabled={latest} aria-label="下个月">›</button>
    </div>
  </header>

  {#key app.period}
    <div in:fly={{ x: 50 * dir, duration: 520, easing: backOut }}>
      <button class="piggy" onclick={openStats}>
        <span class="pig" in:scale={{ start: 0.4, duration: 600, easing: backOut }}>{mood}</span>
        <span class="cap">{prefs.cycleDay === 1 ? "这个月花了" : "这一期花了"} ›</span>
        <span class="big num">{money(spent)}</span>
        {#if budget !== null}
          <span class="jar"><span class="fill-b" class:over={left < 0} style:width="{Math.min(100, (spent / budget) * 100)}%"></span></span>
          <span class="cmp">{left >= 0 ? `预算还剩 ${money(left)}` : `超出预算 ${money(-left)} 啦`}</span>
        {:else}
          <span class="cmp">{compare}</span>
        {/if}
      </button>

      {#if ring.length}
        <button class="candies" onclick={openStats}>
          <Donut slices={ring} size={118} thickness={20}>
            <span class="donut-e">{emoji({ category: ring[0].name, subcategory: null })}</span>
          </Donut>
          <ul>
            {#each ring as p, i (p.name)}
              <li style:--c={p.color} in:scale={{ start: 0.6, delay: 60 * i, duration: 500, easing: backOut }}>
                <span class="ce">{emoji({ category: p.name, subcategory: null })}</span>
                <span class="cn">{p.name}</span>
                <b class="num">{Math.round((p.cents / spent) * 100)}%</b>
              </li>
            {/each}
          </ul>
        </button>
      {/if}

      {#each days as g (g.day)}
        {@const d = new Date(g.at)}
        <h3 class="sticker">
          <b class="num">{d.getDate()}</b>
          <span>周{WEEK[d.getDay()]}</span>
          {#if g.spent}<em class="num">-{money(g.spent)}</em>{/if}
        </h3>
        {#each g.entries as e (e.id)}
          <div class="card">
            <SwipeRow actions={[{ icon: "trash", label: "删除", danger: true, run: () => remove(e.id) }]}>
              <button class="row" onclick={() => open(e.id)}>
                <span class="bubble" style:--c={color(categoryIndex(e))}>{emoji(e)}</span>
                <span class="main">
                  <span class="title">{e.merchant ?? leaf(e)}{#if e.tags.includes("恋爱")} 💗{/if}</span>
                  <span class="chips"><i>{leaf(e)}</i><i>{e.account}</i><i class="num">{time(e.occurred_at)}</i></span>
                </span>
                <span class="amt num" class:income={e.kind === "income"}>{e.kind === "income" ? "+" : "-"}{money(e.amount_cents).slice(1)}</span>
              </button>
            </SwipeRow>
          </div>
        {/each}
      {:else}
        <div class="empty">
          <div class="e">🐣</div>
          <p>这个月还空空的</p>
          <small>付完钱轻点手机背面，我就帮你记下来</small>
        </div>
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
    -webkit-overflow-scrolling: touch;
    padding-top: calc(env(safe-area-inset-top) + 8px);
    background:
      radial-gradient(60% 30% at 100% 0%, rgba(255, 166, 193, 0.35), transparent),
      radial-gradient(50% 25% at 0% 8%, rgba(255, 214, 165, 0.45), transparent),
      var(--bg);
  }
  .back {
    align-self: center;
    width: 40px;
    height: 40px;
    margin-right: 10px;
    border: 0;
    border-radius: 50%;
    background: var(--card);
    color: var(--tint);
    font: inherit;
    font-size: 28px;
    font-weight: 800;
    line-height: 1;
    padding-bottom: 4px;
    box-shadow: 0 6px 14px -6px rgba(255, 111, 156, 0.5);
  }
  .hello {
    flex: 1;
    min-width: 0;
  }
  .logo {
    display: inline-grid;
    place-items: center;
    width: 30px;
    height: 30px;
    margin-right: 8px;
    border-radius: 50%;
    color: #fff;
    font-size: 15px;
    vertical-align: 3px;
  }
  .top {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    padding: 8px 20px 12px;
  }
  .hello small {
    font-size: 14px;
    font-weight: 700;
    color: var(--tint);
  }
  h1 {
    margin: 0;
    font-size: 28px;
    font-weight: 800;
  }
  .pill {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 999px;
    background: var(--card);
    box-shadow: 0 4px 12px -4px rgba(255, 111, 156, 0.3);
  }
  .pill button {
    width: 36px;
    height: 32px;
    border: 0;
    border-radius: 999px;
    background: none;
    color: var(--tint);
    font: inherit;
    font-size: 22px;
    font-weight: 800;
    line-height: 1;
  }
  .pill button:active {
    background: var(--fill);
    transform: scale(0.9);
  }
  .pill button:disabled {
    color: var(--label3);
  }

  .piggy {
    position: relative;
    display: grid;
    justify-items: start;
    width: calc(100% - 32px);
    border: 0;
    font: inherit;
    text-align: left;
    transition: transform 0.35s cubic-bezier(0.34, 1.56, 0.64, 1);
    margin: 4px 16px 0;
    padding: 20px 22px 18px;
    border-radius: 30px;
    background: linear-gradient(135deg, #ffd3e1 0%, #ffe6cc 55%, #fff1b8 100%);
    color: #5a3842;
    box-shadow: 0 16px 30px -16px rgba(255, 111, 156, 0.55), inset 0 2px 0 rgba(255, 255, 255, 0.7);
    overflow: hidden;
  }
  .piggy::after {
    content: "";
    position: absolute;
    right: -30px;
    bottom: -40px;
    width: 140px;
    height: 140px;
    border-radius: 50%;
    background: rgba(255, 255, 255, 0.35);
  }
  .pig {
    position: absolute;
    right: 20px;
    top: 16px;
    font-size: 54px;
    z-index: 1;
    animation: bob 3s ease-in-out infinite;
  }
  @keyframes bob {
    50% {
      transform: translateY(-6px) rotate(6deg);
    }
  }
  .piggy:active,
  .candies:active {
    transform: scale(0.97);
  }
  .piggy > span:not(.pig) {
    position: relative;
    z-index: 1;
  }
  .jar {
    justify-self: stretch;
    height: 12px;
    margin: 8px 70px 0 0;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.65);
    overflow: hidden;
  }
  .fill-b {
    display: block;
    height: 100%;
    border-radius: 6px;
    background: linear-gradient(90deg, #ffa6c1, var(--tint));
  }
  .fill-b.over {
    background: linear-gradient(90deg, #ff9a7a, #ff5a5a);
  }
  .cap {
    font-size: 14px;
    font-weight: 700;
    opacity: 0.75;
  }
  .big {
    font-size: 38px;
    font-weight: 800;
    letter-spacing: -0.01em;
  }
  .cmp {
    display: inline-block;
    margin-top: 8px;
    padding: 4px 10px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.6);
    font-size: 13px;
    font-weight: 700;
  }

  .candies {
    display: flex;
    align-items: center;
    gap: 16px;
    width: calc(100% - 32px);
    margin: 14px 16px 0;
    padding: 16px;
    border: 0;
    border-radius: var(--radius-group);
    background: var(--card);
    color: inherit;
    font: inherit;
    text-align: left;
    box-shadow: 0 8px 20px -12px rgba(255, 111, 156, 0.35);
    transition: transform 0.35s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .donut-e {
    font-size: 34px;
    animation: bob 3s ease-in-out infinite;
  }
  .candies ul {
    flex: 1;
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .candies li {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px 4px 4px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--c) 16%, var(--card));
    font-size: 13px;
    font-weight: 700;
  }
  .ce {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 14px;
    background: var(--card);
  }
  .cn {
    flex: 1;
  }
  .candies li b {
    color: var(--label2);
  }

  .sticker {
    display: flex;
    align-items: baseline;
    gap: 6px;
    margin: 22px 22px 8px;
    font-size: 14px;
    font-weight: 700;
    color: var(--label2);
  }
  .sticker b {
    font-size: 24px;
    font-weight: 800;
    color: var(--label);
  }
  .sticker em {
    margin-left: auto;
    font-style: normal;
    font-size: 14px;
  }
  .card {
    margin: 0 16px 10px;
    border-radius: var(--radius-group);
    overflow: hidden;
    box-shadow: 0 6px 18px -10px rgba(255, 111, 156, 0.4);
    --row-bg: var(--card);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 12px 16px 12px 12px;
    border: 0;
    background: var(--card);
    color: inherit;
    font: inherit;
    text-align: left;
    transition: transform 0.35s cubic-bezier(0.34, 1.56, 0.64, 1), background-color 0.3s;
  }
  .row:active {
    transform: scale(0.97);
    background: color-mix(in srgb, var(--card), var(--tint) 6%);
  }
  .bubble {
    flex: none;
    width: 46px;
    height: 46px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 24px;
    background: color-mix(in srgb, var(--c) 28%, var(--card));
    box-shadow: inset 0 -3px 0 color-mix(in srgb, var(--c) 35%, transparent);
  }
  .main {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 5px;
  }
  .title {
    font-size: 16px;
    font-weight: 800;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .chips {
    display: flex;
    gap: 4px;
    overflow: hidden;
  }
  .chips i {
    flex: none;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--fill);
    font-style: normal;
    font-size: 11px;
    font-weight: 700;
    color: var(--label2);
  }
  .amt {
    font-size: 17px;
    font-weight: 800;
  }
  .amt.income {
    color: var(--green);
  }
  .empty {
    display: grid;
    justify-items: center;
    gap: 6px;
    padding: 60px 30px;
    text-align: center;
  }
  .empty .e {
    font-size: 60px;
    animation: bob 2.4s ease-in-out infinite;
  }
  .empty p {
    margin: 0;
    font-size: 18px;
    font-weight: 800;
  }
  .empty small {
    font-size: 14px;
    color: var(--label2);
  }
  .tail {
    height: calc(env(safe-area-inset-bottom) + 110px);
  }
</style>
