<script lang="ts">
  import Icon from "../../ui/Icon.svelte";
  import Sheet from "../../ui/Sheet.svelte";
  import SwipeRow, { type SwipeAction } from "../../ui/SwipeRow.svelte";
  import AccountEditor from "../../ui/AccountEditor.svelte";
  import AdjustEditor from "../../ui/AdjustEditor.svelte";
  import AccountLogo from "../../ui/AccountLogo.svelte";
  import { holdKeyboard } from "../../ui/nav.svelte";
  import { Reorder } from "../../ui/reorder.svelte";
  import { fade } from "svelte/transition";
  import { app, copy, moveAccount, openAccount, toggleBalance } from "../../lib/store.svelte";
  import { balanceOf } from "../../lib/query";
  import { money } from "../../lib/format";
  import { brand, cardLabel } from "../../lib/look";
  import type { Account } from "../../lib/types";

  let collapsed = $state(false);
  /** 正在编辑的账户；`"new"` 是新建。 */
  let editing = $state<Account | "new" | null>(null);
  /** 正在平账的账户。 */
  let adjusting = $state<Account | null>(null);
  const reorder = new Reorder(moveAccount);

  // 财富页只看此刻：净资产和各账户余额，不随周期变；某段时间的收支点进账户看。
  const bal = (a: Account) => balanceOf(a.id);
  const assets = $derived(app.catalog.accounts.filter((a) => bal(a) > 0));
  const plus = $derived(assets.reduce((s, a) => s + bal(a), 0));
  const minus = $derived(app.catalog.accounts.reduce((s, a) => s + Math.min(0, bal(a)), 0));
  /** 点资产条上的一段，大数字换成那个账户的余额。 */
  let focus = $state<Account | null>(null);
  const shown = $derived(focus ? bal(focus) : plus + minus);
  const mask = (text: string) => (app.hideBalance ? "••••••" : text);

  function actions(a: Account): SwipeAction[] {
    const out: SwipeAction[] = [];
    if (a.card) out.push({ icon: "copy", label: "复制卡号", run: () => copy(a.card!.number, "卡号") });
    out.push({
      icon: "scale",
      label: "平账",
      run: () => {
        holdKeyboard("decimal");
        adjusting = a;
      },
    });
    out.push({ icon: "pencil", label: "编辑账户", run: () => (editing = a) });
    return out;
  }
</script>

<div class="page" onscroll={(e) => (collapsed = e.currentTarget.scrollTop > 46)}>
  <header class="bar" class:collapsed><span>财富</span></header>
  <h1 class="title">财富</h1>

  <section class="worth" onclick={() => (focus = null)} role="presentation">
    <span class="glass"></span>
    <div class="cap">
      <span>{focus ? focus.name : "净资产"}</span>
      <button
        class="eye"
        onclick={(e) => {
          e.stopPropagation();
          toggleBalance();
        }}
        aria-label={app.hideBalance ? "显示金额" : "隐藏金额"}
      >
        <Icon name={app.hideBalance ? "eye-slash" : "eye"} size={18} weight={2} />
      </button>
    </div>
    {#key `${focus?.id}${app.hideBalance}`}
      <b class="big num" class:neg={shown < 0} in:fade={{ duration: 220 }}>{mask(money(shown, shown < 0))}</b>
    {/key}
    <span class="when">{focus ? `占资产 ${plus ? Math.round((Math.max(0, bal(focus)) / plus) * 100) : 0}% · 点空白处回到净资产` : "截至此刻"}</span>
    {#if assets.length}
      <div class="stack">
        {#each assets as a (a.id)}
          <button
            class:dim={focus && focus.id !== a.id}
            style:flex={bal(a)}
            style:background={brand(a).bg}
            onclick={(e) => {
              e.stopPropagation();
              focus = focus?.name === a.name ? null : a;
            }}
            aria-label={a.name}
          ></button>
        {/each}
      </div>
    {/if}
    <div class="two">
      <span><small>资产</small><b class="num">{mask(money(plus))}</b></span>
      <span><small>负债</small><b class="num">{mask(money(-minus))}</b></span>
    </div>
  </section>

  <h3 class="cap2">
    账户<button
      class="add"
      onclick={() => {
        holdKeyboard();
        editing = "new";
      }}><Icon name="plus" size={15} weight={2.4} />添加</button
    >
  </h3>
  <div class="group" class:sorting={reorder.from >= 0} {@attach reorder.attach}>
    {#each app.catalog.accounts as a, i (a.id)}
      <div class="item" class:lifted={reorder.from === i} style:transform="translateY({reorder.shift(i)}px)">
        <SwipeRow actions={actions(a)}>
          <button class="row" onclick={() => openAccount(a.id)}>
            <AccountLogo account={a} />
            <span class="main">
              <span class="name"
                >{a.name}{#if i === 0}<em>默认</em>{/if}</span
              >
              <span class="sub num">{a.card ? cardLabel(a.card) : "无卡号"}</span>
            </span>
            <span class="right num" class:owe={bal(a) < 0}>{mask(money(bal(a), bal(a) < 0))}</span>
            <span class="chev"><Icon name="chevron-right" size={14} weight={2.6} /></span>
          </button>
        </SwipeRow>
      </div>
    {/each}
  </div>
  <p class="foot">长按拖动排序；左滑平账，填实际余额，差额记一笔平账</p>
  <div class="tail"></div>
</div>

<Sheet open={editing !== null} onclose={() => (editing = null)}>
  {#if editing}<AccountEditor account={editing === "new" ? null : editing} onclose={() => (editing = null)} />{/if}
</Sheet>

<Sheet open={adjusting !== null} onclose={() => (adjusting = null)}>
  {#if adjusting}<AdjustEditor account={adjusting} onclose={() => (adjusting = null)} />{/if}
</Sheet>

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
  .title {
    margin: -6px 20px 10px;
    font-size: 34px;
    font-weight: 700;
  }
  .worth {
    position: relative;
    display: grid;
    gap: 6px;
    margin: 0 16px;
    padding: 16px 20px 18px;
    border-radius: 24px;
    overflow: hidden;
    color: #fff;
    background: radial-gradient(100% 120% at 0% 0%, #2f7cf6 0%, transparent 60%), radial-gradient(90% 100% at 100% 100%, #13b98a 0%, transparent 60%), #13233f;
    box-shadow: 0 14px 30px -14px rgba(19, 90, 200, 0.6);
  }
  .glass {
    position: absolute;
    inset: 0;
    background: linear-gradient(170deg, rgba(255, 255, 255, 0.16), transparent 45%);
    pointer-events: none;
  }
  .worth .cap {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 13px;
    font-weight: 600;
  }
  .worth .cap span {
    opacity: 0.9;
  }
  .eye {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    margin: -6px -8px -6px 0;
    border: 0;
    border-radius: 50%;
    background: none;
    color: #fff;
    opacity: 0.85;
  }
  .eye:active {
    background: rgba(255, 255, 255, 0.15);
  }
  .big {
    grid-area: auto;
    font-size: 36px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .big.neg {
    color: #ffb4ab;
  }
  .when {
    font-size: 12px;
    opacity: 0.7;
  }
  .stack {
    display: flex;
    gap: 2px;
    height: 10px;
    margin: 8px 0 6px;
  }
  .stack button {
    position: relative;
    min-width: 6px;
    padding: 0;
    border: 0;
    border-radius: 3px;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.3);
    transition:
      opacity 0.25s,
      transform 0.25s;
  }
  /* 条很细，点击区域上下放大。 */
  .stack button::before {
    content: "";
    position: absolute;
    inset: -12px 0;
  }
  .stack button.dim {
    opacity: 0.3;
  }
  .stack button:not(.dim):active {
    transform: scaleY(1.4);
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
    font-size: 20px;
    font-weight: 700;
  }
  .cap2 {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    margin: 24px 32px 7px;
    font-size: 13px;
    font-weight: 500;
    color: var(--label2);
  }
  .add {
    display: flex;
    align-items: center;
    gap: 3px;
    margin: 0 -4px -3px 0;
    padding: 2px 4px;
    border: 0;
    background: none;
    color: var(--tint);
    font: inherit;
    font-size: 15px;
    font-weight: 500;
  }
  .group {
    margin: 0 16px;
    border-radius: var(--radius-group);
    overflow: hidden;
    background: var(--card);
    box-shadow: var(--shadow-card);
    /* 长按是拿起排序，不弹系统菜单、不选中文字。 */
    -webkit-touch-callout: none;
    user-select: none;
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
    transition:
      background-color 0.35s,
      transform 0.3s;
  }
  .row:active {
    background: var(--press);
    transition:
      background-color 0s,
      transform 0.45s;
    transform: scale(0.985);
  }
  .item {
    position: relative;
    background: var(--card);
  }
  .sorting .item {
    transition: transform 0.25s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .sorting .item.lifted {
    z-index: 2;
    transition: none;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  }
  .item + .item :global(.row::before) {
    content: "";
    position: absolute;
    left: 66px;
    right: 0;
    top: 0;
    height: 0.5px;
    background: var(--sep);
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
  .right.owe {
    color: var(--red);
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
