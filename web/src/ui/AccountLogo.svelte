<script lang="ts">
  import { brand } from "../lib/look";
  import type { Account } from "../lib/types";

  let { account, size = 40 }: { account: Pick<Account, "name" | "brand">; size?: number } = $props();
  const b = $derived(brand(account));
</script>

<span class="logo" style:width="{size}px" style:height="{size}px" style:background={b.bg}>
  {#if b.path}
    <svg width={size * 0.62} height={size * 0.62} viewBox="0 0 24 24" fill="#fff" fill-rule="evenodd" aria-hidden="true"><path d={b.path} /></svg>
  {:else}
    <span style:font-size="{size * 0.43}px">{b.mark}</span>
  {/if}
</span>

<style>
  .logo {
    flex: none;
    display: inline-grid;
    place-items: center;
    border-radius: 50%;
    color: #fff;
    font-weight: 700;
    box-shadow: inset 0 -2px 0 rgba(0, 0, 0, 0.12);
  }
</style>
