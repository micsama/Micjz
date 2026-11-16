import { backOut, quintOut } from "svelte/easing";

/** 详情页边缘右滑返回时的进度，底下的页面跟着视差移动。 */
export const nav = $state({ drag: 0, dragging: false });

/** 主题用 `--motion: bouncy` 选弹性动效，默认平滑（iOS 系统曲线）。 */
export function motion() {
  const bouncy = getComputedStyle(document.documentElement).getPropertyValue("--motion").trim() === "bouncy";
  const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
  return { easing: bouncy ? backOut : quintOut, reduce };
}

/** iOS 只在用户手势里同步 focus 才弹键盘：先聚焦这个占位框，面板渲染后再移到真正的输入框。 */
export function holdKeyboard(mode: "text" | "decimal" | "numeric" = "text") {
  let proxy = document.getElementById("kbd-proxy") as HTMLInputElement | null;
  if (!proxy) {
    proxy = document.createElement("input");
    proxy.id = "kbd-proxy";
    proxy.setAttribute("aria-hidden", "true");
    proxy.style.cssText = "position:fixed;top:0;left:0;opacity:0;height:0;width:0;font-size:16px;pointer-events:none";
    document.body.append(proxy);
  }
  // 占位框和真输入框键盘类型一致，iOS 才直接弹数字键盘。
  proxy.inputMode = mode;
  proxy.focus();
}
