// 页面状态和所有操作；主题只负责展示，改数据都走这里。
import type { Account, Category, Entry, FieldKey, Fields, FlowKind } from "./types";
import { catalog, seed } from "./mock";
import { effect, isIncome, isSpend, listed } from "./kinds";
import { dayKey, newestFirst, isYear, periodOf, periodRange, within } from "./format";
import { holdKeyboard } from "../ui/nav.svelte";

export type Tab = "list" | "stats" | "wealth" | "settings";

/** 推入的页面：账目详情。 */
export type Page = { kind: "entry"; id: string };

/** 统计页的筛选：三项可组合，空为不限。一直保留到手动取消，切标签不丢。 */
export interface Filter {
  account: number | null;
  /** 一级或二级分类；一级包含它下面的二级。 */
  category: number | null;
  tag: number | null;
}
export type Dim = keyof Filter;
const ANY: Filter = { account: null, category: null, tag: null };

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
  app.period = currentPeriod(isYear(app.period));
}

/** 今天所在的周期；`year` 为真时取所在的年。 */
export const currentPeriod = (year = false) => {
  const m = periodOf(new Date().toISOString(), prefs.cycleDay);
  return year ? m.slice(0, 4) : m;
};

/** 已经是最新的周期，不能再往后翻。 */
export const isLatest = (key: string) => key >= currentPeriod(isYear(key));

/** 切到年再切回月时，回到原来看的那个月。 */
let lastMonth = "";

/** 按月或按年看；切换时尽量停在同一段时间里。 */
export function setUnit(year: boolean) {
  if (year === isYear(app.period)) return;
  if (year) {
    lastMonth = app.period;
    app.period = app.period.slice(0, 4);
    return;
  }
  const y = app.period;
  app.period = within(lastMonth, y) ? lastMonth : y === currentPeriod(true) ? currentPeriod() : `${y}-12`;
}

/** 周期的支出预算：按年看是每月预算的 12 倍。 */
export const budgetOf = (key: string) => (prefs.budget === null ? null : prefs.budget * (isYear(key) ? 12 : 1));

interface Toast {
  text: string;
  undo?: () => void;
}

const UNDO_MS = 4000;
const HIDE = "micjz.hideBalance";

export function toggleBalance() {
  app.hideBalance = !app.hideBalance;
  localStorage.setItem(HIDE, app.hideBalance ? "1" : "0");
}

const data = seed();

export const app = $state({
  catalog,
  entries: data.entries,
  /** 截图来的账目对应的识别，按 `recognition_id` 取。 */
  recognitions: data.recognitions,
  /** 正在看的记账周期，月或年，见 `periodOf`。账目、财富、统计三页共用。 */
  period: currentPeriod(),
  /** 隐藏余额，在别人面前打开时用。 */
  hideBalance: localStorage.getItem(HIDE) === "1",
  tab: "list" as Tab,
  creating: false,
  /** 推入的页面，栈顶是当前页。 */
  stack: [] as Page[],
  sheet: null as { id: string; field: FieldKey } | null,
  filter: { ...ANY } as Filter,
  /** 正在选的筛选项，弹出选择面板。 */
  picking: null as Dim | null,
  toast: null as Toast | null,
  /** 已删、可撤销的账目：撤销期过了才真正删除。 */
  hidden: [] as string[],
});

export const find = (id: string) => app.entries.find((e) => e.id === id);

export const categoryOf = (id: number | null): Category | undefined => app.catalog.categories.find((c) => c.id === id);
export const accountOf = (id: number | null): Account | undefined => app.catalog.accounts.find((a) => a.id === id);
/** 一级分类：自己或上级。 */
export const topOf = (c: Category) => (c.parent_id === null ? c : categoryOf(c.parent_id)!);
export const tops = (kind: FlowKind) => app.catalog.categories.filter((c) => c.kind === kind && c.parent_id === null);
export const subsOf = (c: Category) => app.catalog.categories.filter((s) => s.parent_id === c.id);
/** 一级分类在同类里的序号，主题按它取颜色。 */
export const topIndex = (c: Category) => Math.max(0, tops(c.kind).indexOf(topOf(c)));
/** 换类型或识别不出时的默认分类，和服务端一致。 */
const FALLBACK: Record<FlowKind, string> = { expense: "other", income: "other_in" };
/** 兜底分类「其他」，按 key 认，改名不影响。 */
export const fallbackOf = (kind: FlowKind) => app.catalog.categories.find((c) => c.key === FALLBACK[kind])!;

const live = (e: Entry) => !app.hidden.includes(e.id);

/** 周期里的账目；`account` 为账户视角，见 `listed`。 */
export function periodEntries(period: string, account?: number): Entry[] {
  return app.entries.filter((e) => live(e) && listed(e, account) && within(periodOf(e.occurred_at, prefs.cycleDay), period));
}

/** 账户此刻的余额，分，负数是欠款：所有账目的影响相加（期初余额也是一笔平账）。 */
export const balanceOf = (account: number) => app.entries.reduce((s, e) => s + (live(e) ? effect(e, account) : 0), 0);

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
  const { start, days } = periodRange(period, prefs.cycleDay);
  const year = isYear(period);
  const out = new Array(year ? 12 : days).fill(0);
  for (const e of list) {
    if (!isSpend(e)) continue;
    const d = new Date(e.occurred_at);
    const i = year
      ? Number(periodOf(e.occurred_at, prefs.cycleDay).slice(5)) - 1
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
  const { start, days } = periodRange(period, prefs.cycleDay);
  const now = new Date();
  const i = Math.floor((new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime() - start.getTime()) / 86400000);
  return i >= 0 && i < days ? i : -1;
}

/** 周期过了几天（含今天）、一共几天；过去的周期算整期。预算的时间进度用它。 */
export function elapsedDays(period: string): { elapsed: number; days: number } {
  const { start, end, days } = periodRange(period, prefs.cycleDay);
  const now = Date.now();
  if (now >= end.getTime()) return { elapsed: days, days };
  return { elapsed: Math.max(0, Math.floor((now - start.getTime()) / 86400000) + 1), days };
}

/** 按月周期分组，按年看和跳转面板用。 */
export function byMonth(list: Entry[]): Map<string, Entry[]> {
  const out = new Map<string, Entry[]>();
  for (const e of list) {
    const k = periodOf(e.occurred_at, prefs.cycleDay);
    out.set(k, [...(out.get(k) ?? []), e]);
  }
  return out;
}

/** 每个月周期的支出，跳转面板用。 */
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
  return [...sums].map(([name, s]) => ({ name, ...s })).sort((a, b) => b.cents - a.cents);
}

/** 支出按一级分类汇总，从大到小。 */
export const breakdown = (list: Entry[]) => sumBy(list, (e) => topOf(categoryOf(e.category_id)!).name);

/** 支出一级分类在 `breakdown` 里的颜色序号，按名称找。 */
export const spendIndex = (name: string) =>
  Math.max(
    0,
    tops("expense").findIndex((c) => c.name === name),
  );

export const open = (id: string) => app.stack.push({ kind: "entry", id });
/** 统计只有一个：底部「统计」标签。别处的入口都切到它，看的是同一个周期；带了筛选就换成这组筛选。 */
export function openStats(filter?: Partial<Filter>) {
  app.stack = [];
  app.tab = "stats";
  if (filter) app.filter = { ...ANY, ...filter };
}
/** 看某个账户就是按这个账户筛选的统计。 */
export const openAccount = (id: number) => openStats({ account: id });

export const filtering = () => Object.values(app.filter).some((v) => v !== null);
/** 选好一项筛选；`keep` 为真时不收起面板（选了一级分类，接着选二级）。 */
export const setFilter = (dim: Dim, id: number | null, keep = false) => {
  app.filter[dim] = id;
  if (!keep) app.picking = null;
};
export const clearFilter = () => {
  app.filter = { ...ANY };
  app.picking = null;
};

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

/** 调整账户顺序；排第一的是默认账户，识别不出账户时用它。接服务器后写 `accounts.position`，原型里只改内存。 */
export function moveAccount(from: number, to: number) {
  const list = [...app.catalog.accounts];
  list.splice(to, 0, ...list.splice(from, 1));
  app.catalog.accounts = list;
  if (from === 0 || to === 0) toast(`默认账户：${list[0].name}`);
}

/** 新建（`id` 为 0）或修改账户；名称冲突返回错误文案。 */
export function saveAccount(next: Account): string | null {
  const list = app.catalog.accounts;
  if (list.some((a) => a.name === next.name && a.id !== next.id)) return "已经有同名账户";
  const i = list.findIndex((a) => a.id === next.id);
  if (i >= 0) {
    list[i] = next;
    return null;
  }
  list.push({ ...next, id: Math.max(0, ...list.map((a) => a.id)) + 1 });
  toast(`已添加 ${next.name}`);
  return null;
}

/** 删除账户；还有账目在用就不删，返回错误文案。 */
export function removeAccount(id: number): string | null {
  const used = app.entries.filter((e) => e.account_id === id || e.to_account_id === id).length;
  if (used) return `还有 ${used} 笔账目在用，不能删除`;
  const list = app.catalog.accounts;
  const i = list.findIndex((a) => a.id === id);
  list.splice(i, 1);
  toast(i === 0 && list.length ? `已删除，默认账户：${list[0].name}` : "已删除");
  return null;
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
  // 平账金额可以是负数，数字键盘没有负号。
  if (field === "amount_cents") holdKeyboard(find(id)?.kind === "adjust" ? "text" : "decimal");
  if (field === "merchant" || field === "note") holdKeyboard();
  app.sheet = { id, field };
}
export const closeSheet = () => (app.sheet = null);

export function update(id: string, patch: Partial<Fields>) {
  const e = find(id);
  if (!e) return;
  // 支出、收入互换后原分类不再适用：回到「其他」，接着让用户选分类（同快捷指令）。转账、平账不能换类型。
  if (patch.kind && patch.kind !== e.kind) {
    const kind = patch.kind as FlowKind;
    e.kind = kind;
    e.category_id = fallbackOf(kind).id;
    app.sheet = { id, field: "category_id" };
    return;
  }
  Object.assign(e, patch);
}

/** 手记一笔要填的字段；ID、入账时刻由存储方生成，手记的没有识别。 */
export type EntryInput = Omit<Entry, "id" | "recognition_id" | "created_at">;

/** 手记一笔。原型里只进内存；接服务器后由服务器生成 ID（UUIDv7）。记完跳到它所在的周期。 */
export function addEntry(fields: EntryInput) {
  const now = new Date().toISOString();
  app.entries.push({ ...fields, id: crypto.randomUUID(), recognition_id: null, created_at: now });
  app.entries.sort(newestFirst);
  app.tab = "list";
  app.stack = [];
  app.period = periodOf(fields.occurred_at, prefs.cycleDay);
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
