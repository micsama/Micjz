// 视图层：周期、统计、走势都从已取到的账目算，`cycle_day` 只在这里和周期导航里用。只读，不改状态。
import type { Entry } from "./types";
import { app, categoryOf, currentPeriod, settings, topOf, tops } from "./store.svelte";
import { effect, isIncome, isSpend, listed } from "./kinds";
import { dayKey, isYear, newestFirst, periodOf, periodRange, within } from "./format";

const live = (e: Entry) => !app.hidden.includes(e.id);

/** 周期里的账目；`account` 为账户视角，见 `listed`。 */
export function periodEntries(period: string, account?: number): Entry[] {
  return app.entries.filter((e) => live(e) && listed(e, account) && within(periodOf(e.occurred_at, settings.cycle_day), period));
}

/** 账户此刻的余额，分，负数是欠款。服务器算的余额减去撤销期内已删的。 */
export const balanceOf = (account: number) => (app.balances[account] ?? 0) - app.entries.reduce((s, e) => s + (live(e) ? 0 : effect(e, account)), 0);

/** 周期的支出预算：按年看是每月预算的 12 倍。 */
export const budgetOf = (key: string) => (settings.budget_cents === null ? null : settings.budget_cents * (isYear(key) ? 12 : 1));

/** 金额合计和笔数。「几笔」总是跟着它旁边的金额算，不另数。 */
export interface Sum {
  cents: number;
  count: number;
}
const sum = (list: Entry[], pick: (e: Entry) => boolean): Sum =>
  list.reduce((s, e) => (pick(e) ? { cents: s.cents + e.amount_cents, count: s.count + 1 } : s), { cents: 0, count: 0 });
export const spending = (list: Entry[]) => sum(list, isSpend);
export const earning = (list: Entry[]) => sum(list, isIncome);

export function byDay(list: Entry[]): { day: string; at: string; entries: Entry[]; spent: number }[] {
  const groups = new Map<string, Entry[]>();
  for (const e of [...list].sort(newestFirst)) {
    const k = dayKey(e.occurred_at);
    groups.set(k, [...(groups.get(k) ?? []), e]);
  }
  return [...groups.entries()].map(([day, entries]) => ({ day, at: entries[0].occurred_at, entries, spent: spending(entries).cents }));
}

/** 截至 `last` 那天（含）的最近 `n` 天每天的支出，从早到晚。不分周期，首页走势用。 */
export function recentSpend(last: Date, n: number): { date: Date; cents: number }[] {
  const days = Array.from({ length: n }, (_, i) => ({ date: new Date(last.getFullYear(), last.getMonth(), last.getDate() - n + 1 + i), cents: 0 }));
  const index = new Map(days.map((d, i) => [dayKey(d.date.toISOString()), i]));
  for (const e of app.entries) {
    const i = live(e) && isSpend(e) ? index.get(dayKey(e.occurred_at)) : undefined;
    if (i !== undefined) days[i].cents += e.amount_cents;
  }
  return days;
}

/** 周期内的支出柱：按月看每天一根，按年看每月一根；下标 0 是第一天或 1 月。 */
export function spendBars(period: string, list: Entry[]): number[] {
  const { start, days } = periodRange(period, settings.cycle_day);
  const year = isYear(period);
  const out = new Array(year ? 12 : days).fill(0);
  for (const e of list) {
    if (!isSpend(e)) continue;
    const d = new Date(e.occurred_at);
    const i = year
      ? Number(periodOf(e.occurred_at, settings.cycle_day).slice(5)) - 1
      : Math.round((new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime() - start.getTime()) / 86400000);
    out[i] += e.amount_cents;
  }
  return out;
}

/** 今天在 `spendBars` 里的下标；不在这个周期为 -1。 */
export function todayIndex(period: string): number {
  if (isYear(period)) {
    const m = currentPeriod();
    return within(m, period) ? Number(m.slice(5)) - 1 : -1;
  }
  const { start, days } = periodRange(period, settings.cycle_day);
  const now = new Date();
  const i = Math.floor((new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime() - start.getTime()) / 86400000);
  return i >= 0 && i < days ? i : -1;
}

/** 周期过了几天（含今天）、一共几天；过去的周期算整期。预算的时间进度用它。 */
export function elapsedDays(period: string): { elapsed: number; days: number } {
  const { start, end, days } = periodRange(period, settings.cycle_day);
  const now = Date.now();
  if (now >= end.getTime()) return { elapsed: days, days };
  return { elapsed: Math.max(0, Math.floor((now - start.getTime()) / 86400000) + 1), days };
}

/** 按月周期分组，按年看和跳转面板用。 */
export function byMonth(list: Entry[]): Map<string, Entry[]> {
  const out = new Map<string, Entry[]>();
  for (const e of list) {
    const k = periodOf(e.occurred_at, settings.cycle_day);
    out.set(k, [...(out.get(k) ?? []), e]);
  }
  return out;
}

/** 已取到的每个月周期的支出，跳转面板用；没取到的月份没有。 */
export const monthTotals = (): Map<string, Sum> => new Map([...byMonth(app.entries.filter((e) => live(e) && listed(e)))].map(([k, es]) => [k, spending(es)]));

/** 按某个维度汇总支出，从大到小。 */
export function sumBy(list: Entry[], key: (e: Entry) => string | string[] | null): { name: string; cents: number; count: number }[] {
  const sums = new Map<string, { cents: number; count: number }>();
  for (const e of list) {
    if (!isSpend(e)) continue;
    const k = key(e);
    for (const name of k === null ? [] : Array.isArray(k) ? k : [k]) {
      const s = sums.get(name) ?? { cents: 0, count: 0 };
      sums.set(name, { cents: s.cents + e.amount_cents, count: s.count + 1 });
    }
  }
  // 退款多于花销的项（如本期退了上期买的）合计为负，画不进占比，略去。
  return [...sums]
    .map(([name, s]) => ({ name, ...s }))
    .filter((s) => s.cents > 0)
    .sort((a, b) => b.cents - a.cents);
}

/** 支出按一级分类汇总，从大到小。 */
export const breakdown = (list: Entry[]) => sumBy(list, (e) => topOf(categoryOf(e.category_id)!).name);

/** 支出一级分类在 `breakdown` 里的颜色序号，按名称找。 */
export const spendIndex = (name: string) =>
  Math.max(
    0,
    tops("expense").findIndex((c) => c.name === name),
  );

/** 统计看的账目：周期内、符合筛选。按账户筛时连同转账、平账一起算进来（见 `listed`）。 */
export function statEntries(period: string): Entry[] {
  const { account, category, tag } = app.filter;
  return periodEntries(period, account ?? undefined).filter((e) => {
    if (tag !== null && !e.tag_ids.includes(tag)) return false;
    if (category === null) return true;
    const c = categoryOf(e.category_id);
    return !!c && (c.id === category || c.parent_id === category);
  });
}
