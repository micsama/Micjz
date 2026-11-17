<script lang="ts">
  // 平账：填账户此刻的实际余额，和账面的差额记成一笔平账。
  import { onMount, tick } from "svelte";
  import SheetHead from "./SheetHead.svelte";
  import { adjust } from "../lib/store.svelte";
  import { balanceOf } from "../lib/query";
  import { money, yuanCents, yuanInput } from "../lib/format";
  import type { Account } from "../lib/types";

  let { account, onclose }: { account: Account; onclose: () => void } = $props();

  // svelte-ignore state_referenced_locally
  const book = balanceOf(account.id);
  /** 欠款（负余额）单独一个开关：数字键盘没有负号。信用卡默认是欠款。 */
  // svelte-ignore state_referenced_locally
  let owe = $state(book < 0 || (book === 0 && account.card?.type === "credit"));
  let amount = $state(book ? yuanInput(Math.abs(book)) : "");
  let error = $state("");
  let busy = $state(false);
  let input = $state<HTMLInputElement>();

  const cents = $derived(amount.trim() === "" ? null : yuanCents(amount.trim()));
  const actual = $derived(cents === null ? null : owe ? -cents : cents);

  onMount(async () => {
    await tick();
    input?.select();
  });

  async function submit() {
    if (busy) return;
    if (actual === null) return void (error = "填实际余额，最多两位小数");
    busy = true;
    const err = await adjust(account.id, actual);
    busy = false;
    if (err) return void (error = err);
    onclose();
  }
</script>

<SheetHead icon="scale" title="平账" {onclose} action={{ label: "完成", run: submit }} />

<div class="body">
  <p class="book">
    <span>{account.name} 账面余额</span><b class="num" class:neg={book < 0}>{money(book, book < 0)}</b>
  </p>

  <h3>实际余额</h3>
  <div class="seg">
    <button class:on={!owe} onclick={() => (owe = false)}>有余额</button>
    <button class:on={owe} onclick={() => (owe = true)}>欠款</button>
  </div>
  <input
    class="num"
    bind:this={input}
    bind:value={amount}
    inputmode="decimal"
    placeholder="0.00"
    enterkeyhint="done"
    oninput={() => (error = "")}
    onkeydown={(e) => e.key === "Enter" && submit()}
  />
  {#if error}
    <p class="hint bad">{error}</p>
  {:else if actual !== null}
    <p class="hint">
      {actual === book ? "和账面一致，不会记账" : `将记一笔平账 ${money(actual - book, true)}，不计入收支统计`}
    </p>
  {/if}
</div>

<style>
  .body {
    display: grid;
    gap: 10px;
    padding: 4px 16px 12px;
  }
  .book {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin: 0;
    padding: 14px 16px;
    border-radius: var(--radius-group);
    background: var(--card);
    font-size: 15px;
  }
  .book b {
    font-size: 17px;
  }
  .neg {
    color: var(--red);
  }
  h3 {
    margin: 10px 4px -2px;
    font-size: 13px;
    font-weight: 500;
    color: var(--label2);
  }
  .seg {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    padding: 2px;
    border-radius: 9px;
    background: var(--fill);
  }
  .seg button {
    height: 32px;
    border: 0;
    border-radius: 7px;
    background: none;
    color: var(--label);
    font: inherit;
    font-size: 14px;
    font-weight: 500;
    transition: background-color 0.2s;
  }
  .seg button.on {
    background: var(--card);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.12);
    font-weight: 600;
  }
  input {
    height: 50px;
    padding: 0 16px;
    border: 0;
    border-radius: var(--radius-group);
    background: var(--card);
    color: var(--label);
    font: inherit;
    font-size: 19px;
    outline: none;
  }
  .hint {
    margin: 0 4px;
    font-size: 13px;
    color: var(--label2);
  }
  .bad {
    color: var(--red);
  }
</style>
