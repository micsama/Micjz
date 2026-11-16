<script lang="ts">
  // 面板里改一个字段：选完即保存；输入类按「完成」保存。
  import { onMount, tick } from "svelte";
  import Icon from "./Icon.svelte";
  import SheetHead from "./SheetHead.svelte";
  import AccountChoices from "./AccountChoices.svelte";
  import TagChoices from "./TagChoices.svelte";
  import CategoryChoices from "./CategoryChoices.svelte";
  import { app, closeSheet, update } from "../lib/store.svelte";
  import { field, labelOf } from "../lib/fields";
  import { localInput, parseCents, yuanInput } from "../lib/format";
  import type { Entry, FieldKey, Fields, FlowKind } from "../lib/types";

  let { entry, key }: { entry: Entry; key: FieldKey } = $props();

  const def = $derived(field(key));
  const signed = $derived(entry.kind === "adjust");
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

  function save(patch: Partial<Fields>) {
    update(entry.id, patch);
    if (app.sheet?.field === key) closeSheet();
  }

  function submit() {
    if (key === "amount_cents") {
      const cents = parseCents(text, signed);
      if (cents === null) {
        error = signed ? "差额不能为 0，最多两位小数" : "金额要大于 0，最多两位小数";
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
  const QUICK = [
    ["今天", 0],
    ["昨天", 1],
    ["前天", 2],
  ] as const;
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

  function toggleTag(id: number) {
    const ids = entry.tag_ids.includes(id) ? entry.tag_ids.filter((t) => t !== id) : [...entry.tag_ids, id];
    const patch = { tag_ids: app.catalog.tags.filter((t) => ids.includes(t.id)).map((t) => t.id) };
    update(entry.id, patch);
  }

  /** 转账的两边不能是同一个账户。 */
  const other = $derived(key === "account_id" ? entry.to_account_id : key === "to_account_id" ? entry.account_id : null);

  const typed = $derived(["amount_cents", "merchant", "note", "occurred_at", "tag_ids", "category_id"].includes(key));
</script>

<SheetHead icon={def.icon} title={labelOf(def, entry)} onclose={closeSheet} action={typed ? { label: "完成", run: submit } : undefined} />

<div class="body">
  {#if key === "amount_cents"}
    <label class="amount" class:bad={error}>
      <span class="cur">¥</span>
      <input
        bind:this={input}
        bind:value={text}
        inputmode={signed ? "text" : "decimal"}
        enterkeyhint="done"
        onkeydown={(e) => e.key === "Enter" && submit()}
        oninput={() => (error = "")}
      />
    </label>
    <p class="hint" class:bad={error}>{error || (signed ? "比账面多填正数，少了填负数" : "支付页上的实付金额")}</p>
  {:else if key === "merchant" || key === "note"}
    <div class="field">
      <input
        bind:this={input}
        bind:value={text}
        placeholder={key === "note" ? "买了什么" : "商家名称"}
        enterkeyhint="done"
        onkeydown={(e) => e.key === "Enter" && submit()}
      />
      {#if text}<button
          class="clear"
          onclick={() => {
            text = "";
            input?.focus();
          }}
          aria-label="清空"><Icon name="xmark" size={10} weight={3} /></button
        >{/if}
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
      {#each [["expense", "支出", "arrow-out"], ["income", "收入", "arrow-in"]] as const as [k, label, icon] (k)}
        <li>
          <button onclick={() => save({ kind: k })}
            ><span class="em"><Icon name={icon} size={18} weight={2.2} /></span>{label}{#if entry.kind === k}<span class="tick"
                ><Icon name="check" weight={2.6} /></span
              >{/if}</button
          >
        </li>
      {/each}
    </ul>
  {:else if key === "account_id" || key === "to_account_id"}
    <AccountChoices selected={entry[key]} exclude={other} onselect={(id) => save({ [key]: id })} />
  {:else if key === "tag_ids"}
    <TagChoices selected={entry.tag_ids} onselect={toggleTag} />
  {:else if key === "category_id"}
    <CategoryChoices
      kind={entry.kind as FlowKind}
      selected={entry.category_id}
      onselect={(id, final) => {
        if (final) save({ category_id: id });
        else update(entry.id, { category_id: id });
      }}
    />
  {/if}
</div>

<style>
  .body {
    overflow-y: auto;
    overflow-x: hidden;
    touch-action: pan-y;
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
    display: flex;
    color: var(--tint);
  }
  .tick {
    margin-left: auto;
    color: var(--tint);
    display: flex;
  }
</style>
