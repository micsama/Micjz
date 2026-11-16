<script lang="ts">
  // 进页面之前的一屏：取数据中、登录（输入 config.toml 里的 token）、连不上时重试。
  import { onMount, tick } from "svelte";
  import { app, boot, login } from "../lib/store.svelte";

  let token = $state("");
  let error = $state("");
  let busy = $state(false);
  let input = $state<HTMLInputElement>();

  onMount(async () => {
    await tick();
    input?.focus();
  });

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!token.trim() || busy) return;
    busy = true;
    error = (await login(token.trim())) ?? "";
    busy = false;
  }
</script>

<main>
  <div class="mark">¥</div>
  {#if app.phase === "login"}
    <h1>登录记账</h1>
    <p>输入服务器 config.toml 里的 token，这台设备一年内不用再登。</p>
    <form onsubmit={submit}>
      <input
        bind:this={input}
        bind:value={token}
        type="password"
        autocomplete="current-password"
        placeholder="token"
        aria-label="token"
        class:bad={error}
        oninput={() => (error = "")}
      />
      {#if error}<p class="err">{error}</p>{/if}
      <button disabled={!token.trim() || busy}>{busy ? "登录中…" : "登录"}</button>
    </form>
  {:else if app.phase === "failed"}
    <h1>没取到数据</h1>
    <p>{app.failure}</p>
    <form
      onsubmit={(e) => {
        e.preventDefault();
        void boot();
      }}
    >
      <button>重试</button>
    </form>
  {:else}
    <p class="wait"><span class="spin"></span>加载中</p>
  {/if}
</main>

<style>
  main {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 0 32px 18vh;
    text-align: center;
  }
  .mark {
    display: grid;
    place-items: center;
    width: 64px;
    height: 64px;
    margin-bottom: 14px;
    border-radius: 18px;
    background: var(--hero-bg);
    color: var(--hero-fg);
    font-size: 32px;
    font-weight: 700;
    box-shadow: var(--shadow-card);
  }
  h1 {
    margin: 0;
    font-size: 24px;
    font-weight: 700;
  }
  p {
    margin: 0;
    color: var(--label2);
    font-size: 15px;
  }
  form {
    display: grid;
    gap: 10px;
    width: 100%;
    max-width: 320px;
    margin-top: 22px;
  }
  input {
    height: 48px;
    padding: 0 16px;
    border: 0;
    border-radius: 12px;
    background: var(--card);
    color: var(--label);
    font: inherit;
    text-align: center;
    box-shadow:
      var(--shadow-card),
      0 0 0 1.5px transparent;
    outline: none;
    transition: box-shadow 0.2s;
  }
  input:focus {
    box-shadow:
      var(--shadow-card),
      0 0 0 1.5px var(--tint);
  }
  input.bad {
    box-shadow:
      var(--shadow-card),
      0 0 0 1.5px var(--red);
  }
  .err {
    color: var(--red);
    font-size: 14px;
  }
  button {
    height: 50px;
    border: 0;
    border-radius: 14px;
    background: var(--tint);
    color: #fff;
    font: inherit;
    font-weight: 600;
    transition: opacity 0.2s;
  }
  button:disabled {
    opacity: 0.4;
  }
  .wait {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .spin {
    width: 14px;
    height: 14px;
    border: 2px solid var(--label3);
    border-top-color: var(--label2);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
