<script lang="ts">
  // 新建或编辑分类：名称（emoji 直接打在名字开头）、给模型看的 key、上级；按「完成」保存。
  import { onMount, tick } from "svelte";
  import SheetHead from "./SheetHead.svelte";
  import { fallbackOf, removeCategory, saveCategory, subsOf, tops } from "../lib/store.svelte";
  import { badge } from "../lib/format";
  import type { Category, FlowKind } from "../lib/types";

  /** `category` 为空是新建，新建时放在 `kind` 类型、`parent` 下面。 */
  let { category, kind, parent, onclose }: { category: Category | null; kind: FlowKind; parent: number | null; onclose: () => void } = $props();

  // 面板打开时按当时的分类初始化，之后由输入框接管。
  // svelte-ignore state_referenced_locally
  const old = category;
  // svelte-ignore state_referenced_locally
  let parentId = $state(old ? old.parent_id : parent);
  let name = $state(old?.name ?? "");
  let key = $state(old?.key ?? "");
  let error = $state("");
  let busy = $state(false);
  let nameInput = $state<HTMLInputElement>();

  const fallback = $derived(old !== null && fallbackOf(old.kind).id === old.id);
  const subCount = $derived(old ? subsOf(old).length : 0);
  /** 兜底分类必须是一级；带着二级的不能挪到别的一级下面。 */
  const fixedParent = $derived(fallback || subCount > 0);
  const parents = $derived(tops(kind).filter((c) => c.id !== old?.id));

  onMount(async () => {
    await tick();
    if (!old) nameInput?.focus();
  });

  async function submit() {
    if (busy) return;
    if (!name.trim()) return void (error = "填一个名称");
    busy = true;
    error = (await saveCategory(old?.id ?? null, { kind: old ? undefined : kind, parent_id: parentId, name, key })) ?? "";
    busy = false;
    if (!error) onclose();
  }

  async function remove() {
    if (busy) return;
    busy = true;
    error = (await removeCategory(old!.id)) ?? "";
    busy = false;
    if (!error) onclose();
  }
</script>

<SheetHead icon="grid" title={old ? "编辑分类" : "新建分类"} {onclose} action={{ label: "完成", run: submit }} />

<div class="body">
  <div class="name">
    <span class="icon">{badge(name || "?").icon}</span>
    <input
      bind:this={nameInput}
      bind:value={name}
      placeholder="名称，开头可以放 emoji"
      enterkeyhint="done"
      oninput={() => (error = "")}
      onkeydown={(e) => e.key === "Enter" && submit()}
    />
  </div>
  {#if error}<p class="hint bad">{error}</p>{/if}

  <h3>key</h3>
  <input
    class="key"
    bind:value={key}
    disabled={fallback}
    placeholder={old ? "" : "留空自动生成"}
    autocapitalize="off"
    autocomplete="off"
    spellcheck="false"
    enterkeyhint="done"
    oninput={() => (error = "")}
    onkeydown={(e) => e.key === "Enter" && submit()}
  />
  <p class="hint">
    {fallback ? "兜底分类「其他」的 key 固定，识别不出分类时用它。" : "识别时模型按它选分类：小写英文、数字、下划线。随时可改，下次识别生效。"}
  </p>

  <h3>上级</h3>
  <div class="parents">
    <button class:on={parentId === null} disabled={fixedParent} onclick={() => (parentId = null)}>无，作为一级</button>
    {#each parents as p (p.id)}
      <button class:on={parentId === p.id} disabled={fixedParent} onclick={() => (parentId = p.id)}>{p.name}</button>
    {/each}
  </div>
  {#if fallback}
    <p class="hint">兜底分类必须是一级。</p>
  {:else if subCount}
    <p class="hint">下面还有 {subCount} 个二级，不能挪到别的一级下面。</p>
  {/if}

  {#if old && !fallback}
    <button class="remove" onclick={remove}>删除分类</button>
  {/if}
</div>

<style>
  .body {
    display: grid;
    gap: 10px;
    padding: 4px 16px 12px;
    overflow-y: auto;
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
  .icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 14px;
    background: var(--fill);
    font-size: 24px;
  }
  input {
    height: 50px;
    padding: 0 16px;
    border: 0;
    border-radius: var(--radius-group);
    background: var(--card);
    color: var(--label);
    font: inherit;
    font-size: 17px;
    outline: none;
  }
  .name input {
    flex: 1;
    min-width: 0;
    height: 40px;
    padding: 0;
    background: none;
    font-size: 18px;
    font-weight: 600;
  }
  .key {
    font-family: ui-monospace, monospace;
  }
  .key:disabled {
    color: var(--label2);
  }
  .parents {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 8px;
    border-radius: var(--radius-group);
    background: var(--card);
  }
  .parents button {
    padding: 7px 10px;
    min-height: 36px;
    border: 0;
    border-radius: 12px;
    background: var(--fill);
    color: var(--label);
    font: inherit;
    font-size: 14px;
  }
  .parents button.on {
    background: var(--tint);
    color: white;
  }
  .parents button:disabled:not(.on) {
    opacity: 0.4;
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
