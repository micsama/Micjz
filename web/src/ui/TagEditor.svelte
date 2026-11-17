<script lang="ts">
  // 新建或改名标签；按「完成」保存。
  import { onMount, tick } from "svelte";
  import SheetHead from "./SheetHead.svelte";
  import { removeTag, saveTag } from "../lib/store.svelte";
  import type { Tag } from "../lib/types";

  /** `tag` 为空是新建。 */
  let { tag, onclose }: { tag: Tag | null; onclose: () => void } = $props();

  // svelte-ignore state_referenced_locally
  const old = tag;
  let name = $state(old?.name ?? "");
  let error = $state("");
  let busy = $state(false);
  let input = $state<HTMLInputElement>();

  onMount(async () => {
    await tick();
    input?.focus();
  });

  async function submit() {
    if (busy) return;
    if (!name.trim()) return void (error = "填一个名称");
    busy = true;
    error = (await saveTag(old?.id ?? null, name)) ?? "";
    busy = false;
    if (!error) onclose();
  }

  async function remove() {
    if (busy) return;
    busy = true;
    error = (await removeTag(old!.id)) ?? "";
    busy = false;
    if (!error) onclose();
  }
</script>

<SheetHead icon="tag" title={old ? "编辑标签" : "新建标签"} {onclose} action={{ label: "完成", run: submit }} />

<div class="body">
  <input
    bind:this={input}
    bind:value={name}
    placeholder="名称，如「恋爱」"
    enterkeyhint="done"
    oninput={() => (error = "")}
    onkeydown={(e) => e.key === "Enter" && submit()}
  />
  {#if error}<p class="hint bad">{error}</p>{/if}
  <p class="hint">标签只用来筛选和统计，识别时不会自动打。</p>
  {#if old}
    <button class="remove" onclick={remove}>删除标签</button>
  {/if}
</div>

<style>
  .body {
    display: grid;
    gap: 10px;
    padding: 4px 16px 12px;
  }
  input {
    height: 50px;
    padding: 0 16px;
    border: 0;
    border-radius: var(--radius-group);
    background: var(--card);
    color: var(--label);
    font: inherit;
    font-size: 18px;
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
