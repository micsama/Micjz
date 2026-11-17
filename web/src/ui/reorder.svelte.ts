// 长按拿起、拖到新位置松手。手指不动按住才拿起，所以和滚动、点击互不干扰。
// `Reorder` 挂在竖排列表上，容器的直接子元素就是行（行高可以不同）；
// `ChipReorder` 挂在换行排列的小块上，只认带 `data-drag` 的直接子元素。

const HOLD = 300;
const SLOP = 8;

type Lift = {
  /** 按下的是哪一项；不在任何项上返回 -1。 */
  hit: (e: PointerEvent) => number;
  lift: (i: number, e: PointerEvent) => void;
  drag: (e: PointerEvent) => void;
  /** 松手；返回是否拿起过。 */
  drop: () => boolean;
  lifted: () => boolean;
};

function longPress(node: HTMLElement, h: Lift, own = false) {
  let id = -1;
  let x0 = 0;
  let y0 = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let swallow = false;

  const down = (e: PointerEvent) => {
    // 嵌在另一个可拖列表里时，不让外层也拿起。
    if (own) e.stopPropagation();
    swallow = false;
    const i = h.hit(e);
    if (i < 0 || e.button !== 0) return;
    id = e.pointerId;
    x0 = e.clientX;
    y0 = e.clientY;
    timer = setTimeout(() => {
      h.lift(i, e);
      node.setPointerCapture(id);
    }, HOLD);
  };
  const move = (e: PointerEvent) => {
    if (e.pointerId !== id) return;
    if (!h.lifted()) {
      if (Math.hypot(e.clientX - x0, e.clientY - y0) > SLOP) clearTimeout(timer);
      return;
    }
    h.drag(e);
  };
  const up = (e: PointerEvent) => {
    if (e.pointerId !== id) return;
    clearTimeout(timer);
    id = -1;
    // 拿起过就不算点击，免得松手时打开这一项。
    swallow = h.drop();
  };
  const click = (e: MouseEvent) => {
    if (!swallow) return;
    swallow = false;
    e.stopPropagation();
    e.preventDefault();
  };
  // 拿起后不让页面跟着滚；必须是非被动监听才能拦。
  const touchmove = (e: TouchEvent) => {
    if (h.lifted()) e.preventDefault();
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
}

export class Reorder {
  /** 拿起的行；-1 表示没在拖。 */
  from = $state(-1);
  /** 松手会落到的位置。 */
  to = $state(-1);
  dy = $state(0);
  /** 拿起行的高度，被越过的行让出这么多。 */
  #h = 0;
  /** 拿起时各行的上下沿，相对容器。 */
  #rows: { top: number; mid: number; bottom: number }[] = [];
  #y0 = 0;
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

  attach = (node: HTMLElement) =>
    longPress(node, {
      hit: (e) => [...node.children].findIndex((c) => c.contains(e.target as Node)),
      lift: (i, e) => {
        this.#rows = [...node.children].map((c) => {
          const { offsetTop: top, offsetHeight: h } = c as HTMLElement;
          return { top, mid: top + h / 2, bottom: top + h };
        });
        const r = this.#rows[i];
        this.#h = r.bottom - r.top;
        this.#y0 = e.clientY;
        this.from = this.to = i;
        this.dy = 0;
      },
      drag: (e) => {
        const rows = this.#rows;
        const r = rows[this.from];
        const dy = Math.max(rows[0].top - r.top, Math.min(rows[rows.length - 1].bottom - r.bottom, e.clientY - this.#y0));
        // 拿起行的下沿越过下面某行的中线、上沿越过上面某行的中线，就和它换位。
        let to = this.from;
        rows.forEach((o, i) => {
          if (i > this.from && r.bottom + dy > o.mid) to = i;
          if (i < this.from && r.top + dy < o.mid) to = Math.min(to, i);
        });
        this.dy = dy;
        this.to = to;
      },
      drop: () => {
        const { from, to } = this;
        if (from < 0) return false;
        this.from = this.to = -1;
        this.dy = 0;
        if (from !== to) this.#commit(from, to);
        return true;
      },
      lifted: () => this.from >= 0,
    });
}

export class ChipReorder {
  /** 拿起的小块；-1 表示没在拖。 */
  from = $state(-1);
  /** 松手会落到的位置。 */
  to = $state(-1);
  dx = $state(0);
  dy = $state(0);
  #rects: DOMRect[] = [];
  #x0 = 0;
  #y0 = 0;
  #commit: (from: number, to: number) => void;

  constructor(commit: (from: number, to: number) => void) {
    this.#commit = commit;
  }

  attach = (node: HTMLElement) => {
    const items = () => [...node.querySelectorAll<HTMLElement>(":scope > [data-drag]")];
    return longPress(
      node,
      {
        hit: (e) => items().findIndex((c) => c.contains(e.target as Node)),
        lift: (i, e) => {
          this.#rects = items().map((c) => c.getBoundingClientRect());
          this.#x0 = e.clientX;
          this.#y0 = e.clientY;
          this.from = this.to = i;
          this.dx = this.dy = 0;
        },
        drag: (e) => {
          this.dx = e.clientX - this.#x0;
          this.dy = e.clientY - this.#y0;
          // 手指落在哪个小块原来的位置上，就排到那里；落在空隙里保持上一次的结果。
          const i = this.#rects.findIndex((r) => e.clientX >= r.left && e.clientX <= r.right && e.clientY >= r.top && e.clientY <= r.bottom);
          if (i >= 0) this.to = i;
        },
        drop: () => {
          const { from, to } = this;
          if (from < 0) return false;
          this.from = this.to = -1;
          this.dx = this.dy = 0;
          if (from !== to) this.#commit(from, to);
          return true;
        },
        lifted: () => this.from >= 0,
      },
      true,
    );
  };
}
