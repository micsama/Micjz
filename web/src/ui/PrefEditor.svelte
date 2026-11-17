<script lang="ts">
  // 设置里的记账周期、预算：周期点一下即保存；预算按「完成」保存。
  import { onMount, tick } from "svelte";
  import SheetHead from "./SheetHead.svelte";
  import { settings, saveSettings } from "../lib/store.svelte";
  import { cycleText, parseCents, yuanInput } from "../lib/format";

  let { which, onclose }: { which: "cycle" | "budget"; onclose: () => void } = $props();

  let text = $state(settings.budget_cents === null ? "" : yuanInput(settings.budget_cents));
  let error = $state("");
  let input = $state<HTMLInputElement>();
  /** 周期分两组：本月 N 号起（正数）、上月 N 号起（负数，没有 1 号）。 */
  let lastMonth = $state(settings.cycle_day < 0);

  onMount(async () => {
    if (which !== "budget") return;
    await tick();
    input?.focus();
    input?.select();
  });

  function submit() {
    if (!text.trim()) return done(null);
    const cents = parseCents(text);
    if (cents === null) {
      error = "预算要大于 0，最多两位小数";
      return;
    }
    done(cents);
  }

  function done(budget: number | null) {
    saveSettings({ budget_cents: budget });
    onclose();
  }
</script>

<SheetHead
  icon={which === "cycle" ? "calendar" : "target"}
  title={which === "cycle" ? "记账周期" : "每期预算"}
  {onclose}
  action={which === "budget" ? { label: "完成", run: submit } : undefined}
/>

<div class="body">
  {#if which === "cycle"}
    <p class="hint">每月几号开始算一期。发工资那天开始比较顺手。</p>
    <div class="seg">
      <button class:on={!lastMonth} onclick={() => (lastMonth = false)}>本月 N 号起</button>
      <button class:on={lastMonth} onclick={() => (lastMonth = true)}>上月 N 号起</button>
    </div>
    <div class="days">
      {#each Array.from({ length: 28 }, (_, i) => i + 1) as d (d)}
        {@const value = lastMonth ? -d : d}
        <button
          class="num"
          class:on={settings.cycle_day === value}
          disabled={value === -1}
          onclick={() => {
            saveSettings({ cycle_day: value });
            onclose();
          }}>{d}</button
        >
      {/each}
    </div>
    <p class="hint">
      {lastMonth ? "如选 25：10 月 25 日到 11 月 24 日算「11 月」" : "如选 5：11 月 5 日到 12 月 4 日算「11 月」"}<br />
      现在是{cycleText(settings.cycle_day)}。改了之后历史周期都按新规则重算，账目不动。
    </p>
  {:else}
    <label class="amount" class:bad={error}>
      <span class="cur">¥</span>
      <input
        bind:this={input}
        bind:value={text}
        inputmode="decimal"
        enterkeyhint="done"
        placeholder="不设"
        onkeydown={(e) => e.key === "Enter" && submit()}
        oninput={() => (error = "")}
      />
    </label>
    <p class="hint" class:bad={error}>{error || "每个记账周期的支出上限，留空就是不设"}</p>
    {#if settings.budget_cents !== null}<button class="clear" onclick={() => done(null)}>不设预算</button>{/if}
  {/if}
</div>

<style>
  .body {
    padding: 4px 16px 12px;
  }
  .hint {
    margin: 4px 4px 12px;
    text-align: center;
    font-size: 13px;
    color: var(--label2);
  }
  .days {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 6px;
    margin-bottom: 12px;
  }
  .days button {
    aspect-ratio: 1;
    border: 0;
    border-radius: 50%;
    background: var(--card);
    color: var(--label);
    font: inherit;
    font-size: 16px;
    font-weight: 500;
    transition: transform 0.2s;
  }
  .seg {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    margin-bottom: 12px;
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
  }
  .seg button.on {
    background: var(--card);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.12);
    font-weight: 600;
  }
  .days button:disabled {
    opacity: 0.3;
  }
  .days button.on {
    background: var(--tint);
    color: #fff;
    font-weight: 700;
  }
  .days button:active {
    transform: scale(0.88);
  }
  .amount {
    display: flex;
    align-items: baseline;
    justify-content: center;
    gap: 4px;
    padding: 18px 0 6px;
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
    color: var(--label);
    caret-color: var(--tint);
  }
  .amount input::placeholder {
    color: var(--label3);
  }
  .bad {
    color: var(--red);
  }
  .clear {
    display: block;
    width: 100%;
    height: 48px;
    border: 0;
    border-radius: var(--radius-group);
    background: var(--card);
    color: var(--red);
    font: inherit;
    font-size: 16px;
  }
</style>
