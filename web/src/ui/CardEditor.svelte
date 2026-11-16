<script lang="ts">
  // 改账户的卡类型和卡号；按「完成」保存。
  import { onMount, tick } from "svelte";
  import Icon from "./Icon.svelte";
  import { setCard, toast } from "../lib/store.svelte";
  import type { Account } from "../lib/types";

  let { account, onclose }: { account: Account; onclose: () => void } = $props();

  // 面板打开时按当时的账户初始化，之后由输入框接管。
  // svelte-ignore state_referenced_locally
  let type = $state<"debit" | "credit">(account.card?.type ?? "debit");
  // svelte-ignore state_referenced_locally
  let text = $state(account.card?.number.replace(/(\d{4})(?=\d)/g, "$1 ") ?? "");
  let error = $state("");
  let input = $state<HTMLInputElement>();

  onMount(async () => {
    await tick();
    input?.focus();
  });

  function submit() {
    const number = text.replace(/\s/g, "");
    if (!/^\d{12,19}$/.test(number)) {
      error = "卡号是 12～19 位数字";
      return;
    }
    setCard(account.name, { type, number });
    toast("卡号已保存");
    onclose();
  }
</script>

<header class="head">
  <button class="round" onclick={onclose} aria-label="关闭"><Icon name="xmark" size={14} weight={2.6} /></button>
  <h2>{account.name}</h2>
  <button class="link" onclick={submit}>完成</button>
</header>

<div class="body">
  <div class="seg">
    {#each [["debit", "储蓄卡"], ["credit", "信用卡"]] as [k, label] (k)}
      <button class:on={type === k} onclick={() => (type = k as "debit" | "credit")}>{label}</button>
    {/each}
  </div>
  <input class="num" bind:this={input} bind:value={text} inputmode="numeric" placeholder="完整卡号" enterkeyhint="done" onkeydown={(e) => e.key === "Enter" && submit()} oninput={() => (error = "")} />
  <p class="hint" class:bad={error}>{error || "完整卡号只存在自己的库里；识别时只把尾号给模型"}</p>
  {#if account.card}
    <button class="remove" onclick={() => { setCard(account.name, null); onclose(); }}>移除卡号</button>
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
  .link {
    justify-self: end;
    border: 0;
    background: none;
    color: var(--tint);
    font: inherit;
    font-size: 17px;
    font-weight: 600;
  }
  .body {
    display: grid;
    gap: 10px;
    padding: 4px 16px 12px;
  }
  .seg {
    display: grid;
    grid-template-columns: 1fr 1fr;
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
    letter-spacing: 0.04em;
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
  .remove {
    height: 48px;
    border: 0;
    border-radius: var(--radius-group);
    background: var(--card);
    color: var(--red);
    font: inherit;
    font-size: 16px;
  }
</style>
