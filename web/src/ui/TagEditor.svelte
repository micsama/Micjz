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
  <div class="foot">
    <p class:bad={!!error}>{error || "标签只用来筛选和统计，识别时不会自动打。"}</p>
    {#if old}<button class="remove" onclick={remove}>删除</button>{/if}
  </div>
</div>

<style>
  .body {
    display: grid;
    gap: 8px;
    padding: 0 16px 4px;
  }
  input {
    height: 46px;
    padding: 0 14px;
    border: 0;
    border-radius: var(--radius-group);
    background: var(--card);
    color: var(--label);
    font: inherit;
    font-size: 17px;
    outline: none;
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
