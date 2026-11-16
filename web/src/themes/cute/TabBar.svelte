<script lang="ts">
  import { app, type Tab } from "../../lib/store.svelte";
  const tabs: { id: Tab; label: string; emoji: string }[] = [
    { id: "list", label: "账本", emoji: "📒" },
    { id: "stats", label: "统计", emoji: "📊" },
    { id: "wealth", label: "财富", emoji: "💰" },
    { id: "settings", label: "设置", emoji: "🎀" },
  ];
  const at = $derived(tabs.findIndex((t) => t.id === app.tab));
</script>

<nav style:--at={at}>
  <span class="blob"></span>
  {#each tabs as t (t.id)}
    <button class:on={app.tab === t.id} onclick={() => (app.tab = t.id)}>
      <span class="e">{t.emoji}</span>{t.label}
    </button>
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
    grid-template-columns: repeat(4, 1fr);
    width: min(360px, calc(100% - 32px));
    padding: 6px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--card) 88%, transparent);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    box-shadow: 0 12px 30px -8px rgba(255, 111, 156, 0.35), 0 0 0 1px rgba(255, 143, 177, 0.15);
  }
  .blob {
    position: absolute;
    top: 6px;
    bottom: 6px;
    left: 6px;
    width: calc((100% - 12px) / 4);
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
    gap: 6px;
    height: 46px;
    border: 0;
    background: none;
    color: var(--label2);
    font: inherit;
    font-size: 15px;
    font-weight: 700;
    transition: color 0.3s, transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  button.on {
    color: #fff;
  }
  button:active {
    transform: scale(0.9);
  }
  .e {
    font-size: 19px;
    transition: transform 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  button.on .e {
    transform: rotate(-10deg) scale(1.15);
  }
</style>
