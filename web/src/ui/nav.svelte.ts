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

/** 文字键盘出现时，让页面或面板的底边跟随可视区域。 */
export function keyboardInset(node: HTMLElement) {
  const viewport = window.visualViewport;
  if (!viewport) return;
  const sync = () => {
    const gap = window.innerHeight - viewport.height - viewport.offsetTop;
    node.style.setProperty("--kb", `${gap > 80 ? gap : 0}px`);
    // 可视区域变矮后，正在输入的框可能被挡住，滚回视野。
    const active = document.activeElement;
    if (active instanceof HTMLElement && node.contains(active)) requestAnimationFrame(() => active.scrollIntoView({ block: "nearest" }));
  };
  sync();
  viewport.addEventListener("resize", sync);
  viewport.addEventListener("scroll", sync);
  return {
    destroy() {
      viewport.removeEventListener("resize", sync);
      viewport.removeEventListener("scroll", sync);
    },
  };
}
