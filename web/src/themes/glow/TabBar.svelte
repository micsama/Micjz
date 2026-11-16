<script lang="ts">
  // 悬浮的玻璃胶囊，选中项下面垫一块会滑动的玻璃，配合 iOS 的液态玻璃风格。正中间是「记一笔」，不是标签。
  import Icon, { type IconName } from "../../ui/Icon.svelte";
  import { app, selectTab, type Tab } from "../../lib/store.svelte";
  type Slot = { id: Tab; label: string; icon: IconName } | null;
  const slots: Slot[] = [
    { id: "list", label: "账目", icon: "list" },
    { id: "stats", label: "统计", icon: "chart" },
    null,
    { id: "wealth", label: "财富", icon: "wallet" },
    { id: "settings", label: "设置", icon: "gear" },
  ];
  const at = $derived(slots.findIndex((t) => t?.id === app.tab));
</script>

<nav style:--at={at} style:--n={slots.length}>
  <span class="pill"></span>
  {#each slots as t, i (t?.id ?? i)}
    {#if t}
      <button class:on={app.tab === t.id} onclick={(e) => selectTab(t.id, e.currentTarget)}>
        <Icon name={t.icon} size={23} weight={app.tab === t.id ? 2.3 : 1.9} />
        <span>{t.label}</span>
      </button>
    {:else}
      <button class="add" aria-label="记一笔" onclick={() => (app.creating = true)}>
        <span class="plus"><Icon name="plus" size={22} weight={2.6} /></span>
      </button>
    {/if}
  {/each}
</nav>

<style>
  nav {
    position: absolute;
    left: 50%;
    bottom: max(env(safe-area-inset-bottom), 14px);
    translate: -50% 0;
    z-index: 20;
    display: grid;
    grid-template-columns: repeat(var(--n), 1fr);
    width: min(380px, calc(100% - 24px));
    padding: 5px;
    border-radius: 999px;
    background: var(--glass-bg);
    backdrop-filter: blur(24px) saturate(190%);
    -webkit-backdrop-filter: blur(24px) saturate(190%);
    box-shadow:
      var(--glass-shadow),
      inset 0 0 0 0.5px var(--glass-edge),
      inset 0 1px 0 var(--glass-edge);
  }
  .pill {
    position: absolute;
    top: 5px;
    bottom: 5px;
    left: 5px;
    width: calc((100% - 10px) / var(--n));
    border-radius: 999px;
    background: var(--glass-pill);
    box-shadow: inset 0 0.5px 0 var(--glass-edge);
    transform: translateX(calc(var(--at) * 100%));
    transition: transform 0.5s cubic-bezier(0.34, 1.4, 0.64, 1);
  }
  button {
    position: relative;
    display: grid;
    justify-items: center;
    align-content: center;
    gap: 1px;
    height: 52px;
    border: 0;
    background: none;
    color: var(--label);
    font: inherit;
    font-size: 10px;
    font-weight: 600;
    transition:
      color 0.25s,
      transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  button.on {
    color: var(--tint);
  }
  button:active {
    transform: scale(0.9);
  }
  .plus {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 50%;
    color: #fff;
    background: linear-gradient(145deg, color-mix(in oklab, var(--tint) 60%, white), var(--tint));
    box-shadow:
      0 6px 14px -4px color-mix(in oklab, var(--tint) 70%, transparent),
      inset 0 1px 0 rgba(255, 255, 255, 0.45);
  }
</style>
