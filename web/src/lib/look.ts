import { LOGOS } from "./logos";
import type { Account } from "./types";

// 展示用的分类 emoji：主题可选用；新加的分类没配就用一级的，再没有用 ✨。
const EMOJI: Record<string, string> = {
  餐饮: "🍜", 三餐: "🍱", 咖啡饮品: "☕️", 零食水果: "🍓", 聚餐: "🍲",
  交通: "🚇", 公交地铁: "🚇", 打车: "🚕", 加油充电: "⛽️", 停车: "🅿️", 火车飞机: "✈️",
  购物: "🛍️", 日用: "🧻", 服饰: "👕", 数码: "🎧", 家居: "🛋️",
  居住: "🏠", 房租: "🔑", 水电燃气: "💡", 话费网费: "📶", 物业: "🏢",
  娱乐: "🎮", 会员订阅: "📺", 游戏: "🎮", 电影演出: "🎬", 旅行: "🧳",
  医疗: "💊", 药品: "💊", 门诊: "🩺",
  人情: "🎁", 红包: "🧧", 礼物: "🎁", 请客: "🍻",
  工资: "💰", 奖金: "🏆", 退款: "↩️", 理财: "📈", 其他: "✨",
};

export const emoji = (f: { category: string; subcategory: string | null }) =>
  (f.subcategory && EMOJI[f.subcategory]) || EMOJI[f.category] || "✨";

/** 一级分类序号对应的主题色变量。 */
export const color = (index: number) => `var(--c${index % 8})`;

/** 饼环的一段：颜色跟着分类走，不跟排名。 */
export interface Slice {
  name: string;
  cents: number;
  color: string;
}

/** 取前 `n` 个分类，其余（连同「其他」）并成灰色的「其他」。 */
export function slices(parts: { name: string; cents: number }[], index: (name: string) => number, n = 5): Slice[] {
  const top = parts.filter((p) => p.name !== "其他").slice(0, n);
  const rest = parts.filter((p) => !top.includes(p)).reduce((s, p) => s + p.cents, 0);
  const out = top.map((p) => ({ name: p.name, cents: p.cents, color: color(index(p.name)) }));
  if (rest) out.push({ name: "其他", cents: rest, color: "var(--label3)" });
  return out;
}

/** 账户的标志：有机构用机构的图形，「其他」用账户名首字。 */
export function brand(a: Pick<Account, "name" | "brand">): { bg: string; path?: string; mark: string } {
  const b = LOGOS.find(([k]) => k === a.brand);
  return b ? { bg: b[1], path: b[2], mark: "" } : { bg: "#5e6e82", mark: a.name.slice(0, 1) };
}

/** 新建账户时按名称猜机构，省一次点选。 */
export const guessBrand = (name: string) => LOGOS.find(([k]) => name.includes(k))?.[0] ?? null;

export const cardLabel = (card: { type: "debit" | "credit"; number: string }) =>
  `${card.type === "debit" ? "储蓄卡" : "信用卡"} ·· ${card.number.slice(-4)}`;
