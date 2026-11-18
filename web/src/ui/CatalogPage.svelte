<script lang="ts">
  // 设置里推入的分类、标签管理页。
  // 分类：一级一行，点开原地展开它的二级（小块平铺）；铅笔改一级，点二级小块改二级；长按拖动排序。
  // 标签：小块平铺，点开改名。
  import Icon from "./Icon.svelte";
  import Sheet from "./Sheet.svelte";
  import Segmented from "./Segmented.svelte";
  import CategoryEditor from "./CategoryEditor.svelte";
  import TagEditor from "./TagEditor.svelte";
  import { holdKeyboard } from "./nav.svelte";
  import { ChipReorder, Reorder } from "./reorder.svelte";
  import { app, categoryOf, fallbackOf, moveCategory, subsOf, tops } from "../lib/store.svelte";
  import { badge } from "../lib/format";
  import { color } from "../lib/look";
  import type { Category, FlowKind, Tag } from "../lib/types";

  /** `from` 是返回按钮上的上一页名称；`initialKind` 是分类先显示支出还是收入。 */
  let {
    which,
    onback,
    from = "设置",
    initialKind = "expense",
  }: { which: "categories" | "tags"; onback: () => void; from?: string; initialKind?: FlowKind } = $props();

  // svelte-ignore state_referenced_locally
  let kind = $state<FlowKind>(initialKind);
  const list = $derived(tops(kind));
  /** 展开的一级；一次只展开一个。 */
  let openId = $state<number | null>(null);
  const open = $derived(categoryOf(openId));
  const subs = $derived(open ? subsOf(open) : []);

  type Editing = { category: Category | null; parent: number | null } | { tag: Tag | null };
  let editing = $state<Editing | null>(null);

  const reorder = new Reorder((from, to) => moveCategory(list[from], to));
  const chips = new ChipReorder((from, to) => moveCategory(subs[from], to));

  function edit(x: Editing) {
    holdKeyboard();
    editing = x;
  }
</script>

<div class="page">
  <header class="bar">
    <button class="back" onclick={onback}><Icon name="chevron-left" size={22} weight={2.6} />{from}</button>
    <span class="t">{which === "tags" ? "标签" : "分类"}</span>
    <span></span>
  </header>

  {#if which === "tags"}
    <div class="group cloud">
      {#each app.catalog.tags as t (t.id)}
        <button class="chip" onclick={() => edit({ tag: t })}><Icon name="tag" size={13} weight={2.2} />{t.name}</button>
      {/each}
      <button class="chip add" onclick={() => edit({ tag: null })}><Icon name="plus" size={13} weight={2.6} />添加标签</button>
    </div>
    <p class="foot">点标签改名或删除。还有账目在用的不能删除。</p>
  {:else}
    <div class="seg">
      <Segmented
        label="分类类型"
        options={[
          ["expense", "支出"],
          ["income", "收入"],
        ]}
        value={kind}
        onchange={(v) => {
          kind = v;
          openId = null;
        }}
      />
    </div>
    <div class="group" class:sorting={reorder.from >= 0} {@attach reorder.attach}>
      {#each list as c, i (c.id)}
        {@const n = subsOf(c).length}
        {@const expanded = c.id === openId}
        <div class="item" class:lifted={reorder.from === i} style:transform="translateY({reorder.shift(i)}px)" style:--mc={color(i)}>
          <div class="row">
            <button class="head" aria-expanded={expanded} onclick={() => (openId = expanded ? null : c.id)}>
              <span class="mark">{badge(c.name).icon}</span>
              <span class="name"
                >{badge(c.name).text}{#if c.id === fallbackOf(c.kind).id}<em>兜底</em>{/if}</span
              >
              <span class="count">{n || ""}</span>
              <span class="chev" class:up={expanded}><Icon name="chevron-down" size={14} weight={2.6} /></span>
            </button>
            <button class="pencil" aria-label="编辑{c.name}" onclick={() => edit({ category: c, parent: null })}>
              <Icon name="pencil" size={16} weight={2.2} />
            </button>
          </div>
          {#if expanded}
            <div class="subs" class:sorting={chips.from >= 0} {@attach chips.attach}>
              {#each subs as s, j (s.id)}
                <button
                  class="chip"
                  data-drag
                  class:lifted={chips.from === j}
                  class:before={chips.from > j && chips.to === j}
                  class:after={chips.from >= 0 && chips.from < j && chips.to === j}
                  style:transform={chips.from === j ? `translate(${chips.dx}px, ${chips.dy}px)` : undefined}
                  onclick={() => edit({ category: s, parent: c.id })}>{s.name}</button
                >
              {/each}
              <button class="chip add" onclick={() => edit({ category: null, parent: c.id })}><Icon name="plus" size={13} weight={2.6} />添加子类</button>
            </div>
          {/if}
        </div>
      {/each}
    </div>
    <button class="addtop" onclick={() => edit({ category: null, parent: null })}><Icon name="plus" size={16} weight={2.4} />添加一级分类</button>
    <p class="foot">点一级展开它的子类，铅笔改一级本身；一级和子类都能长按拖动排序。emoji 直接打在名称开头。还有账目在用的分类不能删除。</p>
  {/if}
  <div class="tail"></div>
</div>

<Sheet open={editing !== null} onclose={() => (editing = null)}>
  {#if editing && "tag" in editing}
    <TagEditor tag={editing.tag} onclose={() => (editing = null)} />
  {:else if editing}
    <CategoryEditor category={editing.category} {kind} parent={editing.parent} onclose={() => (editing = null)} />
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
  .back {
    justify-self: start;
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
  .back:active {
    opacity: 0.4;
  }
  .t {
    font-weight: 600;
  }
  .seg {
    margin: 12px 16px 0;
  }
  .group {
    margin: 16px 16px 0;
    border-radius: var(--radius-group);
    background: var(--card);
    box-shadow: var(--shadow-card);
  }
  .item {
    position: relative;
    background: var(--card);
    border-radius: var(--radius-group);
  }
  .sorting > .item {
    transition: transform 0.25s cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .sorting > .item.lifted {
    z-index: 2;
    transition: none;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
  }
  .item + .item::before {
    content: "";
    position: absolute;
    z-index: 1;
    left: 58px;
    right: 0;
    top: 0;
    height: 0.5px;
    background: var(--sep);
  }
  .row {
    display: flex;
    align-items: center;
    -webkit-user-select: none;
    user-select: none;
    -webkit-touch-callout: none;
  }
  .head {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 54px;
    padding: 8px 4px 8px 12px;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    font-size: 16px;
    text-align: left;
  }
  .mark {
    flex: none;
    width: 34px;
    height: 34px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    font-size: 20px;
    background: color-mix(in srgb, var(--mc) 16%, var(--card));
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
  .count {
    flex: none;
    font-size: 14px;
    color: var(--label2);
  }
  .chev {
    display: flex;
    color: var(--label3);
    transition: transform 0.2s;
  }
  .chev.up {
    transform: rotate(180deg);
  }
  .pencil {
    flex: none;
    display: grid;
    place-items: center;
    width: 44px;
    height: 54px;
    border: 0;
    background: none;
    color: var(--tint);
  }
  /* 按下整行变底色，和其他列表一致；不改透明度，免得点开时一闪。拖动排序时不显示。 */
  .group:not(.sorting) .row:has(.head:active) {
    background: var(--press);
  }
  .item:first-child > .row {
    border-radius: var(--radius-group) var(--radius-group) 0 0;
  }
  .item:last-child > .row:last-child {
    border-bottom-left-radius: var(--radius-group);
    border-bottom-right-radius: var(--radius-group);
  }
  .pencil:active {
    opacity: 0.5;
  }
  .subs {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 2px 12px 14px 58px;
  }
  .cloud {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 14px;
  }
  .chip {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 34px;
    padding: 0 12px;
    border: 0;
    border-radius: 12px;
    background: color-mix(in srgb, var(--mc, var(--tint)) 12%, var(--card));
    color: var(--label);
    font: inherit;
    font-size: 15px;
    -webkit-user-select: none;
    user-select: none;
    -webkit-touch-callout: none;
  }
  .chip:active {
    opacity: 0.6;
  }
  .chip.add {
    background: none;
    box-shadow: inset 0 0 0 1px var(--sep);
    color: var(--tint);
  }
  .subs.sorting .chip.lifted {
    z-index: 2;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.2);
    background: var(--card);
  }
  /* 松手会插到这个小块前面或后面。 */
  .chip.before::before,
  .chip.after::after {
    content: "";
    position: absolute;
    top: 4px;
    bottom: 4px;
    width: 3px;
    border-radius: 2px;
    background: var(--tint);
  }
  .chip.before::before {
    left: -6px;
  }
  .chip.after::after {
    right: -6px;
  }
  .addtop {
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
  .addtop:active {
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
