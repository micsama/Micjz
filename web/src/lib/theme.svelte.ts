import type { Component } from "svelte";
import type { Entry } from "./types";

/**
 * 主题 = 一套 CSS 变量 + 一组页面组件。
 * 颜色、字体、圆角、动效曲线放变量里；列表、详情、设置、统计、财富、标签栏的结构由主题自己写，
 * 共用 `lib/store` 的数据和操作、`ui/` 的手势组件（面板、左滑、推入返回）。
 */
export interface Theme {
  id: string;
  name: string;
  /** 设置页里的一句介绍。 */
  blurb: string;
  /** 账目列表；传 `account` 时是推入的账户明细页。 */
  List: Component<{ account?: number }>;
  Detail: Component<{ entry: Entry }>;
  Settings: Component;
  Stats: Component;
  Wealth: Component;
  TabBar: Component;
}

const KEY = "micjz.theme";

export const themeState = $state({ id: localStorage.getItem(KEY) ?? "glow" });

export function setTheme(id: string) {
  const apply = () => {
    themeState.id = id;
    localStorage.setItem(KEY, id);
  };
  // 换主题时整页交叉淡入（Safari 18+）；不支持就直接换。
  if (document.startViewTransition) document.startViewTransition(apply);
  else apply();
}
