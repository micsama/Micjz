<script lang="ts">
  // 新建或编辑分类的紧凑面板：名称（emoji 直接打在开头）、给模型看的 key、上级；按「完成」保存。
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
  const note = $derived(
    error ||
      (fallback
        ? "兜底分类：识别不出时用它，key 和层级固定"
        : subCount
          ? `带着 ${subCount} 个二级，只能是一级`
          : "key 给识别用，留空自动生成；小写英文、数字、下划线"),
  );

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

<SheetHead icon="grid" title={old ? "编辑分类" : parentId === null ? "新建一级分类" : "新建子类"} {onclose} action={{ label: "完成", run: submit }} />

<div class="body">
  <div class="card">
    <label class="line">
      <span class="icon">{badge(name || "?").icon}</span>
      <input
        bind:this={nameInput}
        bind:value={name}
        placeholder="名称，开头可放 emoji"
        enterkeyhint="done"
        oninput={() => (error = "")}
        onkeydown={(e) => e.key === "Enter" && submit()}
      />
    </label>
    <label class="line">
      <span class="label">key</span>
      <input
        class="key"
        bind:value={key}
        disabled={fallback}
        placeholder="留空自动生成"
        autocapitalize="off"
        autocomplete="off"
        spellcheck="false"
        enterkeyhint="done"
        oninput={() => (error = "")}
        onkeydown={(e) => e.key === "Enter" && submit()}
      />
    </label>
    <div class="line">
      <span class="label">上级</span>
      <div class="parents">
        <button class:on={parentId === null} disabled={fixedParent} onclick={() => (parentId = null)}>无</button>
        {#each parents as p (p.id)}
          <button class:on={parentId === p.id} disabled={fixedParent} onclick={() => (parentId = p.id)}>{p.name}</button>
        {/each}
      </div>
    </div>
  </div>
  <div class="foot">
    <p class:bad={!!error}>{note}</p>
    {#if old && !fallback}<button class="remove" onclick={remove}>删除</button>{/if}
  </div>
</div>

<style>
  .body {
    display: grid;
    gap: 8px;
    padding: 0 16px 4px;
  }
  .card {
    border-radius: var(--radius-group);
    background: var(--card);
    overflow: hidden;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 46px;
    padding: 0 12px;
  }
  .line + .line {
    box-shadow: inset 0 0.5px 0 var(--sep);
  }
  .icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 10px;
    background: var(--fill);
    font-size: 18px;
  }
  .label {
    flex: none;
    width: 32px;
    font-size: 14px;
    color: var(--label2);
  }
  input {
    flex: 1;
    min-width: 0;
    height: 44px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--label);
    font: inherit;
    font-size: 17px;
    outline: none;
  }
  .key {
    font-family: ui-monospace, monospace;
    font-size: 15px;
  }
  .key:disabled {
    color: var(--label2);
  }
  .parents {
    flex: 1;
    min-width: 0;
    display: flex;
    gap: 6px;
    padding: 7px 0;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .parents::-webkit-scrollbar {
    display: none;
  }
  .parents button {
    flex: none;
    padding: 0 10px;
    height: 30px;
    border: 0;
    border-radius: 10px;
    background: var(--fill);
    color: var(--label);
    font: inherit;
    font-size: 14px;
    white-space: nowrap;
  }
  .parents button.on {
    background: var(--tint);
    color: white;
  }
  .parents button:disabled:not(.on) {
    opacity: 0.4;
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 4px;
  }
  .foot p {
    flex: 1;
    margin: 0;
    font-size: 12px;
    line-height: 1.4;
    color: var(--label2);
  }
  .foot p.bad {
    color: var(--red);
  }
  .remove {
    flex: none;
    padding: 6px 12px;
    border: 0;
    border-radius: 10px;
    background: var(--card);
    color: var(--red);
    font: inherit;
    font-size: 14px;
  }
</style>
