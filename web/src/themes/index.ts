import type { Theme } from "../lib/theme.svelte";
import GlowList from "./glow/List.svelte";
import GlowDetail from "./glow/Detail.svelte";
import GlowSettings from "./glow/Settings.svelte";
import GlowTabBar from "./glow/TabBar.svelte";
import GlowStats from "./glow/Stats.svelte";
import GlowWealth from "./glow/Wealth.svelte";
import CuteDetail from "./cute/Detail.svelte";
import CuteTabBar from "./cute/TabBar.svelte";

export const THEMES: Theme[] = [
  {
    id: "glow",
    name: "晶彩",
    blurb: "iOS 的手感，鲜亮的渐变和图标",
    List: GlowList,
    Detail: GlowDetail,
    Settings: GlowSettings,
    Stats: GlowStats,
    Wealth: GlowWealth,
    TabBar: GlowTabBar,
  },
  // 软糖的账目、设置、统计、财富页沿用晶彩的结构，只换配色：主题可以只重写一部分页面。
  {
    id: "cute",
    name: "软糖",
    blurb: "圆圆软软，卡片和小票",
    List: GlowList,
    Detail: CuteDetail,
    Settings: GlowSettings,
    Stats: GlowStats,
    Wealth: GlowWealth,
    TabBar: CuteTabBar,
  },
];

export const theme = (id: string) => THEMES.find((t) => t.id === id) ?? THEMES[0];
