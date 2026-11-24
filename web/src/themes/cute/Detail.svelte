<script lang="ts">
  import ActionSheet from "../../ui/ActionSheet.svelte";
  import ImageViewer from "../../ui/ImageViewer.svelte";
  import { app, back, edit, loadRecognition, remove } from "../../lib/store.svelte";
  import { imageUrl, mapUrl } from "../../lib/api";
  import { changes, fieldsOf, labelOf } from "../../lib/fields";
  import { dateTime } from "../../lib/format";
  import { view } from "../../lib/view";
  import type { Entry } from "../../lib/types";
  import Mark from "../../ui/Mark.svelte";

  let { entry }: { entry: Entry } = $props();

  let confirm = $state(false);
  let viewing = $state(false);

  const v = $derived(view(entry));
  const shot = $derived(entry.recognition_id ? app.recognitions[entry.recognition_id] : undefined);
  const diff = $derived(shot ? changes(entry, shot) : []);
  /** 截图没留存下来（后台写入失败）时不显示坏图。 */
  let lost = $state(false);
  $effect(() => {
    if (entry.recognition_id) void loadRecognition(entry.recognition_id);
  });
  const tiles = $derived(fieldsOf(entry).filter((f) => f.key !== "amount_cents"));
</script>

<div class="page" style:--c={v.color}>
  <header class="top">
    <button class="back" onclick={back} aria-label="返回">‹</button>
  </header>

  <div class="wrap">
    <span class="bubble"><Mark mark={v.mark} /></span>
    <section class="ticket">
      <h1>{v.title}</h1>
      <p class="when num">{dateTime(entry.occurred_at)}</p>
      <div class="dash"></div>
      <button class="amount num" class:income={v.tone === "in"} onclick={() => edit(entry.id, "amount_cents")}>{v.amount}</button>
      <small>点金额可以改哦</small>
    </section>
  </div>

  <section class="tiles">
    {#each tiles as f (f.key)}
      <button class="tile" class:wide={f.key === "note" || f.key === "category_id"} onclick={() => edit(entry.id, f.key)}>
        <span class="te">{f.emoji}</span>
        <span class="tl">{labelOf(f, entry)}</span>
        <span class="tv" class:empty={!f.show(entry)}>{f.show(entry) || "还没填～"}</span>
      </button>
    {/each}
  </section>

  {#if shot}
    <h3 class="cap">📸 截图</h3>
    {#if lost}
      <p class="lost">截图没留存下来～</p>
    {:else}
      <button class="polaroid" onclick={() => (viewing = true)}>
        <img src={imageUrl(shot.id)} alt="支付截图" onerror={() => (lost = true)} />
        <span>点开看原图 🔍</span>
      </button>
    {/if}
    {#if diff.length}
      <div class="sticky">
        <b>后来改了 {diff.length} 处 ✏️</b>
        {#each diff as d (d.def.key)}
          <p>{d.def.emoji} <s>{d.from}</s> → {d.to}</p>
        {/each}
      </div>
    {/if}
    {#if shot.location}
      <a class="tile where" href={mapUrl(shot.location, entry.merchant)} target="_blank" rel="noreferrer">
        <span class="te">📍</span>
        <span class="tl">记账位置</span>
        <span class="tv">{shot.location.city ?? "在地图里看看"}</span>
      </a>
    {/if}
  {/if}

  <button class="del" onclick={() => (confirm = true)}>不要这笔了 🗑️</button>
  <div class="tail"></div>
</div>

<ActionSheet
  open={confirm}
  title="删掉之后几秒内还能撤销"
  actions={[{ label: "删掉它", danger: true, run: () => remove(entry.id) }]}
  oncancel={() => (confirm = false)}
/>
{#if viewing && shot}<ImageViewer src={imageUrl(shot.id)} onclose={() => (viewing = false)} />{/if}

<style>
  .page {
    position: absolute;
    inset: 0;
    overflow-y: auto;
    overflow-x: hidden;
    touch-action: pan-y;
    -webkit-overflow-scrolling: touch;
    padding-top: calc(env(safe-area-inset-top) + 6px);
    background: radial-gradient(70% 30% at 50% 0%, color-mix(in srgb, var(--c) 35%, transparent), transparent), var(--bg);
  }
  .top {
    padding: 4px 16px;
  }
  .back {
    width: 40px;
    height: 40px;
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
    transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .back:active {
    transform: scale(0.85);
  }

  .wrap {
    position: relative;
    margin: 30px 20px 0;
  }
  .ticket {
    display: grid;
    justify-items: center;
    padding: 44px 20px 20px;
    border-radius: 26px;
    background: var(--card);
    text-align: center;
    box-shadow: 0 16px 34px -18px rgba(255, 111, 156, 0.55);
    /* 两侧打孔 */
    -webkit-mask:
      radial-gradient(circle 12px at 0 62%, transparent 11px, #000 12px) left / 51% 100% no-repeat,
      radial-gradient(circle 12px at 100% 62%, transparent 11px, #000 12px) right / 51% 100% no-repeat;
    mask:
      radial-gradient(circle 12px at 0 62%, transparent 11px, #000 12px) left / 51% 100% no-repeat,
      radial-gradient(circle 12px at 100% 62%, transparent 11px, #000 12px) right / 51% 100% no-repeat;
  }
  .bubble {
    position: absolute;
    top: -30px;
    left: calc(50% - 32px);
    z-index: 1;
    width: 64px;
    height: 64px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 34px;
    background: color-mix(in srgb, var(--c) 30%, var(--card));
    border: 4px solid var(--card);
    animation: wiggle 3s ease-in-out infinite;
  }
  @keyframes wiggle {
    50% {
      transform: rotate(-8deg) translateY(-3px);
    }
  }
  h1 {
    margin: 0;
    font-size: 20px;
    font-weight: 800;
  }
  .when {
    margin: 4px 0 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--label2);
  }
  .dash {
    align-self: stretch;
    margin: 16px 6px 12px;
    border-top: 2px dashed var(--sep);
  }
  .amount {
    border: 0;
    background: none;
    color: var(--label);
    font: inherit;
    font-size: 42px;
    font-weight: 800;
    transition: transform 0.35s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .amount:active {
    transform: scale(0.92);
  }
  .amount.income {
    color: var(--green);
  }
  .ticket small {
    font-size: 12px;
    font-weight: 600;
    color: var(--label3);
  }

  .tiles {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    margin: 16px 16px 0;
  }
  .tile {
    display: grid;
    grid-template-columns: auto 1fr;
    grid-template-rows: auto auto;
    column-gap: 8px;
    row-gap: 2px;
    align-items: center;
    padding: 12px 14px;
    border: 0;
    border-radius: 20px;
    background: var(--card);
    color: inherit;
    font: inherit;
    text-align: left;
    box-shadow: 0 6px 16px -12px rgba(255, 111, 156, 0.5);
    transition: transform 0.35s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .tile:active {
    transform: scale(0.94) rotate(-1deg);
  }
  .tile.wide {
    grid-column: span 2;
  }
  .te {
    grid-row: span 2;
    width: 36px;
    height: 36px;
    border-radius: 12px;
    display: grid;
    place-items: center;
    font-size: 18px;
    background: var(--fill);
  }
  .tl {
    font-size: 12px;
    font-weight: 700;
    color: var(--label2);
  }
  .tv {
    font-size: 15px;
    font-weight: 800;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tv.empty {
    color: var(--label3);
    font-weight: 600;
  }

  .cap {
    margin: 26px 22px 10px;
    font-size: 15px;
    font-weight: 800;
  }
  .polaroid {
    display: grid;
    gap: 8px;
    width: calc(100% - 40px);
    margin: 0 20px;
    padding: 10px 10px 12px;
    border: 0;
    border-radius: 8px;
    background: #fff;
    color: #8a7a80;
    font: inherit;
    font-size: 13px;
    font-weight: 700;
    rotate: -1.5deg;
    box-shadow: 0 14px 28px -14px rgba(0, 0, 0, 0.35);
    transition:
      rotate 0.4s cubic-bezier(0.34, 1.56, 0.64, 1),
      scale 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  .polaroid:active {
    rotate: 0deg;
    scale: 1.03;
  }
  /* 只露出上部：金额、商家一般在这里。 */
  .polaroid img {
    width: 100%;
    height: 200px;
    object-fit: cover;
    object-position: center 18%;
    border-radius: 3px;
  }
  .lost {
    margin: 0 22px;
    font-size: 14px;
    font-weight: 600;
    color: var(--label3);
  }
  .sticky {
    margin: 18px 24px 0;
    padding: 14px 16px 16px;
    background: #fff4b8;
    color: #5f5330;
    border-radius: 4px 4px 18px 4px;
    rotate: 1deg;
    box-shadow: 0 10px 20px -12px rgba(160, 130, 0, 0.5);
    font-size: 13px;
  }
  .sticky b {
    display: block;
    margin-bottom: 6px;
    font-size: 14px;
  }
  .sticky p {
    margin: 4px 0 0;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .sticky s {
    opacity: 0.55;
  }
  .where {
    width: calc(100% - 32px);
    margin: 16px 16px 0;
    text-decoration: none;
  }
  .del {
    display: block;
    margin: 30px auto 0;
    padding: 10px 20px;
    border: 0;
    border-radius: 999px;
    background: color-mix(in srgb, var(--red) 12%, transparent);
    color: var(--red);
    font: inherit;
    font-size: 15px;
    font-weight: 800;
  }
  .del:active {
    transform: scale(0.92);
  }
  .tail {
    height: calc(env(safe-area-inset-bottom) + 40px);
  }
</style>
