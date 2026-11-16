<script lang="ts">
  import Icon from "../../ui/Icon.svelte";
  import Sheet from "../../ui/Sheet.svelte";
  import PrefEditor from "../../ui/PrefEditor.svelte";
  import { holdKeyboard } from "../../ui/nav.svelte";
  import { app, prefs, toast } from "../../lib/store.svelte";
  import { money } from "../../lib/format";
  import { setTheme, themeState } from "../../lib/theme.svelte";
  import { THEMES } from "../index";

  let collapsed = $state(false);
  let editing = $state<"cycle" | "budget" | null>(null);
  // 原型里模拟更新的几个阶段。
  const STAGES = ["拉取代码", "构建", "重启服务"];
  let stage = $state(-1);
  let updated = $state(false);

  function runUpdate() {
    if (stage >= 0) return;
    updated = false;
    stage = 0;
    const tick = () => {
      if (stage < STAGES.length - 1) {
        stage += 1;
        setTimeout(tick, 1100);
      } else {
        stage = -1;
        updated = true;
      }
    };
    setTimeout(tick, 1100);
  }
</script>

<div class="page" onscroll={(e) => (collapsed = e.currentTarget.scrollTop > 46)}>
  <header class="bar" class:collapsed><span>设置</span></header>
  <h1>设置</h1>

  <h3 class="cap">主题</h3>
  <div class="themes">
    {#each THEMES as t (t.id)}
      <button class="theme" class:on={themeState.id === t.id} onclick={() => setTheme(t.id)}>
        <div class="preview" data-theme={t.id}>
          <div class="p-hero"></div>
          {#each [0, 1, 2] as i (i)}
            <div class="p-row"><i style:background="var(--c{i})"></i><b></b><em></em></div>
          {/each}
        </div>
        <span class="head">
          <span class="name">{t.name}</span>
          <span class="radio">{#if themeState.id === t.id}<Icon name="check" size={13} weight={3.4} />{/if}</span>
        </span>
        <small>{t.blurb}</small>
      </button>
    {/each}
  </div>

  <h3 class="cap">记账</h3>
  <div class="group">
    <button class="row" onclick={() => (editing = "cycle")}>
      <span class="fi" style:--fc="var(--c2)"><Icon name="calendar" size={18} weight={2.2} /></span>
      <span class="label">记账周期</span>
      <span class="value">{prefs.cycleDay === 1 ? "自然月" : `每月 ${prefs.cycleDay} 号起`}</span>
      <span class="chev"><Icon name="chevron-right" size={14} weight={2.6} /></span>
    </button>
    <button class="row" onclick={() => { holdKeyboard("decimal"); editing = "budget"; }}>
      <span class="fi" style:--fc="var(--c0)"><Icon name="target" size={18} weight={2.2} /></span>
      <span class="label">每期预算</span>
      <span class="value num">{prefs.budget === null ? "不设" : money(prefs.budget)}</span>
      <span class="chev"><Icon name="chevron-right" size={14} weight={2.6} /></span>
    </button>
  </div>
  <p class="foot">首页的支出、预算和统计都按这个周期算。</p>

  <h3 class="cap">服务</h3>
  <div class="group">
    <button class="row" onclick={runUpdate}>
      <span class="fi" style:--fc="var(--c1)"><Icon name="update" size={18} weight={2.2} /></span>
      <span class="label">更新到最新版本</span>
      <span class="value">
        {#if stage >= 0}<span class="spin"></span>{STAGES[stage]}…{:else if updated}已是最新 · 0c3c136{/if}
      </span>
    </button>
    <div class="row">
      <span class="fi" style:--fc="var(--c7)"><Icon name="receipt" size={18} weight={2.2} /></span>
      <span class="label">账目</span>
      <span class="value num">{app.entries.length} 笔</span>
    </div>
  </div>

  <div class="group out">
    <button class="row center" onclick={() => toast("原型里不能退出")}>退出登录</button>
  </div>
  <div class="tail"></div>
</div>

<Sheet open={editing !== null} onclose={() => (editing = null)}>
  {#if editing}<PrefEditor which={editing} onclose={() => (editing = null)} />{/if}
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
    transition: background-color 0.2s;
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
    margin: -6px 20px 4px;
    font-size: 34px;
    font-weight: 700;
  }
  .cap {
    margin: 24px 32px 7px;
    font-size: 13px;
    font-weight: 500;
    color: var(--label2);
  }
  .themes {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    margin: 0 16px;
  }
  .theme {
    position: relative;
    display: grid;
    gap: 2px;
    padding: 10px 10px 12px;
    border: 0;
    border-radius: 18px;
    background: var(--card);
    color: var(--label);
    font: inherit;
    text-align: left;
    box-shadow: var(--shadow-card), 0 0 0 2px transparent;
    transition: box-shadow 0.25s, transform 0.25s;
  }
  .theme.on {
    box-shadow: var(--shadow-card), 0 0 0 2.5px var(--tint);
  }
  .theme:active {
    transform: scale(0.97);
  }
  .preview {
    display: grid;
    gap: 5px;
    padding: 8px;
    margin-bottom: 8px;
    border-radius: 12px;
    background: var(--bg);
  }
  .p-hero {
    height: 38px;
    border-radius: calc(var(--radius-group) * 0.6);
    background: linear-gradient(120deg, var(--c0), var(--c2), var(--c1));
  }
  .p-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px;
    border-radius: calc(var(--radius-group) * 0.45);
    background: var(--card);
  }
  .p-row i {
    width: 14px;
    height: 14px;
    border-radius: 5px;
  }
  .p-row b {
    flex: 1;
    height: 5px;
    border-radius: 3px;
    background: var(--label3);
  }
  .p-row em {
    width: 18px;
    height: 5px;
    border-radius: 3px;
    background: var(--tint);
  }
  .name {
    font-size: 16px;
    font-weight: 600;
  }
  .theme small {
    font-size: 12px;
    color: var(--label2);
    line-height: 1.35;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-right: 4px;
  }
  .radio {
    flex: none;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: #fff;
    box-shadow: inset 0 0 0 1.5px var(--label3);
  }
  .theme.on .radio {
    background: var(--tint);
    box-shadow: none;
  }
  .group {
    margin: 0 16px;
    border-radius: var(--radius-group);
    overflow: hidden;
    background: var(--card);
    box-shadow: var(--shadow-card);
  }
  .group.out {
    margin-top: 26px;
  }
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 48px;
    padding: 8px 14px;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    font-size: 16px;
    text-align: left;
    transition: background-color 0.35s;
  }
  button.row:active {
    background: var(--press);
    transition: none;
  }
  .group > * + *::before {
    content: "";
    position: absolute;
    left: 54px;
    right: 0;
    top: 0;
    height: 0.5px;
    background: var(--sep);
  }
  .fi {
    flex: none;
    width: 29px;
    height: 29px;
    border-radius: 8px;
    display: grid;
    place-items: center;
    color: #fff;
    background: linear-gradient(160deg, color-mix(in oklab, var(--fc) 80%, white), var(--fc));
  }
  .value {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
    color: var(--label2);
    font-size: 15px;
  }
  .chev {
    display: flex;
    color: var(--label3);
  }
  .foot {
    margin: 7px 32px 0;
    font-size: 13px;
    color: var(--label2);
  }
  .center {
    justify-content: center;
    color: var(--red);
  }
  .spin {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid var(--label3);
    border-top-color: var(--tint);
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .tail {
    height: calc(env(safe-area-inset-bottom) + 100px);
  }
</style>
