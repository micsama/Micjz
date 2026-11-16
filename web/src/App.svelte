<script lang="ts">
  // 外壳：标签页 + 推入的详情 + 共用面板。哪个页面长什么样由当前主题决定。
  import { app, back, closeSheet, find } from "./lib/store.svelte";
  import { themeState } from "./lib/theme.svelte";
  import { theme } from "./themes";
  import { nav } from "./ui/nav.svelte";
  import Pushed from "./ui/Pushed.svelte";
  import Sheet from "./ui/Sheet.svelte";
  import FieldEditor from "./ui/FieldEditor.svelte";
  import Toast from "./ui/Toast.svelte";
  import ActionSheet from "./ui/ActionSheet.svelte";

  const t = $derived(theme(themeState.id));
  const depth = $derived(app.stack.length);
  const sheetEntry = $derived(app.sheet ? find(app.sheet.id) : undefined);

  $effect.pre(() => {
    document.documentElement.dataset.theme = t.id;
  });
</script>

<div
  class="base"
  class:dragging={nav.dragging}
  style:transform={depth ? `translateX(${-30 * (1 - nav.drag)}%)` : ""}
  style:--dim={depth ? 1 - nav.drag : 0}
>
  {#if app.tab === "list"}<t.List />{:else if app.tab === "wealth"}<t.Wealth />{:else}<t.Settings />{/if}
  <t.TabBar />
</div>

{#each app.stack as page, i (page.kind === "entry" ? page.id : `${page.kind}-${i}`)}
  {#if page.kind === "stats"}
    <Pushed onback={back}>{#key t.id}<t.Stats />{/key}</Pushed>
  {:else if page.kind === "account"}
    <Pushed onback={back}>{#key t.id}<t.List account={page.name} />{/key}</Pushed>
  {:else}
    {@const entry = find(page.id)}
    {#if entry}
      <Pushed onback={back}>{#key t.id}<t.Detail {entry} />{/key}</Pushed>
    {/if}
  {/if}
{/each}

<Sheet open={!!sheetEntry} onclose={closeSheet}>
  {#if app.sheet && sheetEntry}
    {#key app.sheet.field}<FieldEditor entry={sheetEntry} key={app.sheet.field} />{/key}
  {/if}
</Sheet>

<ActionSheet open={!!app.menu} title={app.menu?.title ?? ""} actions={app.menu?.actions ?? []} oncancel={() => (app.menu = null)} />

<Toast />

<style>
  .base {
    position: absolute;
    inset: 0;
    transition: transform 0.46s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .base.dragging {
    transition: none;
  }
  .base::after {
    content: "";
    position: absolute;
    inset: 0;
    background: #000;
    opacity: calc(var(--dim) * 0.12);
    pointer-events: none;
    transition: opacity 0.46s;
    z-index: 30;
  }
</style>
