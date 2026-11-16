<script lang="ts">
  // 外壳：标签页 + 推入的详情 + 共用面板。哪个页面长什么样由当前主题决定。
  import { app, back, closeSheet, find } from "./lib/store.svelte";
  import { themeState } from "./lib/theme.svelte";
  import { theme } from "./themes";
  import { nav } from "./ui/nav.svelte";
  import Pushed from "./ui/Pushed.svelte";
  import Sheet from "./ui/Sheet.svelte";
  import FieldEditor from "./ui/FieldEditor.svelte";
  import FilterPicker from "./ui/FilterPicker.svelte";
  import NewEntry from "./ui/NewEntry.svelte";
  import Toast from "./ui/Toast.svelte";

  const t = $derived(theme(themeState.id));
  const depth = $derived(app.stack.length);
  const sheetEntry = $derived(app.sheet ? find(app.sheet.id) : undefined);

  $effect.pre(() => {
    document.documentElement.dataset.theme = t.id;
  });
</script>

<div
  class="base"
  inert={app.creating}
  class:dragging={nav.dragging}
  style:transform={depth ? `translateX(${-30 * (1 - nav.drag)}%)` : ""}
  style:--dim={depth ? 1 - nav.drag : 0}
>
  {#if app.tab === "list"}<t.List />{:else if app.tab === "stats"}<t.Stats />{:else if app.tab === "wealth"}<t.Wealth />{:else}<t.Settings />{/if}
  <t.TabBar />
</div>

{#each app.stack as page (page.id)}
  {@const entry = find(page.id)}
  {#if entry}
    <Pushed onback={back}
      >{#key t.id}<t.Detail {entry} />{/key}</Pushed
    >
  {/if}
{/each}

<Sheet open={!!sheetEntry} onclose={closeSheet}>
  {#if app.sheet && sheetEntry}
    {#key app.sheet.field}<FieldEditor entry={sheetEntry} key={app.sheet.field} />{/key}
  {/if}
</Sheet>

<Sheet open={app.picking !== null} onclose={() => (app.picking = null)}>
  {#if app.picking}
    {#key app.picking}<FilterPicker dim={app.picking} />{/key}
  {/if}
</Sheet>

{#if app.creating}<NewEntry />{/if}

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
