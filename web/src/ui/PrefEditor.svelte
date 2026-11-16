<script lang="ts">
  // 设置里的记账周期、预算：周期点一下即保存；预算按「完成」保存。
  import { onMount, tick } from "svelte";
  import Icon from "./Icon.svelte";
  import { prefs, setPrefs } from "../lib/store.svelte";
  import { parseCents, yuanInput } from "../lib/format";

  let { which, onclose }: { which: "cycle" | "budget"; onclose: () => void } = $props();

  let text = $state(prefs.budget === null ? "" : yuanInput(prefs.budget));
  let error = $state("");
  let input = $state<HTMLInputElement>();

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
    setPrefs({ budget });
    onclose();
  }
</script>

<header class="head">
  <button class="round" onclick={onclose} aria-label="关闭"><Icon name="xmark" size={14} weight={2.6} /></button>
  <h2>
    <span class="glyph"><Icon name={which === "cycle" ? "calendar" : "target"} size={19} weight={2.2} /></span>{which === "cycle" ? "记账周期" : "每期预算"}
  </h2>
  {#if which === "budget"}<button class="link" onclick={submit}>完成</button>{:else}<span></span>{/if}
</header>

<div class="body">
  {#if which === "cycle"}
    <p class="hint">每月几号开始算一期。发工资那天开始比较顺手。</p>
    <div class="days">
      {#each Array.from({ length: 28 }, (_, i) => i + 1) as d (d)}
        <button
          class="num"
          class:on={prefs.cycleDay === d}
          onclick={() => {
            setPrefs({ cycleDay: d });
            onclose();
          }}>{d}</button
        >
      {/each}
    </div>
    <p class="hint">{prefs.cycleDay === 1 ? "现在按自然月统计" : `现在每月 ${prefs.cycleDay} 号到下月 ${prefs.cycleDay - 1} 号算一期`}</p>
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
    {#if prefs.budget !== null}<button class="clear" onclick={() => done(null)}>不设预算</button>{/if}
  {/if}
</div>

<style>
  .head {
    display: grid;
    grid-template-columns: 80px 1fr 80px;
    align-items: center;
    padding: 0 16px 10px;
  }
  .glyph {
    display: flex;
    color: var(--tint);
  }
  h2 {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 6px;
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
    padding: 6px 0;
    color: var(--tint);
    font: inherit;
    font-size: 17px;
    font-weight: 600;
  }
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
