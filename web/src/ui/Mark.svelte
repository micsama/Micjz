<script lang="ts">
  // 账目图标：分类名开头的 emoji（或首字）原样放大，其余用线条图标。大小跟随外层字号。
  import Icon from "./Icon.svelte";
  import type { Mark } from "../lib/view";

  let { mark }: { mark: Mark } = $props();
</script>

{#if "icon" in mark}<Icon name={mark.icon} size="0.9em" weight={2.2} />{:else}{mark.text}{/if}
