<script lang="ts" generics="T extends string">
  // 分段切换：几个互斥选项并排，当前项浮起。记一笔的账目类型、筛选分类的支出/收入共用。
  let { label, options, value, onchange }: { label: string; options: readonly (readonly [T, string])[]; value: T; onchange: (v: T) => void } = $props();
</script>

<div class="segmented" role="group" aria-label={label}>
  {#each options as [v, text] (v)}
    <button class:on={value === v} aria-pressed={value === v} onclick={() => onchange(v)}>{text}</button>
  {/each}
</div>

<style>
  .segmented {
    display: flex;
    border-radius: 12px;
    padding: 3px;
    background: var(--fill);
  }
  button {
    flex: 1;
    min-height: 36px;
    padding: 4px 8px;
    border: 0;
    border-radius: 9px;
    background: none;
    color: var(--label2);
    font: inherit;
    font-size: 15px;
    cursor: pointer;
    -webkit-tap-highlight-color: transparent;
  }
  button.on {
    background: var(--card);
    color: var(--tint);
    font-weight: 650;
    box-shadow: 0 2px 5px #0000000d;
  }
</style>
