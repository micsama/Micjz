<script lang="ts">
  import Icon from "../../ui/Icon.svelte";
  import ActionSheet from "../../ui/ActionSheet.svelte";
  import ImageViewer from "../../ui/ImageViewer.svelte";
  import { app, back, edit, remove } from "../../lib/store.svelte";
  import { changes, fieldsOf, labelOf } from "../../lib/fields";
  import { dateTime } from "../../lib/format";
  import { view } from "../../lib/view";
  import type { Entry } from "../../lib/types";
  import Mark from "../../ui/Mark.svelte";

  let { entry }: { entry: Entry } = $props();

  let scrolled = $state(false);
  let confirm = $state(false);
  let viewing = $state(false);

  const v = $derived(view(entry));
  const shot = $derived(entry.recognition_id ? app.recognitions[entry.recognition_id] : undefined);
  const diff = $derived(shot ? changes(entry, shot) : []);
  const rows = $derived(fieldsOf(entry).filter((f) => f.key !== "amount_cents"));
  const tint = ["--c1", "--c4", "--c3", "--c0", "--c5", "--c2", "--c7"];
</script>

<div class="page" style:--c={v.color} onscroll={(e) => (scrolled = e.currentTarget.scrollTop > 120)}>
  <header class="bar" class:scrolled>
    <button class="back" onclick={back}><Icon name="chevron-left" size={22} weight={2.6} />账目</button>
    <span class="t">{v.title}</span>
    <span></span>
  </header>

  <section class="hero">
    <div class="aura"></div>
    <span class="icon"><Mark mark={v.mark} /></span>
    <h1>{v.title}</h1>
    <button class="amount num" class:income={v.tone === "in"} onclick={() => edit(entry.id, "amount_cents")}>{v.amount}</button>
    <p>{dateTime(entry.occurred_at)}</p>
  </section>

  <div class="group">
    {#each rows as f, i (f.key)}
      <button class="row" onclick={() => edit(entry.id, f.key)}>
        <span class="fi" style:--fc="var({tint[i]})"><Icon name={f.icon} size={18} weight={2.2} /></span>
        <span class="label">{labelOf(f, entry)}</span>
        <span class="value" class:empty={!f.show(entry)}>{f.show(entry) || "未填写"}</span>
        <span class="chev"><Icon name="chevron-right" size={14} weight={2.6} /></span>
      </button>
    {/each}
  </div>

  {#if shot}
    <h3 class="cap">截图</h3>
    <div class="group">
      <button class="shot" onclick={() => (viewing = true)}>
        <img src={shot.image} alt="支付截图" />
        <span class="shot-text">
          {#if diff.length}<b>改过 {diff.length} 处</b><small>最初显示 → 现在</small>{/if}
          <span class="view"><Icon name="photo" size={15} />查看截图</span>
        </span>
      </button>
      {#each diff as d (d.def.key)}
        <div class="row diff">
          <span class="fi" style:--fc="var(--c7)"><Icon name={d.def.icon} size={18} weight={2.2} /></span>
          <span class="label">{labelOf(d.def, entry)}</span>
          <span class="value"><s>{d.from}</s><span class="arrow">→</span>{d.to}</span>
        </div>
      {/each}
    </div>
  {/if}

  <div class="group danger">
    <button class="row del" onclick={() => (confirm = true)}><Icon name="trash" size={18} />删除账目</button>
  </div>
  <div class="tail"></div>
</div>

<ActionSheet open={confirm} title="删除后可以在几秒内撤销" actions={[{ label: "删除账目", danger: true, run: () => remove(entry.id) }]} oncancel={() => (confirm = false)} />
{#if viewing && shot}<ImageViewer src={shot.image} onclose={() => (viewing = false)} />{/if}

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
    margin-top: calc(-1 * env(safe-area-inset-top));
    padding: env(safe-area-inset-top) 8px 0;
    height: 44px;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    transition: background-color 0.2s, box-shadow 0.2s;
  }
  .bar.scrolled {
    background: var(--bar-bg);
    backdrop-filter: saturate(180%) blur(24px);
    -webkit-backdrop-filter: saturate(180%) blur(24px);
    box-shadow: 0 0.5px 0 var(--sep);
  }
  .back {
    justify-self: start;
    display: flex;
    align-items: center;
    gap: 1px;
    border: 0;
    background: none;
    color: var(--tint);
    font: inherit;
    font-size: 17px;
    padding: 6px 4px;
  }
  .back:active {
    opacity: 0.4;
  }
  .t {
    font-weight: 600;
    opacity: 0;
    transition: opacity 0.2s;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bar.scrolled .t {
    opacity: 1;
  }

  .hero {
    position: relative;
    display: grid;
    justify-items: center;
    padding: 18px 20px 26px;
    text-align: center;
  }
  .aura {
    position: absolute;
    left: 50%;
    top: -120px;
    width: 420px;
    height: 360px;
    translate: -50% 0;
    background: radial-gradient(closest-side, color-mix(in oklab, var(--c) 45%, transparent), transparent);
    filter: blur(10px);
    pointer-events: none;
  }
  .icon {
    position: relative;
    width: 76px;
    height: 76px;
    border-radius: 22px;
    display: grid;
    place-items: center;
    font-size: 40px;
    color: white;
    background: linear-gradient(145deg, color-mix(in oklab, var(--c) 50%, white), var(--c));
    box-shadow: 0 12px 26px -10px var(--c), inset 0 1px 0 rgba(255, 255, 255, 0.5);
  }
  h1 {
    position: relative;
    margin: 14px 0 0;
    font-size: 20px;
    font-weight: 600;
  }
  .amount {
    position: relative;
    margin-top: 2px;
    border: 0;
    background: none;
    color: var(--label);
    font: inherit;
    font-size: 46px;
    font-weight: 700;
    letter-spacing: -0.02em;
    border-radius: 12px;
    padding: 0 10px;
    transition: transform 0.25s;
  }
  .amount:active {
    transform: scale(0.96);
  }
  .amount.income {
    color: var(--green);
  }
  .hero p {
    position: relative;
    margin: 4px 0 0;
    font-size: 14px;
    color: var(--label2);
  }

  .cap {
    margin: 26px 32px 7px;
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
  .group.danger {
    margin-top: 26px;
  }
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 48px;
    padding: 8px 14px 8px 14px;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    font-size: 16px;
    text-align: left;
    transition: background-color 0.35s;
  }
  button.row:active,
  .shot:active {
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
  .label {
    flex: none;
  }
  .value {
    flex: 1;
    min-width: 0;
    text-align: right;
    color: var(--label2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .value.empty {
    color: var(--label3);
  }
  .chev {
    display: flex;
    color: var(--label3);
  }
  .diff .value {
    color: var(--label);
  }
  .diff s {
    color: var(--label3);
  }
  .arrow {
    margin: 0 6px;
    color: var(--tint);
  }
  .shot {
    position: relative;
    display: flex;
    gap: 14px;
    width: 100%;
    padding: 12px 14px;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
  }
  .shot img {
    width: 62px;
    height: 134px;
    object-fit: cover;
    border-radius: 9px;
    box-shadow: 0 0 0 0.5px var(--sep), 0 6px 16px -6px rgba(0, 0, 0, 0.3);
  }
  .shot-text {
    display: grid;
    align-content: center;
    gap: 4px;
  }
  .shot-text b {
    font-size: 16px;
  }
  .shot-text small {
    font-size: 13px;
    color: var(--label2);
  }
  .view {
    display: flex;
    align-items: center;
    gap: 5px;
    margin-top: 6px;
    font-size: 14px;
    color: var(--tint);
  }
  .del {
    justify-content: center;
    gap: 6px;
    color: var(--red);
  }
  .tail {
    height: calc(env(safe-area-inset-bottom) + 40px);
  }
</style>
