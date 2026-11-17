<script lang="ts">
  import { addEntry, app, accountOf, subsOf, toast, tops } from "../lib/store.svelte";
  import { newEntryId, type EntryInput } from "../lib/api";
  import { dayLabel, localInput, time } from "../lib/format";
  import { amountInput, amountResult } from "../lib/amount";
  import type { FieldKey, FlowKind } from "../lib/types";
  import AccountLogo from "./AccountLogo.svelte";
  import { holdKeyboard, keyboardInset } from "./nav.svelte";
  import ChoicePopover from "./ChoicePopover.svelte";
  import AccountChoices from "./AccountChoices.svelte";
  import TagChoices from "./TagChoices.svelte";
  import CategoryChoices from "./CategoryChoices.svelte";
  import Segmented from "./Segmented.svelte";
  import Icon from "./Icon.svelte";

  const firstCategory = (kind: FlowKind) => {
    const first = tops(kind)[0];
    return subsOf(first)[0] ?? first;
  };
  const now = new Date().toISOString();
  /** 这一笔的 ID：保存失败重试沿用，记上之后「再记一笔」换新的。 */
  let entryId = newEntryId();
  let draft: EntryInput = $state({
    kind: "expense",
    amount_cents: 0,
    category_id: firstCategory("expense").id,
    account_id: app.catalog.accounts[0]?.id ?? 0,
    to_account_id: null,
    merchant: null,
    occurred_at: now,
    note: null,
    tag_ids: [],
  });
  let amount = $state("");
  let picking = $state<FieldKey | null>(null);
  let useNow = $state(true);
  let dateText = $state(localInput(now));
  let writing = $state(false);
  let error = $state("");
  const transfer = $derived(draft.kind === "transfer");
  const account = $derived(accountOf(draft.account_id));
  const destination = $derived(accountOf(draft.to_account_id));
  const result = $derived(amountResult(amount));
  const calculated = $derived(/[+-]/.test(amount));
  const preview = $derived(result ?? amountResult(amount.replace(/[+-]$/, "")));
  const amountText = $derived(calculated && preview !== null ? (preview / 100).toFixed(2) : amount || "0.00");
  const ready = $derived(result !== null && result > 0 && (!transfer || (!!destination && draft.account_id !== draft.to_account_id)));
  const date = $derived(useNow ? "现在" : `${dayLabel(draft.occurred_at).split(" · ")[0]} ${time(draft.occurred_at)}`);
  const tagText = $derived(
    draft.tag_ids.length
      ? app.catalog.tags
          .filter((t) => draft.tag_ids.includes(t.id))
          .map((t) => t.name)
          .join("、")
      : "标签",
  );

  function openNote() {
    holdKeyboard();
    picking = "note";
  }
  function focusText(node: HTMLTextAreaElement) {
    node.focus();
  }
  function changeKind(kind: "expense" | "income" | "transfer") {
    if (kind === draft.kind) return;
    draft.kind = kind;
    if (kind === "transfer") {
      draft.category_id = null;
      draft.to_account_id = app.catalog.accounts.find((a) => a.id !== draft.account_id)?.id ?? null;
    } else {
      draft.category_id = firstCategory(kind).id;
      draft.to_account_id = null;
    }
    error = "";
  }
  function digit(key: string) {
    error = "";
    amount = amountInput(amount, key);
  }
  function amountKey(event: KeyboardEvent) {
    if (/^[0-9.+-]$/.test(event.key)) {
      event.preventDefault();
      digit(event.key);
    }
    if (event.key === "Backspace") {
      event.preventDefault();
      digit("back");
    }
  }
  function swap() {
    if (draft.to_account_id === null) return;
    [draft.account_id, draft.to_account_id] = [draft.to_account_id, draft.account_id];
  }
  function quickDate(ago: number) {
    const d = new Date();
    d.setDate(d.getDate() - ago);
    draft.occurred_at = d.toISOString();
    useNow = ago === 0;
    dateText = localInput(draft.occurred_at);
    picking = null;
  }
  function setDate() {
    if (!dateText) return;
    draft.occurred_at = new Date(dateText).toISOString();
    useNow = false;
    picking = null;
  }
  /** 正在提交：按钮先停用，免得连点。 */
  let saving = $state(false);
  async function save(again = false) {
    const cents = result;
    if (cents === null || cents <= 0) {
      error = "先填一个大于 0 的金额";
      return;
    }
    if (transfer && (!destination || draft.account_id === draft.to_account_id)) {
      error = "请选择不同的转出、转入账户";
      return;
    }
    if (saving) return;
    saving = true;
    const saved = await addEntry(entryId, {
      ...draft,
      amount_cents: cents,
      occurred_at: useNow ? new Date().toISOString() : draft.occurred_at,
      merchant: transfer ? null : draft.merchant?.trim() || null,
      note: draft.note?.trim() || null,
      tag_ids: [...draft.tag_ids],
    });
    saving = false;
    // 没记上：面板、填好的内容和 ID 都留着，提示由 store 弹出。
    if (!saved) return;
    entryId = newEntryId();
    if (!again) app.creating = false;
    if (again) {
      amount = "";
      draft.note = null;
      draft.merchant = null;
      error = "";
    }
    if (saved === "existed") toast("这笔之前已保存，没有重复记");
    else toast(again ? "已记好，继续下一笔" : "已记好");
  }
</script>

<div class="composer" use:keyboardInset>
  <header>
    <button class="close" aria-label="取消记账" onclick={() => (app.creating = false)}><Icon name="xmark" size={20} /></button>
    <Segmented
      label="账目类型"
      options={[
        ["expense", "支出"],
        ["income", "收入"],
        ["transfer", "转账"],
      ]}
      value={draft.kind as "expense" | "income" | "transfer"}
      onchange={changeKind}
    />
    <span class="spacer"></span>
  </header>

  <div class="choices">
    {#if transfer}
      <div class="transfer">
        <button class="transfer-account" onclick={() => (picking = "account_id")}>
          <small>从这个账户转出</small>
          {#if account}<AccountLogo {account} size={48} /><b>{account.name}</b>{/if}
          <span>更换账户 <Icon name="chevron-down" size={12} /></span>
        </button>
        <button class="swap" aria-label="交换转出和转入账户" onclick={swap}><Icon name="arrows" size={22} /></button>
        <button class="transfer-account" onclick={() => (picking = "to_account_id")}>
          <small>转入这个账户</small>
          {#if destination}<AccountLogo account={destination} size={48} /><b>{destination.name}</b>{:else}<b>选择账户</b>{/if}
          <span>更换账户 <Icon name="chevron-down" size={12} /></span>
        </button>
      </div>
    {:else}
      <CategoryChoices
        kind={draft.kind as FlowKind}
        selected={draft.category_id}
        onselect={(id) => {
          draft.category_id = id;
          error = "";
        }}
      />
    {/if}
  </div>

  <footer>
    <ChoicePopover
      above
      open={picking !== null}
      title={picking === "account_id" || picking === "to_account_id"
        ? "选择账户"
        : picking === "occurred_at"
          ? "交易时间"
          : picking === "tag_ids"
            ? "标签"
            : picking === "note"
              ? "备注"
              : "商家"}
      onclose={() => (picking = null)}
    >
      <div class="popover-body">
        {#if picking === "account_id" || picking === "to_account_id"}
          <AccountChoices
            selected={draft[picking]}
            exclude={picking === "to_account_id" ? draft.account_id : draft.to_account_id}
            onselect={(id) => {
              if (picking === "account_id") draft.account_id = id;
              else draft.to_account_id = id;
              picking = null;
            }}
          />
        {:else if picking === "occurred_at"}
          <div class="quick-days">
            {#each ["现在", "昨天", "前天"] as label, ago (ago)}<button onclick={() => quickDate(ago)}>{label}</button>{/each}
          </div>
          <label class="specific"
            >具体时间<input type="datetime-local" onfocus={() => (writing = true)} onblur={() => (writing = false)} bind:value={dateText} /></label
          ><button class="confirm" disabled={!dateText} onclick={setDate}>确定时间</button>
        {:else if picking === "tag_ids"}
          <TagChoices
            selected={draft.tag_ids}
            onselect={(id) => (draft.tag_ids = draft.tag_ids.includes(id) ? draft.tag_ids.filter((t) => t !== id) : [...draft.tag_ids, id])}
          />
        {:else if picking === "note"}
          <form
            onsubmit={(e) => {
              e.preventDefault();
              picking = null;
            }}
          >
            <textarea use:focusText aria-label="完整备注" bind:value={draft.note} placeholder="写点备注…" rows="3"></textarea><button
              class="confirm"
              type="submit">完成</button
            >
          </form>
        {:else}
          <form
            onsubmit={(e) => {
              e.preventDefault();
              picking = null;
            }}
          >
            <input class="merchant-input" aria-label="商家名称" bind:value={draft.merchant} placeholder="商家名称，可不填" enterkeyhint="done" /><button
              class="confirm"
              type="submit">完成</button
            >
          </form>
        {/if}
      </div>
    </ChoicePopover>
    <div class="shortcuts">
      {#if !transfer}<button onclick={() => (picking = "account_id")}
          >{#if account}<AccountLogo {account} size={20} />{account.name}{:else}账户{/if}<Icon name="chevron-down" size={11} /></button
        >{/if}
      <button onclick={() => (picking = "occurred_at")}><Icon name="calendar" size={15} />{date}</button>
      <button class:chosen={draft.tag_ids.length > 0} onclick={() => (picking = "tag_ids")}><Icon name="tag" size={15} /><span>{tagText}</span></button>
      {#if !transfer}<button class:chosen={!!draft.merchant} onclick={() => (picking = "merchant")}
          ><Icon name="store" size={15} /><span>{draft.merchant || "商家"}</span></button
        >{/if}
    </div>
    <div class="entry-line">
      <button class="note" class:empty={!draft.note} onclick={openNote} aria-label={draft.note ? `查看和编辑备注：${draft.note}` : "添加备注"}>
        <Icon name="note" size={15} /><span>{draft.note || "写点备注…"}</span>
      </button>
      <div class="amount-value">
        {#if calculated}<span class="expression num">{amount.replaceAll("-", " − ").replaceAll("+", " + ")}</span>{/if}
        <button
          class="amount num"
          class:expense={draft.kind === "expense"}
          class:income={draft.kind === "income"}
          aria-label={`金额 ${amountText}，可用数字键输入`}
          onkeydown={amountKey}><small>¥</small>{amountText}</button
        >
      </div>
    </div>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if !writing && picking !== "merchant" && picking !== "note"}
      <div class="keypad" aria-label="金额键盘">
        {#each ["1", "2", "3", "4", "5", "6", "7", "8", "9", ".", "0"] as key (key)}
          <button
            class="key num"
            style:grid-column={key === "." ? 1 : key === "0" ? 2 : ((Number(key) - 1) % 3) + 1}
            style:grid-row={key === "." || key === "0" ? 4 : Math.ceil(Number(key) / 3)}
            onclick={() => digit(key)}>{key}</button
          >
        {/each}
        <button class="key again" disabled={!ready || saving} onclick={() => save(true)}>再记一笔</button>
        <button class="key erase" aria-label="删除一位" onclick={() => digit("back")}><Icon name="chevron-left" size={22} /></button>
        <button class="key operator plus" aria-label="加" onclick={() => digit("+")}><Icon name="plus" size={24} weight={2.2} /></button>
        <button class="key operator minus" aria-label="减" onclick={() => digit("-")}><Icon name="minus" size={24} weight={2.2} /></button>
        <button class="key save" disabled={!ready || saving} onclick={() => save()}>保存</button>
      </div>
    {/if}
  </footer>
</div>

<style>
  .composer {
    position: absolute;
    inset: 0;
    bottom: var(--kb, 0px);
    z-index: 30;
    box-sizing: border-box;
    padding-top: env(safe-area-inset-top);
    background: var(--sheet-bg);
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .popover-body button {
    min-height: 44px;
    border-radius: 12px;
    background: var(--fill);
    padding: 8px 12px;
  }
  .quick-days {
    display: flex;
    gap: 8px;
  }
  .quick-days button {
    flex: 1;
  }
  .specific {
    display: grid;
    gap: 8px;
    margin: 16px 0 8px;
    font-size: 13px;
    color: var(--label2);
  }
  .specific input,
  .merchant-input {
    width: 100%;
    box-sizing: border-box;
    border: 0;
    min-height: 44px;
    padding: 10px;
    background: var(--fill);
    border-radius: 12px;
    font-size: 16px;
  }
  .popover-body .confirm {
    margin-top: 8px;
    width: 100%;
    background: var(--tint);
    color: white;
  }
  .again {
    grid-column: 3;
    grid-row: 4;
    font-size: 12px !important;
    color: var(--tint);
  }
  button:disabled {
    opacity: 0.4;
  }

  button,
  input,
  textarea {
    font: inherit;
    color: inherit;
  }
  textarea {
    width: 100%;
    box-sizing: border-box;
    min-height: 88px;
    max-height: 160px;
    resize: vertical;
    border: 0;
    border-radius: 12px;
    padding: 10px;
    font-size: 16px;
    background: var(--fill);
  }
  button {
    border: 0;
    cursor: pointer;
    -webkit-tap-highlight-color: transparent;
  }
  button:active {
    opacity: 0.65;
  }
  header {
    display: grid;
    grid-template-columns: 44px 1fr 44px;
    align-items: center;
    gap: 12px;
    padding: 0 12px 10px;
    border-bottom: 0.5px solid var(--sep);
  }
  .close {
    display: grid;
    place-items: center;
    height: 44px;
    width: 44px;
    border-radius: 50%;
    background: var(--fill);
    color: var(--label2);
  }
  .choices {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    overscroll-behavior: contain;
    padding: 12px 16px 14px;
  }
  .transfer {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 36px minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    margin: 16px 0;
  }
  .transfer-account {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    padding: 20px 8px;
    border-radius: 20px;
    background: var(--card);
    min-width: 0;
  }
  .transfer-account b {
    max-width: 100%;
    overflow-wrap: anywhere;
    font-size: 17px;
  }
  .transfer-account small,
  .transfer-account span {
    color: var(--label2);
    font-size: 12px;
  }
  .transfer-account span {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .swap {
    display: grid;
    place-items: center;
    background: none;
    color: var(--tint);
    min-height: 44px;
  }
  footer {
    position: relative;
    flex: none;
    padding: 8px 12px max(env(safe-area-inset-bottom), 12px);
    background: var(--bg);
    border-top: 0.5px solid var(--sep);
  }
  .shortcuts {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    scrollbar-width: none;
    padding: 0 0 8px;
  }
  .shortcuts button {
    flex: none;
    display: flex;
    align-items: center;
    gap: 5px;
    min-height: 32px;
    max-width: 180px;
    padding: 5px 10px;
    border-radius: 16px;
    background: var(--card);
    font-size: 13px;
    color: var(--label2);
    white-space: nowrap;
  }
  .shortcuts button span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .shortcuts button.chosen {
    color: var(--tint);
  }
  .entry-line {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 52px;
    padding: 6px 12px;
    border-radius: calc(var(--radius-group) + 2px);
    background: var(--card);
  }
  .amount-value {
    min-width: 0;
    max-width: 100%;
    flex: 0 1 auto;
    margin-left: auto;
  }
  .expression {
    display: block;
    color: var(--label2);
    font-size: 11px;
    white-space: nowrap;
    overflow-x: auto;
    max-width: 220px;
    text-align: right;
  }
  .note {
    min-width: 0;
    flex: 1 1 0;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 0;
    background: none;
    text-align: left;
    color: var(--label);
    overflow: hidden;
  }
  .note.empty {
    color: var(--label3);
  }
  .note span {
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 15px;
  }
  .amount {
    display: flex;
    align-items: baseline;
    gap: 3px;
    flex: none;
    max-width: 100%;
    overflow-x: auto;
    padding: 0;
    background: none;
    color: var(--tint);
    font-size: clamp(24px, 7vw, 32px);
    font-weight: 600;
    letter-spacing: -0.04em;
  }
  .amount.expense {
    color: var(--red);
  }
  .amount.income {
    color: var(--green);
  }
  .amount small {
    font-size: 14px;
    font-weight: 400;
    letter-spacing: 0;
  }
  .error {
    margin: 4px 0;
    color: var(--red);
    font-size: 12px;
  }
  .keypad {
    margin-top: 8px;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr)) minmax(60px, 0.85fr);
    grid-template-rows: repeat(4, clamp(42px, 6.3dvh, 52px));
    gap: 7px;
  }
  .key {
    display: grid;
    place-items: center;
    background: var(--card);
    border-radius: 11px;
    font-size: 25px;
    font-weight: 450;
    touch-action: manipulation;
  }
  .erase {
    grid-column: 4;
    grid-row: 1;
    display: flex;
    gap: 1px;
    justify-content: center;
    color: var(--label2);
  }
  .operator {
    grid-column: 4;
    color: var(--tint);
    font-size: 26px;
  }
  .plus {
    grid-row: 2;
  }
  .minus {
    grid-row: 3;
  }
  .save {
    grid-column: 4;
    grid-row: 4;
  }
  .save {
    background: var(--tint);
    color: white;
    font-size: 15px;
    font-weight: 650;
  }
</style>
