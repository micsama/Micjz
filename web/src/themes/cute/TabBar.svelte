<script lang="ts">
  import Icon from "../../ui/Icon.svelte";
  import { app, selectTab, type Tab } from "../../lib/store.svelte";
  const tabs: ({ id: Tab; label: string; emoji: string } | null)[] = [
    { id: "list", label: "账本", emoji: "📒" },
    { id: "stats", label: "统计", emoji: "📊" },
    null,
    { id: "wealth", label: "财富", emoji: "💰" },
    { id: "settings", label: "设置", emoji: "🎀" },
  ];
  const at = $derived(tabs.findIndex((t) => t?.id === app.tab));
</script>

<nav style:--at={at}>
  <span class="blob"></span>
  {#each tabs as t, i (t?.id ?? i)}
    {#if t}
      <button class:on={app.tab === t.id} onclick={(e) => selectTab(t.id, e.currentTarget)}>
        <span class="e">{t.emoji}</span>{t.label}
      </button>
    {:else}<button class="add" onclick={() => (app.creating = true)} aria-label="记一笔"><Icon name="plus" size={26} weight={2.6} /></button>{/if}
  {/each}
</nav>

<style>
  /* 加号比栏高，往上凸出一截。 */
  .add {
    width: 52px;
    height: 52px;
    margin-top: -18px;
    align-self: center;
    justify-self: center;
    border-radius: 50%;
    background: var(--tint);
    color: white;
    font-size: 28px;
    box-shadow: var(--shadow-card);
    display: grid;
    place-items: center;
  }
  nav {
    position: absolute;
    left: 50%;
    bottom: max(env(safe-area-inset-bottom), 14px);
    translate: -50% 0;
    z-index: 20;
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    width: min(360px, calc(100% - 32px));
    padding: 4px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--card) 88%, transparent);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    box-shadow:
      0 12px 30px -8px rgba(255, 111, 156, 0.35),
      0 0 0 1px rgba(255, 143, 177, 0.15);
  }
  .blob {
    position: absolute;
    top: 4px;
    bottom: 4px;
    left: 4px;
    width: calc((100% - 8px) / 5);
    border-radius: 999px;
    background: linear-gradient(135deg, #ffa6c1, var(--tint));
    transform: translateX(calc(var(--at) * 100%));
    transition: transform 0.55s cubic-bezier(0.34, 1.56, 0.64, 1);
    box-shadow: 0 6px 14px -4px rgba(255, 111, 156, 0.6);
  }
  button {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-direction: column;
    gap: 0;
    height: 40px;
    border: 0;
    background: none;
    color: var(--label2);
    font: inherit;
    font-size: 10px;
    font-weight: 700;
    transition:
      color 0.3s,
      transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  button.on {
    color: #fff;
  }
  button:active {
    transform: scale(0.9);
  }
  .e {
    font-size: 16px;
    transition: transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  button.on .e {
    transform: rotate(-10deg) scale(1.15);
  }
</style>
