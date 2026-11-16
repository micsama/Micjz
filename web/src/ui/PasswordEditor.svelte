<script lang="ts">
  // 设置里改登录密码：改完其他设备都要重新登录，这台不用。
  import { onMount, tick } from "svelte";
  import SheetHead from "./SheetHead.svelte";
  import { changePassword, toast } from "../lib/store.svelte";

  let { onclose }: { onclose: () => void } = $props();

  let current = $state("");
  let next = $state("");
  let again = $state("");
  let error = $state("");
  let busy = $state(false);
  let first = $state<HTMLInputElement>();

  onMount(async () => {
    await tick();
    first?.focus();
  });

  async function submit() {
    if (busy) return;
    if (!current || !next) return void (error = "当前密码和新密码都要填");
    if ([...next].length < 8) return void (error = "新密码至少 8 位");
    if (next !== again) return void (error = "两次输入的新密码不一样");
    busy = true;
    const err = await changePassword(current, next);
    busy = false;
    if (err) return void (error = err);
    toast("密码已修改，其他设备需要重新登录");
    onclose();
  }
</script>

<SheetHead icon="lock" title="修改密码" {onclose} action={{ label: "完成", run: submit }} />

<form
  class="body"
  onsubmit={(e) => {
    e.preventDefault();
    void submit();
  }}
>
  <!-- 钥匙串按用户名存密码，改完能顺手更新那一条。 -->
  <input class="user" autocomplete="username" value="dzmfg" readonly tabindex="-1" aria-hidden="true" />
  <div class="fields">
    <input bind:this={first} bind:value={current} type="password" autocomplete="current-password" placeholder="当前密码" oninput={() => (error = "")} />
    <input bind:value={next} type="password" autocomplete="new-password" placeholder="新密码，至少 8 位" oninput={() => (error = "")} />
    <input bind:value={again} type="password" autocomplete="new-password" placeholder="再输一次新密码" oninput={() => (error = "")} />
  </div>
  {#if error}<p class="err" role="alert">{error}</p>{:else}<p class="hint">改完其他设备都要重新登录，这台不用。</p>{/if}
  <button hidden aria-hidden="true"></button>
</form>

<style>
  .body {
    padding: 4px 16px 12px;
  }
  .user {
    position: absolute;
    width: 0;
    height: 0;
    opacity: 0;
    pointer-events: none;
  }
  .fields {
    display: grid;
    border-radius: var(--radius-group);
    overflow: hidden;
    background: var(--card);
  }
  input {
    height: 48px;
    padding: 0 16px;
    border: 0;
    background: none;
    color: var(--label);
    font: inherit;
    font-size: 16px;
    outline: none;
  }
  .fields input + input {
    border-top: 0.5px solid var(--sep);
  }
  .hint,
  .err {
    margin: 10px 4px 0;
    text-align: center;
    font-size: 13px;
    color: var(--label2);
  }
  .err {
    color: var(--red);
  }
</style>
