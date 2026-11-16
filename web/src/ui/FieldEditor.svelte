<script lang="ts">
  // 面板里改一个字段：选完即保存；输入类按「完成」保存。
  import { onMount, tick } from "svelte";
  import Icon from "./Icon.svelte";
  import { app, closeSheet, update } from "../lib/store.svelte";
  import { field } from "../lib/fields";
  import { localInput, parseCents, yuanInput } from "../lib/format";
  import { color, emoji } from "../lib/look";
  import type { Entry, FieldKey } from "../lib/types";

  let { entry, key }: { entry: Entry; key: FieldKey } = $props();

  const def = $derived(field(key));
  let text = $state("");
  let error = $state("");
  let input = $state<HTMLInputElement>();

  onMount(async () => {
    if (key === "amount_cents") text = yuanInput(entry.amount_cents);
    if (key === "merchant" || key === "note") text = entry[key] ?? "";
    if (key === "occurred_at") text = localInput(entry.occurred_at);
    if (key === "occurred_at") return;
    await tick();
    input?.focus();
    input?.select();
  });

  function save(patch: Partial<Entry>) {
    update(entry.id, patch);
    if (app.sheet?.field === key) closeSheet();
  }

  function submit() {
    if (key === "amount_cents") {
      const cents = parseCents(text);
      if (cents === null) {
        error = "金额要大于 0，最多两位小数";
        input?.animate([{ translate: "0" }, { translate: "-8px" }, { translate: "8px" }, { translate: "0" }], { duration: 260 });
        return;
      }
      save({ amount_cents: cents });
    } else if (key === "merchant" || key === "note") {
      save({ [key]: text.trim() || null });
    } else if (key === "occurred_at") {
      if (text) save({ occurred_at: new Date(text).toISOString() });
    } else closeSheet();
  }

  // 时间的快捷日期：只换日期，保留时分，选完即保存。
  const QUICK = [["今天", 0], ["昨天", 1], ["前天", 2]] as const;
  const daysAgo = (ago: number) => {
    const d = new Date();
    d.setDate(d.getDate() - ago);
    return d;
  };
  const sameDay = (a: Date, b: Date) => a.toDateString() === b.toDateString();
  function pickDay(ago: number) {
    const t = new Date(text || entry.occurred_at);
    const d = daysAgo(ago);
    d.setHours(t.getHours(), t.getMinutes(), 0, 0);
    save({ occurred_at: d.toISOString() });
  }

  function toggleTag(tag: string) {
    const tags = entry.tags.includes(tag) ? entry.tags.filter((t) => t !== tag) : [...entry.tags, tag];
    update(entry.id, { tags: app.catalog.tags.filter((t) => tags.includes(t)) });
  }

  const typed = $derived(["amount_cents", "merchant", "note", "occurred_at", "tags"].includes(key));
</script>

<header class="head">
  <button class="round" onclick={closeSheet} aria-label="关闭"><Icon name="xmark" size={14} weight={2.6} /></button>
  <h2><span class="emoji">{def.emoji}</span>{def.label}</h2>
  {#if typed}
    <button class="link done" onclick={submit}>完成</button>
  {:else}
    <span class="spacer"></span>
  {/if}
</header>

<div class="body">
  {#if key === "amount_cents"}
    <label class="amount" class:bad={error}>
      <span class="cur">¥</span>
      <input bind:this={input} bind:value={text} inputmode="decimal" enterkeyhint="done" onkeydown={(e) => e.key === "Enter" && submit()} oninput={() => (error = "")} />
    </label>
    <p class="hint" class:bad={error}>{error || "支付页上的实付金额"}</p>
  {:else if key === "merchant" || key === "note"}
    <div class="field">
      <input bind:this={input} bind:value={text} placeholder={key === "note" ? "买了什么" : "商家名称"} enterkeyhint="done" onkeydown={(e) => e.key === "Enter" && submit()} />
      {#if text}<button class="clear" onclick={() => { text = ""; input?.focus(); }} aria-label="清空"><Icon name="xmark" size={10} weight={3} /></button>{/if}
    </div>
  {:else if key === "occurred_at"}
    <div class="quick">
      {#each QUICK as [label, ago] (ago)}
        <button class:on={text !== "" && sameDay(new Date(text), daysAgo(ago))} onclick={() => pickDay(ago)}>{label}</button>
      {/each}
    </div>
    <div class="field"><input bind:this={input} type="datetime-local" bind:value={text} /></div>
  {:else if key === "kind"}
    <ul class="opts">
      {#each [["expense", "支出", "↘️"], ["income", "收入", "↗️"]] as [k, label, emoji] (k)}
        <li><button onclick={() => save({ kind: k as Entry["kind"] })}><span class="em">{emoji}</span>{label}{#if entry.kind === k}<span class="tick"><Icon name="check" weight={2.6} /></span>{/if}</button></li>
      {/each}
    </ul>
  {:else if key === "account"}
    <ul class="opts">
      {#each app.catalog.accounts as a (a.name)}
        <li>
          <button onclick={() => save({ account: a.name })}>
            <span class="two"><span>{a.name}</span>{#if a.card}<small>{a.card.type === "debit" ? "储蓄卡" : "信用卡"} 尾号 {a.card.number.slice(-4)}</small>{/if}</span>
            {#if entry.account === a.name}<span class="tick"><Icon name="check" weight={2.6} /></span>{/if}
          </button>
        </li>
      {/each}
    </ul>
  {:else if key === "tags"}
    <ul class="opts">
      {#each app.catalog.tags as t (t)}
        <li><button onclick={() => toggleTag(t)}><span class="em">🏷️</span>{t}{#if entry.tags.includes(t)}<span class="tick"><Icon name="check" weight={2.6} /></span>{/if}</button></li>
      {/each}
    </ul>
  {:else if key === "category"}
    <div class="cats">
      {#each app.catalog[entry.kind] as c, i (c.name)}
        <div class="cat" style:--cc={color(i)}>
          <span class="cat-name"><span class="ce">{emoji({ category: c.name, subcategory: null })}</span>{c.name}</span>
          <span class="chips">
            {#each c.subs.length ? c.subs : [null] as sub (sub)}
              <button class:on={entry.category === c.name && entry.subcategory === sub} onclick={() => save({ category: c.name, subcategory: sub })}>
                {sub ?? c.name}
              </button>
            {/each}
          </span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .head {
    display: grid;
    grid-template-columns: 80px 1fr 80px;
    align-items: center;
    padding: 0 16px 10px;
  }
  h2 {
    margin: 0;
    text-align: center;
    font-size: 17px;
    font-weight: 600;
  }
  .emoji {
    margin-right: 6px;
  }
  .link {
    border: 0;
    background: none;
    padding: 6px 0;
    color: var(--tint);
    font: inherit;
    font-size: 17px;
  }
  .done {
    justify-self: end;
    font-weight: 600;
  }
  .round {
    width: 30px;
    height: 30px;
    border-radius: 50%;
    border: 0;
    display: grid;
    place-items: center;
    background: var(--fill);
    color: var(--label2);
  }
  .body {
    overflow-y: auto;
    padding: 4px 16px 8px;
    -webkit-overflow-scrolling: touch;
  }
  .amount {
    display: flex;
    align-items: baseline;
    justify-content: center;
    gap: 4px;
    padding: 18px 0 6px;
    font-variant-numeric: tabular-nums;
  }
  .cur {
    font-size: 30px;
    font-weight: 600;
    color: var(--label2);
  }
  .amount input {
    width: 7ch;
    border: 0;
    background: none;
    outline: none;
    font: inherit;
    font-size: 52px;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--label);
    text-align: left;
    caret-color: var(--tint);
  }
  .hint {
    margin: 0 0 12px;
    text-align: center;
    font-size: 13px;
    color: var(--label3);
  }
  .bad {
    color: var(--red);
  }
  /* 分类：一级做行标题，二级平铺成胶囊，点一下就选中。 */
  .cats {
    display: grid;
    margin: 4px 0 8px;
    border-radius: var(--radius-group);
    background: var(--card);
    overflow: hidden;
  }
  .cat {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
  }
  .cat + .cat {
    border-top: 0.5px solid var(--sep);
  }
  .cat-name {
    flex: none;
    width: 66px;
    display: flex;
    align-items: center;
    gap: 4px;
    height: 32px;
    font-size: 14px;
    font-weight: 600;
    color: var(--label2);
  }
  .ce {
    font-size: 16px;
  }
  .chips {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chips button {
    height: 32px;
    padding: 0 12px;
    border: 0;
    border-radius: 16px;
    background: var(--fill);
    color: var(--label);
    font: inherit;
    font-size: 15px;
    transition: transform 0.2s, background-color 0.2s;
  }
  .chips button.on {
    background: var(--cc);
    color: #fff;
    font-weight: 600;
  }
  .chips button:active {
    transform: scale(0.92);
  }
  .quick {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    margin: 8px 0 0;
  }
  .quick button {
    height: 44px;
    border: 0;
    border-radius: var(--radius-group);
    background: var(--card);
    color: var(--label);
    font: inherit;
    font-size: 16px;
    font-weight: 600;
    transition: transform 0.2s;
  }
  .quick button.on {
    background: var(--tint);
    color: #fff;
  }
  .quick button:active {
    transform: scale(0.95);
  }
  .field {
    position: relative;
    margin: 8px 0 14px;
    border-radius: var(--radius-group);
    background: var(--card);
  }
  .field input {
    width: 100%;
    box-sizing: border-box;
    height: 50px;
    padding: 0 40px 0 16px;
    border: 0;
    background: none;
    outline: none;
    font: inherit;
    font-size: 17px;
    color: var(--label);
  }
  .clear {
    position: absolute;
    right: 12px;
    top: 15px;
    width: 20px;
    height: 20px;
    border: 0;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--label3);
    color: var(--sheet-bg);
  }
  .opts {
    list-style: none;
    margin: 4px 0 8px;
    padding: 0;
    border-radius: var(--radius-group);
    overflow: hidden;
    background: var(--card);
  }
  .opts li + li button {
    box-shadow: inset 16px 0.5px 0 -16px var(--sep);
  }
  .opts li + li {
    border-top: 0.5px solid var(--sep);
    margin-left: 16px;
  }
  .opts li + li button {
    margin-left: -16px;
    width: calc(100% + 16px);
    box-shadow: none;
  }
  .opts button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 50px;
    padding: 10px 16px;
    border: 0;
    background: none;
    font: inherit;
    font-size: 17px;
    color: var(--label);
    text-align: left;
    transition: background-color 0.3s;
  }
  .opts button:active {
    background: var(--press);
    transition: none;
  }
  .em {
    font-size: 18px;
  }
  .two {
    display: grid;
    gap: 2px;
    flex: 1;
  }
  .two small {
    font-size: 13px;
    color: var(--label2);
  }
  .tick {
    margin-left: auto;
    color: var(--tint);
    display: flex;
  }
</style>
