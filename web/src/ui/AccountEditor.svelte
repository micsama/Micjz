<script lang="ts">
  // 新建或编辑账户：名称、机构、银行卡；按「完成」保存。
  import { onMount, tick } from "svelte";
  import Icon from "./Icon.svelte";
  import SheetHead from "./SheetHead.svelte";
  import { holdKeyboard } from "./nav.svelte";
  import AccountLogo from "./AccountLogo.svelte";
  import { removeAccount, saveAccount, toast } from "../lib/store.svelte";
  import { guessBrand } from "../lib/look";
  import { LOGOS } from "../lib/logos";
  import type { Account } from "../lib/types";

  /** `account` 为空是新建。 */
  let { account, onclose }: { account: Account | null; onclose: () => void } = $props();

  // 面板打开时按当时的账户初始化，之后由输入框接管。
  // svelte-ignore state_referenced_locally
  const old = account;
  let name = $state(old?.name ?? "");
  let brand = $state(old?.brand ?? null);
  /** 用户点选过机构后，不再按名称猜。 */
  let picked = $state(old !== null);
  let cardType = $state<"none" | "debit" | "credit">(old?.card?.type ?? "none");
  let number = $state(old?.card?.number.replace(/(\d{4})(?=\d)/g, "$1 ") ?? "");
  let external = $state(old?.external ?? false);
  /** 校验错误，显示在出错的那一项下面。 */
  let error = $state<{ at: "name" | "card"; text: string } | null>(null);
  let nameInput = $state<HTMLInputElement>();
  let numberInput = $state<HTMLInputElement>();
  let brandRow = $state<HTMLDivElement>();

  onMount(async () => {
    await tick();
    if (!old) nameInput?.focus();
    // 机构是一行横滑，选中的可能在屏幕外。
    brandRow?.querySelector(".on")?.scrollIntoView({ inline: "center", block: "nearest" });
  });

  function typed() {
    error = null;
    if (!picked) brand = guessBrand(name);
  }

  function pick(b: string | null) {
    brand = b;
    picked = true;
  }

  async function setCard(t: typeof cardType) {
    const opening = cardType === "none" && t !== "none";
    cardType = t;
    error = null;
    if (!opening) return;
    holdKeyboard("numeric");
    await tick();
    numberInput?.focus();
  }

  /** 正在等服务器：不重复提交。 */
  let busy = $state(false);

  async function submit() {
    if (busy) return;
    const n = name.trim();
    const digits = number.replace(/\s/g, "");
    if (!n) return void (error = { at: "name", text: "填一个名称" });
    if (cardType !== "none" && !/^\d{12,19}$/.test(digits)) return void (error = { at: "card", text: "卡号是 12～19 位数字" });
    const card = cardType === "none" ? null : { type: cardType, number: digits };
    busy = true;
    const err = await saveAccount({ id: old?.id ?? 0, name: n, brand, card, external });
    busy = false;
    if (err) return void (error = { at: "name", text: err });
    onclose();
  }

  async function remove() {
    if (busy) return;
    busy = true;
    const err = await removeAccount(old!.id);
    busy = false;
    if (err) return toast(err);
    onclose();
  }
</script>

<SheetHead icon="card" title={old ? "编辑账户" : "新建账户"} {onclose} action={{ label: "完成", run: submit }} />

<div class="body">
  <div class="name">
    <AccountLogo account={{ name: name || "?", brand }} size={44} />
    <input
      bind:this={nameInput}
      bind:value={name}
      placeholder="名称，如「工资卡」"
      enterkeyhint="done"
      oninput={typed}
      onkeydown={(e) => e.key === "Enter" && submit()}
    />
  </div>
  {#if error?.at === "name"}<p class="hint bad">{error.text}</p>{/if}

  <h3>机构</h3>
  <div class="brands" bind:this={brandRow}>
    {#each LOGOS as [b] (b)}
      <button class:on={brand === b} onclick={() => pick(b)}>
        <AccountLogo account={{ name: b, brand: b }} size={38} />
        <small>{b}</small>
      </button>
    {/each}
    <button class:on={brand === null} onclick={() => pick(null)}>
      <AccountLogo account={{ name: name || "其", brand: null }} size={38} />
      <small>其他</small>
    </button>
  </div>

  <h3>银行卡</h3>
  <div class="seg">
    {#each [["none", "无"], ["debit", "储蓄卡"], ["credit", "信用卡"]] as [k, label] (k)}
      <button class:on={cardType === k} onclick={() => setCard(k as typeof cardType)}>{label}</button>
    {/each}
  </div>
  {#if cardType !== "none"}
    <input
      class="num"
      bind:this={numberInput}
      bind:value={number}
      inputmode="numeric"
      placeholder="完整卡号"
      enterkeyhint="done"
      onkeydown={(e) => e.key === "Enter" && submit()}
      oninput={() => (error = null)}
    />
  {/if}
  {#if error?.at === "card"}<p class="hint bad">{error.text}</p>{/if}
  <h3>余额</h3>
  <div class="seg">
    {#each [[false, "自己的钱"], [true, "外部，不记余额"]] as const as [k, label] (label)}
      <button class:on={external === k} onclick={() => (external = k)}>{label}</button>
    {/each}
  </div>
  {#if external}<p class="hint">如「公司付」：餐补照常算支出，不扣自己的余额，不进净资产，不能平账。</p>{/if}

  <p class="hint">识别时模型看到账户名、机构、卡类型和卡号尾号；完整卡号只存在自己的库里。</p>

  {#if old}
    <button class="remove" onclick={remove}>删除账户</button>
  {/if}
</div>

<style>
  .body {
    display: grid;
    gap: 10px;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    overscroll-behavior: contain;
    padding: 4px 16px 12px;
  }
  h3 {
    margin: 10px 4px -2px;
    font-size: 13px;
    font-weight: 500;
    color: var(--label2);
  }
  .name {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 8px 8px 10px;
    border-radius: var(--radius-group);
    background: var(--card);
  }
  .name input {
    flex: 1;
    min-width: 0;
    height: 40px;
    padding: 0;
    background: none;
    font-size: 18px;
    font-weight: 600;
    letter-spacing: 0;
  }
  .brands {
    display: flex;
    gap: 4px;
    padding: 8px 4px;
    overflow-x: auto;
    scrollbar-width: none;
    border-radius: var(--radius-group);
    background: var(--card);
  }
  .brands::-webkit-scrollbar {
    display: none;
  }
  .brands button {
    flex: none;
    width: 62px;
    display: grid;
    justify-items: center;
    gap: 4px;
    padding: 6px 0;
    border: 0;
    border-radius: 12px;
    background: none;
    color: var(--label);
    font: inherit;
    transition: background-color 0.2s;
  }
  .brands button.on {
    background: color-mix(in srgb, var(--tint) 14%, transparent);
    box-shadow: inset 0 0 0 1.5px var(--tint);
  }
  .brands small {
    font-size: 11px;
    white-space: nowrap;
  }
  .seg {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
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
    margin-top: 8px;
    height: 48px;
    border: 0;
    border-radius: var(--radius-group);
    background: var(--card);
    color: var(--red);
    font: inherit;
    font-size: 16px;
  }
</style>
