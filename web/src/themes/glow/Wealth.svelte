<script lang="ts">
  import Icon from "../../ui/Icon.svelte";
  import Sheet from "../../ui/Sheet.svelte";
  import SwipeRow from "../../ui/SwipeRow.svelte";
  import type { Action } from "../../ui/ActionSheet.svelte";
  import CardEditor from "../../ui/CardEditor.svelte";
  import { holdKeyboard } from "../../ui/nav.svelte";
  import { app, copy, makeDefault, openAccount, periodEntries, prefs, total } from "../../lib/store.svelte";
  import { money, periodLabel } from "../../lib/format";
  import { brand, cardLabel } from "../../lib/look";
  import type { Account } from "../../lib/types";

  let collapsed = $state(false);
  let editing = $state<Account | null>(null);

  const list = $derived(periodEntries(app.period));
  const spent = $derived(total(list, "expense"));
  const income = $derived(total(list, "income"));
  const label = $derived(periodLabel(app.period, prefs.cycleDay));
  const rows = $derived(
    app.catalog.accounts.map((a) => {
      const mine = list.filter((e) => e.account === a.name);
      return { a, spent: total(mine, "expense"), income: total(mine, "income"), count: mine.length };
    }),
  );
  const flow = $derived(rows.filter((r) => r.spent));

  function actions(a: Account): Action[] {
    const out: Action[] = [];
    // 左滑先露出最常用的复制卡号，其余进「更多」。
    if (a.card) out.push({ label: "复制卡号", run: () => copy(a.card!.number, "卡号") });
    out.push({ label: a.card ? "修改卡号" : "添加卡号", run: () => { holdKeyboard("numeric"); editing = a; } });
    if (app.catalog.accounts[0].name !== a.name) out.push({ label: "设为默认账户", run: () => makeDefault(a.name) });
    return out;
  }
</script>

<div class="page" onscroll={(e) => (collapsed = e.currentTarget.scrollTop > 46)}>
  <header class="bar" class:collapsed><span>财富</span></header>
  <h1>财富</h1>

  <section class="hero">
    <span class="cap">{label.title} · 经过各账户</span>
    <div class="two">
      <span><small>支出</small><b class="num">{money(spent)}</b></span>
      <span><small>收入</small><b class="num">{money(income)}</b></span>
    </div>
    {#if flow.length}
      <div class="split">
        {#each flow as r (r.a.name)}<i style:flex={r.spent} style:background={brand(r.a.name).bg}></i>{/each}
      </div>
    {/if}
  </section>

  <h3 class="cap2">账户</h3>
  <div class="group">
    {#each rows as r, i (r.a.name)}
      {@const b = brand(r.a.name)}
      <SwipeRow actions={actions(r.a)} title={r.a.name + (r.a.card ? ` · ${cardLabel(r.a.card)}` : "")}>
      <button class="row" onclick={() => openAccount(r.a.name)}>
        <span class="logo" style:background={b.bg}>{b.mark}</span>
        <span class="main">
          <span class="name">{r.a.name}{#if i === 0}<em>默认</em>{/if}</span>
          <span class="sub num">{r.a.card ? cardLabel(r.a.card) : "无卡号"}</span>
        </span>
        <span class="right">
          <span class="num">{r.spent ? money(r.spent) : "—"}</span>
          <small class="num">{r.count} 笔</small>
        </span>
        <span class="chev"><Icon name="chevron-right" size={14} weight={2.6} /></span>
      </button>
      </SwipeRow>
    {/each}
  </div>
  <p class="foot">点账户看它的账目；左滑可以复制、修改卡号，或设为默认账户（识别不出账户时用它）。余额和对账以后加在这里。</p>
  <div class="tail"></div>
</div>

<Sheet open={editing !== null} onclose={() => (editing = null)}>
  {#if editing}<CardEditor account={editing} onclose={() => (editing = null)} />{/if}
</Sheet>

<style>
  .page {
    position: absolute;
    inset: 0;
    overflow-y: auto;
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
  }
  .bar span {
    opacity: 0;
    transition: opacity 0.2s;
  }
  .bar.collapsed {
    background: var(--bar-bg);
    backdrop-filter: saturate(180%) blur(24px);
    -webkit-backdrop-filter: saturate(180%) blur(24px);
    box-shadow: 0 0.5px 0 var(--sep);
  }
  .bar.collapsed span {
    opacity: 1;
  }
  h1 {
    margin: -6px 20px 10px;
    font-size: 34px;
    font-weight: 700;
  }
  .hero {
    display: grid;
    gap: 12px;
    margin: 0 16px;
    padding: 18px 20px;
    border-radius: 24px;
    color: #fff;
    background:
      radial-gradient(100% 120% at 0% 0%, #2f7cf6 0%, transparent 60%),
      radial-gradient(90% 100% at 100% 100%, #13b98a 0%, transparent 60%),
      #13233f;
    box-shadow: 0 14px 30px -14px rgba(19, 90, 200, 0.6);
  }
  .cap {
    font-size: 13px;
    font-weight: 600;
    opacity: 0.85;
  }
  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
  }
  .two span {
    display: grid;
  }
  .two small {
    font-size: 13px;
    opacity: 0.75;
  }
  .two b {
    font-size: 24px;
    font-weight: 700;
  }
  .split {
    display: flex;
    gap: 2px;
    height: 8px;
    border-radius: 4px;
    overflow: hidden;
  }
  .split i {
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.25);
  }
  .cap2 {
    margin: 24px 32px 7px;
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
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 64px;
    padding: 10px 12px 10px 14px;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    transition: background-color 0.35s, transform 0.3s;
  }
  .row:active {
    background: var(--press);
    transition: background-color 0s, transform 0.45s;
    transform: scale(0.985);
  }
  .group :global(.swipe + .swipe .row::before) {
    content: "";
    position: absolute;
    left: 66px;
    right: 0;
    top: 0;
    height: 0.5px;
    background: var(--sep);
  }
  .logo {
    flex: none;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: #fff;
    font-size: 17px;
    font-weight: 700;
    box-shadow: inset 0 -2px 0 rgba(0, 0, 0, 0.12);
  }
  .main {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 2px;
  }
  .name {
    font-size: 16px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .name em {
    margin-left: 6px;
    padding: 1px 6px;
    border-radius: 6px;
    font-style: normal;
    font-size: 11px;
    color: var(--tint);
    background: color-mix(in srgb, var(--tint) 14%, transparent);
    vertical-align: 2px;
  }
  .sub {
    font-size: 13px;
    color: var(--label2);
  }
  .right {
    display: grid;
    justify-items: end;
    font-size: 15px;
    font-weight: 600;
  }
  .right small {
    font-size: 12px;
    font-weight: 400;
    color: var(--label3);
  }
  .chev {
    display: flex;
    color: var(--label3);
  }
  .foot {
    margin: 8px 32px 0;
    font-size: 13px;
    line-height: 1.45;
    color: var(--label2);
  }
  .tail {
    height: calc(env(safe-area-inset-bottom) + 100px);
  }
</style>
