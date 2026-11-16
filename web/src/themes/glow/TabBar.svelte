<script lang="ts">
  import Icon from "../../ui/Icon.svelte";
  import { app, type Tab } from "../../lib/store.svelte";
  const tabs: { id: Tab; label: string; icon: "list" | "wallet" | "gear" }[] = [
    { id: "list", label: "账目", icon: "list" },
    { id: "wealth", label: "财富", icon: "wallet" },
    { id: "settings", label: "设置", icon: "gear" },
  ];
</script>

<nav>
  {#each tabs as t (t.id)}
    <button class:on={app.tab === t.id} onclick={() => (app.tab = t.id)}>
      <Icon name={t.icon} size={25} weight={app.tab === t.id ? 2.3 : 1.8} />
      <span>{t.label}</span>
    </button>
  {/each}
</nav>

<style>
  nav {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 20;
    display: flex;
    justify-content: space-around;
    padding: 6px 16px max(env(safe-area-inset-bottom), 10px);
    background: var(--bar-bg);
    backdrop-filter: saturate(180%) blur(24px);
    -webkit-backdrop-filter: saturate(180%) blur(24px);
    box-shadow: 0 -0.5px 0 var(--sep);
  }
  button {
    display: grid;
    justify-items: center;
    gap: 2px;
    min-width: 72px;
    border: 0;
    background: none;
    color: var(--label3);
    font: inherit;
    font-size: 10px;
    font-weight: 500;
    transition: color 0.2s;
  }
  button.on {
    color: var(--tint);
  }
  button:active {
    transform: scale(0.92);
  }
</style>
