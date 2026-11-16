<script lang="ts">
  import { app, accountOf, categoryOf, balanceOf, clearFilter, setFilter, filtering, type Dim } from "../lib/store.svelte";
  import Icon from "./Icon.svelte";
  import { money } from "../lib/format";
  import { cardLabel } from "../lib/look";
  import AccountChoices from "./AccountChoices.svelte";
  import TagChoices from "./TagChoices.svelte";
  import CategoryChoices from "./CategoryChoices.svelte";
  import ChoicePopover from "./ChoicePopover.svelte";
  import AccountLogo from "./AccountLogo.svelte";
  const dims: { id: Dim; label: string }[] = [
    { id: "account", label: "账户" },
    { id: "category", label: "分类" },
    { id: "tag", label: "标签" },
  ];
  const account = $derived(accountOf(app.filter.account));
  const selected = (d: Dim) =>
    d === "account" ? account?.name : d === "category" ? categoryOf(app.filter.category)?.name : app.catalog.tags.find((t) => t.id === app.filter.tag)?.name;
</script>

<div class="filters">
  {#each dims as d (d.id)}<button class:on={app.filter[d.id] !== null} onclick={() => (app.picking = d.id)}
      >{selected(d.id) ?? d.label}<Icon name="chevron-down" size={12} weight={2.4} /></button
    >{/each}
  {#if filtering()}<button onclick={clearFilter}>清除</button>{/if}
  <ChoicePopover
    open={app.picking !== null}
    title={`筛选${dims.find((d) => d.id === app.picking)?.label ?? ""}`}
    onclose={() => (app.picking = null)}
    onclear={() => {
      if (app.picking) setFilter(app.picking, null);
    }}
  >
    <div class="options">
      {#if app.picking === "account"}
        <AccountChoices selected={app.filter.account} onselect={(id) => setFilter("account", id)} />
      {:else if app.picking === "tag"}
        <TagChoices selected={app.filter.tag === null ? [] : [app.filter.tag]} onselect={(id) => setFilter("tag", id)} />
      {:else if app.picking === "category"}
        <CategoryChoices selected={app.filter.category} onselect={(id, final) => setFilter("category", id, !final)} />
      {/if}
    </div>
  </ChoicePopover>
</div>
{#if account}
  <section class="account">
    <AccountLogo {account} size={44} />
    <span><b>{account.name}</b><small>{account.card ? cardLabel(account.card) : "无卡号"}</small></span>
    <span class="balance"><small>当前余额</small><b class="num">{app.hideBalance ? "••••" : money(balanceOf(account.id))}</b></span>
  </section>
{/if}

<style>
  .filters {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 12px 16px;
  }
  button {
    display: flex;
    align-items: center;
    gap: 4px;
    border: 0;
    border-radius: 18px;
    background: var(--fill);
    color: var(--label);
    font: inherit;
    padding: 8px 12px;
  }
  button.on {
    color: var(--tint);
    background: color-mix(in srgb, var(--tint) 15%, var(--card));
  }
  .account {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 12px 16px;
    padding: 16px;
    background: var(--card);
    border-radius: var(--radius-group);
  }
  .account span {
    display: grid;
    min-width: 0;
    overflow-wrap: anywhere;
    gap: 4px;
  }
  small {
    color: var(--label2);
    font-size: 12px;
  }
  .balance {
    margin-left: auto;
    text-align: right;
  }
  .options {
    display: grid;
    gap: 8px;
  }
</style>
