<script lang="ts">
  // 统计筛选的一项：选完即生效并收起；分类选了带二级的一级，面板留着接着选二级，也可以就停在一级。
  import { app, categoryOf, setFilter, type Dim } from "../lib/store.svelte";
  import type { FlowKind } from "../lib/types";
  import type { IconName } from "./Icon.svelte";
  import SheetHead from "./SheetHead.svelte";
  import AccountChoices from "./AccountChoices.svelte";
  import TagChoices from "./TagChoices.svelte";
  import CategoryChoices from "./CategoryChoices.svelte";
  import Segmented from "./Segmented.svelte";

  let { dim }: { dim: Dim } = $props();

  const LOOK: Record<Dim, { icon: IconName; label: string }> = {
    account: { icon: "card", label: "账户" },
    category: { icon: "grid", label: "分类" },
    tag: { icon: "tag", label: "标签" },
  };
  const look = $derived(LOOK[dim]);
  // 支出、收入分类分开看，打开时停在当前筛选所属的那边。
  let kind = $state<FlowKind>(categoryOf(app.filter.category)?.kind ?? "expense");
</script>

<SheetHead
  icon={look.icon}
  title={`按${look.label}筛选`}
  onclose={() => (app.picking = null)}
  action={app.filter[dim] === null ? undefined : { label: "清除", quiet: true, run: () => setFilter(dim, null) }}
/>

<div class="body">
  {#if dim === "account"}
    <AccountChoices selected={app.filter.account} onselect={(id) => setFilter("account", id)} />
  {:else if dim === "tag"}
    <TagChoices selected={app.filter.tag === null ? [] : [app.filter.tag]} onselect={(id) => setFilter("tag", id)} />
  {:else}
    <div class="kind">
      <Segmented
        label="分类类型"
        options={[
          ["expense", "支出"],
          ["income", "收入"],
        ]}
        value={kind}
        onchange={(k) => (kind = k)}
      />
    </div>
    <CategoryChoices {kind} selected={app.filter.category} onselect={(id, final) => setFilter("category", id, !final)} />
  {/if}
</div>

<style>
  .kind {
    margin-bottom: 14px;
  }
  .body {
    overflow-y: auto;
    overflow-x: hidden;
    touch-action: pan-y;
    padding: 4px 16px 8px;
    -webkit-overflow-scrolling: touch;
  }
</style>
