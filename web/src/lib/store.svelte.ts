// 页面状态和所有操作；主题只负责展示，改数据都走这里。
import type { Account, Entry, FieldKey, Fields } from "./types";
import { catalog, entries as mockEntries } from "./mock";
import { dayKey, periodOf, periodRange } from "./format";
import { holdKeyboard } from "../ui/nav.svelte";

export type Tab = "list" | "wealth" | "settings";

/** 推入的页面：账目详情、统计、某个账户的账目（和账目页同一个页面，按账户筛选）。 */
export type Page = { kind: "entry"; id: string } | { kind: "stats" } | { kind: "account"; name: string };

/** 用户设置；原型里存 localStorage，正式版存 SQLite。 */
interface Prefs {
  /** 记账周期从每月几号开始，1–28。 */
  cycleDay: number;
  /** 每个周期的支出预算，分；空为不设。 */
  budget: number | null;
}

const PREFS = "micjz.prefs";
export const prefs: Prefs = $state({ cycleDay: 1, budget: 300000, ...JSON.parse(localStorage.getItem(PREFS) ?? "{}") });

export function setPrefs(patch: Partial<Prefs>) {
  Object.assign(prefs, patch);
  localStorage.setItem(PREFS, JSON.stringify(prefs));
  app.period = currentPeriod();
}

export const currentPeriod = () => periodOf(new Date().toISOString(), prefs.cycleDay);

interface Toast {
  text: string;
  undo?: () => void;
}

const UNDO_MS = 4000;

export const app = $state({
  catalog,
  entries: mockEntries(),
  /** 正在看的记账周期，见 `periodOf`。 */
  period: currentPeriod(),
  tab: "list" as Tab,
  /** 推入的页面，栈顶是当前页。 */
  stack: [] as Page[],
  sheet: null as { id: string; field: FieldKey } | null,
  toast: null as Toast | null,
  /** 已删、可撤销的账目：撤销期过了才真正删除。 */
  hidden: [] as string[],
});

export const find = (id: string) => app.entries.find((e) => e.id === id);

export function periodEntries(period: string, account?: string): Entry[] {
  return app.entries.filter(
    (e) => periodOf(e.occurred_at, prefs.cycleDay) === period && !app.hidden.includes(e.id) && (!account || e.account === account),
  );
}

export function byDay(list: Entry[]): { day: string; at: string; entries: Entry[]; spent: number }[] {
  const groups = new Map<string, Entry[]>();
  for (const e of list) {
    const k = dayKey(e.occurred_at);
    groups.set(k, [...(groups.get(k) ?? []), e]);
  }
  return [...groups.entries()].map(([day, entries]) => ({
    day,
    at: entries[0].occurred_at,
    entries,
    spent: total(entries, "expense"),
  }));
}

/** 周期内每天的支出，下标 0 是周期第一天。 */
export function daily(period: string, list: Entry[]): number[] {
  const { start, days } = periodRange(period, prefs.cycleDay);
  const out = new Array(days).fill(0);
  for (const e of list) {
    if (e.kind !== "expense") continue;
    const d = new Date(e.occurred_at);
    const i = Math.round((new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime() - start.getTime()) / 86400000);
    out[i] += e.amount_cents;
  }
  return out;
}

/** 今天在周期里是第几天（下标）；不在这个周期为 -1。 */
export function todayIndex(period: string): number {
  const { start, days } = periodRange(period, prefs.cycleDay);
  const now = new Date();
  const i = Math.floor((new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime() - start.getTime()) / 86400000);
  return i >= 0 && i < days ? i : -1;
}

/** 按某个维度汇总支出，从大到小。 */
export function sumBy(list: Entry[], key: (e: Entry) => string | string[] | null): { name: string; cents: number; count: number }[] {
  const sums = new Map<string, { cents: number; count: number }>();
  for (const e of list) {
    if (e.kind !== "expense") continue;
    const k = key(e);
    for (const name of k === null ? [] : Array.isArray(k) ? k : [k]) {
      const s = sums.get(name) ?? { cents: 0, count: 0 };
      sums.set(name, { cents: s.cents + e.amount_cents, count: s.count + 1 });
    }
  }
  return [...sums].map(([name, s]) => ({ name, ...s })).sort((a, b) => b.cents - a.cents);
}

export const total = (list: Entry[], kind: Fields["kind"]) =>
  list.filter((e) => e.kind === kind).reduce((s, e) => s + e.amount_cents, 0);

/** 支出按一级分类汇总，从大到小。 */
export const breakdown = (list: Entry[]) => sumBy(list, (e) => e.category);

/** 一级分类在列表里的序号，主题按它取颜色。 */
export const categoryIndex = (e: Pick<Fields, "kind" | "category">) =>
  Math.max(0, app.catalog[e.kind].findIndex((c) => c.name === e.category));

export const open = (id: string) => app.stack.push({ kind: "entry", id });
export const openStats = () => app.stack.push({ kind: "stats" });
export const openAccount = (name: string) => app.stack.push({ kind: "account", name });

/** 调整账户顺序；排第一的是默认账户，识别不出账户时用它。接服务器后写 `accounts.position`，原型里只改内存。 */
export function moveAccount(from: number, to: number) {
  const list = [...app.catalog.accounts];
  list.splice(to, 0, ...list.splice(from, 1));
  app.catalog.accounts = list;
  if (from === 0 || to === 0) toast(`默认账户：${list[0].name}`);
}

export function setCard(name: string, card: Account["card"]) {
  app.catalog.accounts.find((a) => a.name === name)!.card = card;
}

/** 复制文本。局域网 http 下没有 clipboard API，退回 execCommand。 */
export async function copy(text: string, what: string) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const t = document.createElement("textarea");
    t.value = text;
    t.style.cssText = "position:fixed;opacity:0";
    document.body.append(t);
    t.select();
    document.execCommand("copy");
    t.remove();
  }
  toast(`已复制${what}`);
}
export const back = () => app.stack.pop();
export function edit(id: string, field: FieldKey) {
  if (field === "amount_cents") holdKeyboard("decimal");
  if (field === "merchant" || field === "note") holdKeyboard();
  app.sheet = { id, field };
}
export const closeSheet = () => (app.sheet = null);

export function update(id: string, patch: Partial<Fields>) {
  const e = find(id);
  if (!e) return;
  // 换类型后原分类不再适用：回到「其他」，接着让用户选分类（同快捷指令）。
  if (patch.kind && patch.kind !== e.kind) {
    Object.assign(e, { kind: patch.kind, category: "其他", subcategory: null });
    app.sheet = { id, field: "category" };
    return;
  }
  Object.assign(e, patch);
}

export function remove(id: string) {
  const top = app.stack.at(-1);
  if (top?.kind === "entry" && top.id === id) app.stack.pop();
  app.hidden.push(id);
  const timer = setTimeout(() => {
    app.entries = app.entries.filter((e) => e.id !== id);
    app.hidden = app.hidden.filter((h) => h !== id);
  }, UNDO_MS);
  toast("已删除", () => {
    clearTimeout(timer);
    app.hidden = app.hidden.filter((h) => h !== id);
  });
}

let toastTimer: ReturnType<typeof setTimeout> | undefined;
export function toast(text: string, undo?: () => void) {
  clearTimeout(toastTimer);
  app.toast = { text, undo };
  toastTimer = setTimeout(() => (app.toast = null), UNDO_MS);
}
