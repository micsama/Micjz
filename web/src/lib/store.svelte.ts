// 页面状态和所有操作；主题只负责展示，读写服务器都走这里。
// 数据层只按 `occurred_at` 存取，按东八区自然年从服务器取、缓存；周期、统计这些视图在 `query.ts` 里从缓存算。
import type { Account, Catalog, Category, Entry, FieldKey, FlowKind, Recognition } from "./types";
import { api, ApiError, type AccountInput, type EntryInput, type EntryPatch, type Settings } from "./api";
import { isYear, newestFirst, periodOf, periodRange, shiftPeriod, within } from "./format";
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

interface Toast {
  text: string;
  undo?: () => void;
}

const UNDO_MS = 4000;
const HIDE = "micjz.hideBalance";

/** 服务器上的用户设置；启动时取到之后才进入页面。 */
export const settings: Settings = $state({ cycle_day: 1, budget_cents: null, version: "", entry_count: 0 });

export const app = $state({
  /** 进页面之前：取数据中、要登录、取不到。 */
  phase: "loading" as "loading" | "login" | "failed" | "ready",
  /** `failed` 时的原因。 */
  failure: "",
  catalog: { categories: [], accounts: [], tags: [] } as Catalog,
  /** 已取过的年份里的全部账目，新的在前。 */
  entries: [] as Entry[],
  /** 已取过的自然年。 */
  years: [] as number[],
  /** 全库最早的一笔，跳转面板翻到这一年为止。 */
  earliest: null as string | null,
  /** 各账户此刻的余额，分，服务器算。 */
  balances: {} as Record<number, number>,
  /** 截图来的账目对应的识别，按 `recognition_id` 取，打开详情时才取。 */
  recognitions: {} as Record<string, Recognition>,
  /** 正在看的记账周期，月或年，见 `periodOf`。账目、财富、统计三页共用。 */
  period: "",
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

// ── 和服务器之间 ──

/** 请求失败的统一处理：401 回登录页，409 说明目录变了、重取。返回结果或那个错误，提示由调用方决定。 */
async function attempt<T>(p: Promise<T>): Promise<T | ApiError> {
  try {
    return await p;
  } catch (e) {
    if (!(e instanceof ApiError)) throw e;
    if (e.status === 401) app.phase = "login";
    if (e.status === 409) void reloadCatalog();
    return e;
  }
}

/** 失败就弹提示，返回空。 */
async function guard<T>(p: Promise<T>): Promise<T | undefined> {
  const r = await attempt(p);
  if (!(r instanceof ApiError)) return r;
  if (r.status !== 401) toast(r.message);
  return undefined;
}

async function reloadCatalog() {
  const c = await attempt(api.catalog());
  if (!(c instanceof ApiError)) app.catalog = c;
}

/** 写之后余额、笔数会变，从服务器重取。 */
async function refreshTotals() {
  const [b, s] = await Promise.all([attempt(api.balances()), attempt(api.settings())]);
  if (!(b instanceof ApiError)) app.balances = Object.fromEntries(b.balances.map((x) => [x.account_id, x.balance_cents]));
  if (!(s instanceof ApiError)) Object.assign(settings, s);
}

/** 打开、登录后：取目录、设置、余额，再把当期和上一期要的年份取齐才进页面，免得先闪一下空数据。 */
export async function boot() {
  app.phase = "loading";
  const [c, s, b] = await Promise.all([attempt(api.catalog()), attempt(api.settings()), attempt(api.balances())]);
  const failed = [c, s, b].find((r) => r instanceof ApiError);
  if (failed) {
    if (failed.status !== 401) [app.phase, app.failure] = ["failed", failed.message];
    return;
  }
  app.catalog = c as Catalog;
  Object.assign(settings, s);
  app.balances = Object.fromEntries((b as Awaited<ReturnType<typeof api.balances>>).balances.map((x) => [x.account_id, x.balance_cents]));
  app.period = currentPeriod();
  await Promise.all([want(app.period), want(shiftPeriod(app.period, -1))]);
  if (app.phase === "loading") app.phase = "ready";
}

/** 登录成功返回空，失败返回给人看的原因。登录的 401、409 是表单上的错，不走 `attempt` 的回登录页、重取目录。 */
export async function login(username: string, password: string): Promise<string | null> {
  try {
    await api.login(username, password);
  } catch (e) {
    if (e instanceof ApiError) return e.message;
    throw e;
  }
  await boot();
  return null;
}

/** 改密码：成功返回空；其他设备随之下线，这台换了新 Cookie 继续用。 */
export async function changePassword(current: string, next: string): Promise<string | null> {
  const r = await attempt(api.changePassword(current, next));
  return r instanceof ApiError ? r.message : null;
}

/** 不管服务器那边成没成，都重新打开：Cookie 清掉了就回到登录页。 */
export async function logout() {
  await attempt(api.logout());
  location.reload();
}

/** 网页回到前台：快捷指令可能记过账，重取目录、设置、余额和已取过的年份。 */
export async function refresh() {
  if (app.phase !== "ready") return;
  await Promise.all([reloadCatalog(), refreshTotals(), ...app.years.map(fetchYear)]);
}

const yearOf = (iso: string) => new Date(iso).getFullYear();
const inflight = new Map<number, Promise<void>>();

/** 取一整年，替换内存里这一年的账目。同一年同时只取一次。 */
function fetchYear(y: number): Promise<void> {
  let p = inflight.get(y);
  if (!p) {
    p = guard(api.entries(new Date(y, 0, 1), new Date(y + 1, 0, 1)))
      .then((r) => {
        if (!r) return;
        app.entries = [...app.entries.filter((e) => yearOf(e.occurred_at) !== y), ...r.entries].sort(newestFirst);
        if (!app.years.includes(y)) app.years.push(y);
        app.earliest = r.earliest;
      })
      .finally(() => inflight.delete(y));
    inflight.set(y, p);
  }
  return p;
}

/** 这段周期（月或年）覆盖的自然年都取齐；视图要看哪段就先要哪段。 */
export function want(period: string): Promise<void> {
  const { start, end } = periodRange(period, settings.cycle_day);
  const last = Math.min(new Date(end.getTime() - 1).getFullYear(), new Date().getFullYear());
  const todo = [];
  for (let y = start.getFullYear(); y <= last; y++) if (!app.years.includes(y)) todo.push(fetchYear(y));
  return Promise.all(todo).then(() => {});
}

/** 服务器返回的一笔放进内存：它那年取过就放进去（有就替换）；没取过就去取那一年。 */
function place(e: Entry) {
  app.entries = [...app.entries.filter((x) => x.id !== e.id), e].sort(newestFirst);
  if (!app.years.includes(yearOf(e.occurred_at))) void fetchYear(yearOf(e.occurred_at));
}

const loading = new Set<string>();
/** 详情页打开时取那次识别。 */
export async function loadRecognition(id: string) {
  if (app.recognitions[id] || loading.has(id)) return;
  loading.add(id);
  const r = await attempt(api.recognition(id));
  loading.delete(id);
  if (!(r instanceof ApiError)) app.recognitions[id] = r;
}

// ── 周期 ──

/** 今天所在的周期；`year` 为真时取所在的年。 */
export const currentPeriod = (year = false) => {
  const m = periodOf(new Date().toISOString(), settings.cycle_day);
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

/** 改记账周期、预算；以服务器返回为准。 */
export async function saveSettings(patch: Partial<Pick<Settings, "cycle_day" | "budget_cents">>) {
  const s = await guard(api.putSettings({ cycle_day: settings.cycle_day, budget_cents: settings.budget_cents, ...patch }));
  if (!s) return;
  const moved = s.cycle_day !== settings.cycle_day;
  Object.assign(settings, s);
  if (moved) app.period = currentPeriod(isYear(app.period));
}

// ── 目录 ──

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

// ── 导航 ──

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

export const back = () => app.stack.pop();
export function edit(id: string, field: FieldKey) {
  // 平账金额可以是负数，数字键盘没有负号。
  if (field === "amount_cents") holdKeyboard(find(id)?.kind === "adjust" ? "text" : "decimal");
  if (field === "merchant" || field === "note") holdKeyboard();
  app.sheet = { id, field };
}
export const closeSheet = () => (app.sheet = null);

export function toggleBalance() {
  app.hideBalance = !app.hideBalance;
  localStorage.setItem(HIDE, app.hideBalance ? "1" : "0");
}

// ── 账目的写 ──

/** 手记一笔，成功返回真。记完跳到它所在的周期。 */
export async function addEntry(fields: EntryInput): Promise<boolean> {
  const e = await guard(api.addEntry(fields));
  if (!e) return false;
  place(e);
  void refreshTotals();
  app.tab = "list";
  app.stack = [];
  app.period = periodOf(e.occurred_at, settings.cycle_day);
  return true;
}

/**
 * 改一笔：先改内存让界面立刻跟上，再以服务器返回为准；失败就改回去。
 * 支出、收入互换后原分类不再适用：换成「其他」，接着让用户选分类（同快捷指令）。转账、平账不能换类型。
 */
export async function update(id: string, patch: EntryPatch) {
  const e = find(id);
  if (!e) return;
  if (patch.kind && patch.kind !== e.kind) {
    patch = { kind: patch.kind, category_id: fallbackOf(patch.kind as FlowKind).id };
    app.sheet = { id, field: "category_id" };
  }
  const before = Object.fromEntries(Object.keys(patch).map((k) => [k, e[k as keyof EntryPatch]]));
  Object.assign(e, patch);
  const next = await guard(api.patchEntry(id, patch));
  if (!next) {
    const cur = find(id);
    if (cur) Object.assign(cur, before);
    return;
  }
  place(next);
  void refreshTotals();
}

/** 撤销期内的删除：到期或网页切到后台时才真正发出。 */
const pending = new Map<string, ReturnType<typeof setTimeout>>();
const unhide = (id: string) => (app.hidden = app.hidden.filter((h) => h !== id));

export function remove(id: string) {
  const top = app.stack.at(-1);
  if (top?.kind === "entry" && top.id === id) app.stack.pop();
  app.hidden.push(id);
  pending.set(
    id,
    setTimeout(() => commitDelete(id), UNDO_MS),
  );
  toast("已删除", () => {
    clearTimeout(pending.get(id));
    pending.delete(id);
    unhide(id);
  });
}

async function commitDelete(id: string) {
  pending.delete(id);
  const r = await attempt(api.deleteEntry(id));
  // 404：已经不在了，和删除成功一样。
  if (r instanceof ApiError && r.status !== 404) {
    unhide(id);
    if (r.status !== 401) toast(`没删掉：${r.message}`);
    return;
  }
  app.entries = app.entries.filter((e) => e.id !== id);
  unhide(id);
  void refreshTotals();
}

/** 网页要切走了：撤销期没过的删除马上发出（请求带 keepalive，页面关了也能送达）。 */
export function flushDeletes() {
  for (const [id, timer] of pending) {
    clearTimeout(timer);
    void commitDelete(id);
  }
}

// ── 账户 ──

/** 调整账户顺序；排第一的是默认账户，识别不出账户时用它。先挪内存，再以服务器的顺序为准。 */
export async function moveAccount(from: number, to: number) {
  const list = [...app.catalog.accounts];
  const [moved] = list.splice(from, 1);
  list.splice(to, 0, moved);
  app.catalog.accounts = list;
  if (from === 0 || to === 0) toast(`默认账户：${list[0].name}`);
  const r = await guard(api.moveAccount(moved.id, to));
  if (r) app.catalog.accounts = r.accounts;
  else void reloadCatalog();
}

/** 新建（`id` 为 0）或修改账户；失败返回给人看的原因。 */
export async function saveAccount(next: Account): Promise<string | null> {
  const { id, ...input }: { id: number } & AccountInput = next;
  const r = await attempt(id ? api.putAccount(id, input) : api.addAccount(input));
  if (r instanceof ApiError) return r.message;
  const list = app.catalog.accounts;
  const i = list.findIndex((a) => a.id === r.id);
  if (i >= 0) list[i] = r;
  else {
    list.push(r);
    toast(`已添加 ${r.name}`);
    void refreshTotals();
  }
  return null;
}

/** 删除账户；还有账目在用、只剩一个时服务器不让删，返回原因。 */
export async function removeAccount(id: number): Promise<string | null> {
  const r = await attempt(api.deleteAccount(id));
  if (r instanceof ApiError && r.status !== 404) return r.message;
  const list = app.catalog.accounts;
  const i = list.findIndex((a) => a.id === id);
  if (i >= 0) list.splice(i, 1);
  toast(i === 0 && list.length ? `已删除，默认账户：${list[0].name}` : "已删除");
  return null;
}

// ── 提示 ──

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

let toastTimer: ReturnType<typeof setTimeout> | undefined;
export function toast(text: string, undo?: () => void) {
  clearTimeout(toastTimer);
  app.toast = { text, undo };
  toastTimer = setTimeout(() => (app.toast = null), UNDO_MS);
}
