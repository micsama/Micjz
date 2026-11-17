<script lang="ts">
  // 设置里推入的分类、标签管理页。分类分支出、收入，一级点进去看二级，长按拖动排序；点行打开编辑面板。
  import Icon from "./Icon.svelte";
  import Sheet from "./Sheet.svelte";
  import Segmented from "./Segmented.svelte";
  import CategoryEditor from "./CategoryEditor.svelte";
  import TagEditor from "./TagEditor.svelte";
  import { holdKeyboard } from "./nav.svelte";
  import { Reorder } from "./reorder.svelte";
  import { app, categoryOf, fallbackOf, moveCategory, subsOf, topIndex, tops } from "../lib/store.svelte";
  import { badge } from "../lib/format";
  import { color } from "../lib/look";
  import type { Category, FlowKind, Tag } from "../lib/types";

  let { which, onback }: { which: "categories" | "tags"; onback: () => void } = $props();

  let kind = $state<FlowKind>("expense");
  /** 点进去的一级；按 id 记，改名、挪位后仍指向它。 */
  let openId = $state<number | null>(null);
  /** 被删掉或挪成二级后自动回到一级列表。 */
  const top = $derived(categoryOf(openId)?.parent_id === null ? categoryOf(openId) : undefined);
  const list = $derived(top ? subsOf(top) : tops(kind));
  /** 正在编辑的分类或标签；`"new"` 是新建。 */
  let editing = $state<Category | Tag | "new" | null>(null);
  const reorder = new Reorder((from, to) => moveCategory(list[from], to));

  const title = $derived(which === "tags" ? "标签" : (top?.name ?? "分类"));

  function edit(x: Category | Tag | "new") {
    holdKeyboard();
    editing = x;
  }
</script>

<div class="page">
  <header class="bar">
    <button class="back" onclick={() => (top ? (openId = null) : onback())}><Icon name="chevron-left" size={22} weight={2.6} />{top ? "分类" : "设置"}</button>
    <span class="t">{title}</span>
    {#if top}<button class="link" onclick={() => edit(top)}>编辑</button>{:else}<span></span>{/if}
  </header>

  {#if which === "tags"}
    <div class="group">
      {#each app.catalog.tags as t (t.id)}
        <button class="row" onclick={() => edit(t)}>
          <span class="mark"><Icon name="tag" size={17} weight={2.2} /></span>
          <span class="name">{t.name}</span>
          <span class="chev"><Icon name="chevron-right" size={14} weight={2.6} /></span>
        </button>
      {:else}
        <p class="empty">还没有标签</p>
      {/each}
    </div>
    <button class="add" onclick={() => edit("new")}><Icon name="plus" size={16} weight={2.4} />添加标签</button>
    <p class="foot">标签按添加顺序排列；还有账目在用的不能删除。</p>
  {:else}
    {#if !top}
      <div class="seg">
        <Segmented
          label="分类类型"
          options={[
            ["expense", "支出"],
            ["income", "收入"],
          ]}
          value={kind}
          onchange={(v) => (kind = v)}
        />
      </div>
    {/if}
    {#if list.length}
      <div class="group" class:sorting={reorder.from >= 0} {@attach reorder.attach}>
        {#each list as c, i (c.id)}
          {@const subs = top ? 0 : subsOf(c).length}
          <div class="item" class:lifted={reorder.from === i} style:transform="translateY({reorder.shift(i)}px)">
            <button class="row" onclick={() => (top ? edit(c) : (openId = c.id))}>
              <span class="mark" style:--mc={color(top ? topIndex(top) : i)}>{badge(c.name).icon}</span>
              <span class="main">
                <span class="name"
                  >{badge(c.name).text}{#if c.id === fallbackOf(c.kind).id}<em>兜底</em>{/if}</span
                >
                <span class="sub">{c.key}</span>
              </span>
              {#if !top}<span class="count">{subs ? `${subs} 个二级` : ""}</span>{/if}
              <span class="chev"><Icon name="chevron-right" size={14} weight={2.6} /></span>
            </button>
          </div>
        {/each}
      </div>
    {:else}
      <p class="empty alone">还没有二级分类，记账时直接选「{top?.name}」</p>
    {/if}
    <button class="add" onclick={() => edit("new")}><Icon name="plus" size={16} weight={2.4} />{top ? "添加二级分类" : "添加一级分类"}</button>
    <p class="foot">
      {top
        ? "长按拖动排序。右上角「编辑」改这个一级本身。"
        : "点一级看它的二级，长按拖动排序。emoji 直接打在名称开头。还有账目在用的分类不能删除，先把账改到别的分类。"}
    </p>
  {/if}
  <div class="tail"></div>
</div>

<Sheet open={editing !== null} onclose={() => (editing = null)}>
  {#if editing && which === "tags"}
    <TagEditor tag={editing === "new" ? null : (editing as Tag)} onclose={() => (editing = null)} />
  {:else if editing}
    <CategoryEditor
      category={editing === "new" ? null : (editing as Category)}
      kind={top?.kind ?? kind}
      parent={top?.id ?? null}
      onclose={() => (editing = null)}
    />
  {/if}
</Sheet>

<style>
  .page {
    position: absolute;
    inset: 0;
    overflow-y: auto;
    overflow-x: hidden;
    touch-action: pan-y;
    -webkit-overflow-scrolling: touch;
    padding-top: env(safe-area-inset-top);
  }
  .bar {
    position: sticky;
    top: calc(-1 * env(safe-area-inset-top));
    z-index: 5;
    margin-top: calc(-1 * env(safe-area-inset-top));
    padding: env(safe-area-inset-top) 8px 0;
    height: 44px;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    background: var(--bar-bg);
    backdrop-filter: saturate(180%) blur(24px);
    -webkit-backdrop-filter: saturate(180%) blur(24px);
    box-shadow: 0 0.5px 0 var(--sep);
  }
  .back,
  .link {
    display: flex;
    align-items: center;
    gap: 1px;
    border: 0;
    background: none;
    color: var(--tint);
    font: inherit;
    font-size: 17px;
    padding: 6px 4px;
  }
  .back {
    justify-self: start;
  }
  .link {
    justify-self: end;
    padding-right: 8px;
  }
  .back:active,
  .link:active {
    opacity: 0.4;
  }
  .t {
    font-weight: 600;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .seg {
    margin: 12px 16px 0;
  }
  .group {
    margin: 16px 16px 0;
    border-radius: var(--radius-group);
    overflow: hidden;
    background: var(--card);
    box-shadow: var(--shadow-card);
  }
  .item {
    position: relative;
    background: var(--card);
  }
  .sorting .item {
    transition: transform 0.25s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .sorting .item.lifted {
    z-index: 2;
    transition: none;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  }
  .group > * + *::before {
    content: "";
    position: absolute;
    z-index: 1;
    left: 60px;
    right: 0;
    top: 0;
    height: 0.5px;
    background: var(--sep);
  }
  .row {
    box-sizing: border-box;
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 56px;
    padding: 8px 14px;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    font-size: 16px;
    text-align: left;
    -webkit-user-select: none;
    user-select: none;
    -webkit-touch-callout: none;
  }
  .row:active {
    background: var(--press);
  }
  .mark {
    flex: none;
    width: 34px;
    height: 34px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    font-size: 20px;
    color: var(--tint);
    background: color-mix(in srgb, var(--mc, var(--tint)) 16%, var(--card));
  }
  .main {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: 1px;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name em {
    margin-left: 6px;
    padding: 1px 6px;
    border-radius: 6px;
    background: var(--fill);
    color: var(--label2);
    font-size: 11px;
    font-style: normal;
    vertical-align: 2px;
  }
  .sub {
    font-family: ui-monospace, monospace;
    font-size: 12px;
    color: var(--label2);
  }
  .count {
    flex: none;
    font-size: 14px;
    color: var(--label2);
  }
  .chev {
    display: flex;
    color: var(--label3);
  }
  .empty {
    margin: 0;
    padding: 16px;
    text-align: center;
    font-size: 14px;
    color: var(--label2);
  }
  .empty.alone {
    margin: 16px 16px 0;
  }
  .add {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    width: calc(100% - 32px);
    margin: 16px 16px 0;
    height: 48px;
    border: 0;
    border-radius: var(--radius-group);
    background: var(--card);
    box-shadow: var(--shadow-card);
    color: var(--tint);
    font: inherit;
    font-size: 16px;
  }
  .add:active {
    background: var(--press);
  }
  .foot {
    margin: 10px 32px 0;
    font-size: 13px;
    line-height: 1.45;
    color: var(--label2);
  }
  .tail {
    height: calc(env(safe-area-inset-bottom) + 40px);
  }
</style>
