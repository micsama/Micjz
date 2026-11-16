// 长按一行拿起，上下拖到新位置松手。挂在列表容器上，容器的直接子元素就是行，行高相同。
// 手指不动按住才拿起，所以和上下滚动、左滑互不干扰。

const HOLD = 300;
const SLOP = 8;

export class Reorder {
  /** 拿起的行；-1 表示没在拖。 */
  from = $state(-1);
  /** 松手会落到的位置。 */
  to = $state(-1);
  dy = $state(0);
  #h = 0;
  #commit: (from: number, to: number) => void;

  constructor(commit: (from: number, to: number) => void) {
    this.#commit = commit;
  }

  /** 第 i 行此刻的纵向位移：拿起的跟手，被越过的让位。 */
  shift(i: number): number {
    const { from, to } = this;
    if (from < 0) return 0;
    if (i === from) return this.dy;
    if (from < i && i <= to) return -this.#h;
    if (to <= i && i < from) return this.#h;
    return 0;
  }

  attach = (node: HTMLElement) => {
    let id = -1;
    let x0 = 0;
    let y0 = 0;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let swallow = false;

    const down = (e: PointerEvent) => {
      swallow = false;
      const row = [...node.children].find((c) => c.contains(e.target as Node)) as HTMLElement | undefined;
      if (!row || e.button !== 0) return;
      id = e.pointerId;
      x0 = e.clientX;
      y0 = e.clientY;
      timer = setTimeout(() => {
        this.#h = row.offsetHeight;
        this.from = this.to = [...node.children].indexOf(row);
        this.dy = 0;
        node.setPointerCapture(id);
      }, HOLD);
    };

    const move = (e: PointerEvent) => {
      if (e.pointerId !== id) return;
      if (this.from < 0) {
        if (Math.hypot(e.clientX - x0, e.clientY - y0) > SLOP) clearTimeout(timer);
        return;
      }
      const last = node.children.length - 1;
      const h = this.#h;
      this.dy = Math.max(-this.from * h, Math.min((last - this.from) * h, e.clientY - y0));
      this.to = Math.round(this.from + this.dy / h);
    };

    const up = (e: PointerEvent) => {
      if (e.pointerId !== id) return;
      clearTimeout(timer);
      id = -1;
      if (this.from < 0) return;
      const { from, to } = this;
      this.from = this.to = -1;
      this.dy = 0;
      // 拿起过就不算点击，免得松手时打开这一行。
      swallow = true;
      if (from !== to) this.#commit(from, to);
    };

    const click = (e: MouseEvent) => {
      if (!swallow) return;
      swallow = false;
      e.stopPropagation();
      e.preventDefault();
    };

    // 拿起后不让页面跟着滚；必须是非被动监听才能拦。
    const touchmove = (e: TouchEvent) => {
      if (this.from >= 0) e.preventDefault();
    };

    node.addEventListener("pointerdown", down);
    node.addEventListener("pointermove", move);
    node.addEventListener("pointerup", up);
    node.addEventListener("pointercancel", up);
    node.addEventListener("click", click, true);
    node.addEventListener("touchmove", touchmove, { passive: false });
    return () => {
      clearTimeout(timer);
      node.removeEventListener("pointerdown", down);
      node.removeEventListener("pointermove", move);
      node.removeEventListener("pointerup", up);
      node.removeEventListener("pointercancel", up);
      node.removeEventListener("click", click, true);
      node.removeEventListener("touchmove", touchmove);
    };
  };
}
