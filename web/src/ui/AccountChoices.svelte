<script lang="ts">
  import { app } from "../lib/store.svelte";
  import { cardLabel } from "../lib/look";
  import AccountLogo from "./AccountLogo.svelte";
  import Icon from "./Icon.svelte";
  let { selected, exclude = null, onselect }: { selected: number | null; exclude?: number | null; onselect: (id: number) => void } = $props();
</script>

<div class="accounts">
  {#each app.catalog.accounts.filter((a) => a.id !== exclude) as a (a.id)}
    <button class:on={selected === a.id} aria-pressed={selected === a.id} onclick={() => onselect(a.id)}
      ><AccountLogo account={a} size={28} /><span
        >{a.name}{#if a.card}<small>{cardLabel(a.card)}</small>{/if}</span
      >{#if selected === a.id}<Icon name="check" size={17} />{/if}</button
    >
  {/each}
</div>

<style>
  .accounts {
    display: grid;
    gap: 4px;
  }
  button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 48px;
    padding: 8px 12px;
    border: 0;
    border-radius: 12px;
    background: var(--card);
    color: var(--label);
    font: inherit;
    text-align: left;
  }
  button.on {
    color: var(--tint);
    background: color-mix(in srgb, var(--tint) 12%, var(--card));
  }
  span {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  small {
    display: block;
    margin-top: 2px;
    font-size: 12px;
    color: var(--label2);
  }
</style>
