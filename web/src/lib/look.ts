import { LOGOS } from "./logos";
import type { Account } from "./types";

/** 一级分类序号对应的主题色变量。 */
export const color = (index: number) => `var(--c${index % 8})`;

/** 饼环的一段：颜色跟着分类走，不跟排名。 */
export interface Slice {
  name: string;
  cents: number;
  color: string;
}

/** 取前 `n` 个分类，其余连同兜底分类 `other` 并成灰色的一段，用兜底分类的名字。 */
export function slices(parts: { name: string; cents: number }[], index: (name: string) => number, other: string, n = 5): Slice[] {
  const top = parts.filter((p) => p.name !== other).slice(0, n);
  const rest = parts.filter((p) => !top.includes(p)).reduce((s, p) => s + p.cents, 0);
  const out = top.map((p) => ({ name: p.name, cents: p.cents, color: color(index(p.name)) }));
  if (rest) out.push({ name: other, cents: rest, color: "var(--label3)" });
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
