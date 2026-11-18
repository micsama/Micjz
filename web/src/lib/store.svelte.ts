// 页面状态和所有操作；主题只负责展示，读写服务器都走这里。
// 账目按实际周期起止取、缓存；周期和统计视图在 query.ts 里计算。
import { webPerformance, type BootKind } from "./performance";
import type { Account, Catalog, Category, Entry, FieldKey, FlowKind, Recognition } from "./types";
import { checkUpdated } from "./update.svelte";
import { api, ApiError, type AccountInput, type CategoryInput, type EntryInput, type EntryPatch, type Settings } from "./api";
import { isYear, money, newestFirst, periodOf, periodRange, shiftPeriod, within } from "./format";
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
export const settings: Settings = $state({ cycle_day: 1, budget_cents: null, version: "", entry_count: 0, earliest: null });

export const app = $state({
  /** 进页面之前：取数据中、要登录、取不到。 */
  phase: "loading" as "loading" | "login" | "failed" | "ready",
  /** `failed` 时的原因。 */
  failure: "",
  catalog: { categories: [], accounts: [], tags: [] } as Catalog,
  /** 已取过的账目，新的在前。 */
  entries: [] as Entry[],
  periodLoads: {} as Record<string, "loading" | "ready" | "failed">,
  /** 各账户此刻的余额，分，服务器算。 */
  balances: {} as Record<number, number>,
  balanceLoad: "loading" as "loading" | "ready" | "failed",
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
  return !(c instanceof ApiError);
}

type WebPerformance = ReturnType<typeof webPerformance>;

export async function loadBalances(perf?: WebPerformance): Promise<boolean> {
  const read = () => attempt(api.balances());
  const b = await (perf ? perf.measure("balances_ms", read) : read());
  if (b instanceof ApiError) {
    if (app.balanceLoad !== "ready") app.balanceLoad = "failed";
    return false;
  }
  app.balances = Object.fromEntries(b.balances.map((x) => [x.account_id, x.balance_cents]));
  app.balanceLoad = "ready";
  return true;
}

async function loadSettings(perf?: WebPerformance): Promise<ApiError | undefined> {
  const read = () => attempt(api.settings());
  const s = await (perf ? perf.measure("settings_ms", read) : read());
  if (s instanceof ApiError) return s;
  Object.assign(settings, s);
}

/** 写之后余额、笔数和最早账目的时间可能改变。 */
async function refreshTotals() {
  await Promise.all([loadBalances(), loadSettings()]);
}

/** 设置决定当期范围。目录和当期账目齐了就显示，余额独立加载。 */
export async function boot(kind: BootKind = "retry") {
  const perf = webPerformance(kind);
  app.phase = "loading";
  app.balanceLoad = "loading";
  try {
    const catalog = perf.measure("catalog_ms", () => attempt(api.catalog()));
    const firstPeriod = loadSettings(perf).then(async (error) => {
      if (error) return error;
      app.period = currentPeriod();
      following = true;
      return perf.measure("entries_ms", () => want(app.period));
    });
    const [c, got] = await Promise.all([catalog, firstPeriod]);
    if (app.phase !== "loading") return;
    if (c instanceof ApiError || got instanceof ApiError || !got) {
      const failure = c instanceof ApiError ? c.message : got instanceof ApiError ? got.message : "账目没取到，请重试";
      [app.phase, app.failure] = ["failed", failure];
      return;
    }
    app.catalog = c;
    const updated = checkUpdated(settings.version);
    if (updated) toast(updated);
    app.phase = "ready";
    void loadBalances(perf);
  } finally {
    void perf.report(app.phase === "loading" ? "failed" : app.phase).catch(() => {});
  }
}

/** 登录成功返回空，失败返回给人看的原因。登录的 401、409 是表单上的错，不走 `attempt` 的回登录页、重取目录。 */
export async function login(username: string, password: string): Promise<string | null> {
  try {
    await api.login(username, password);
  } catch (e) {
    if (e instanceof ApiError) return e.message;
    throw e;
  }
  await boot("login");
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

let refreshing = false;

/** 回到前台只刷新正在看的周期。历史缓存下次打开时再校验。 */
export async function refresh() {
  if (app.phase !== "ready" || refreshing) return;
  refreshing = true;
  const perf = webPerformance("resume");
  let ok = false;
  try {
    if (following) app.period = currentPeriod(isYear(app.period));
    fresh.clear();
    const [catalog, balances, settingError, entries] = await Promise.all([
      perf.measure("catalog_ms", reloadCatalog),
      loadBalances(perf),
      loadSettings(perf),
      perf.measure("entries_ms", () => want(app.period)),
    ]);
    if (app.phase === "ready") ok = catalog && balances && !settingError && entries;
  } finally {
    refreshing = false;
    void perf.report(String(app.phase) === "login" ? "login" : ok ? "ready" : "failed").catch(() => {});
  }
}

const fresh = new Set<string>();
const inflight = new Map<string, { start: Date; end: Date; request: Promise<boolean> }>();
const cacheKey = (period: string, day: number) => `${day}:${period}`;
const cachedKeys = (period: string, day: number) => [cacheKey(period, day), cacheKey(period.slice(0, 4), day)];

export function periodStatus(period: string) {
  const [key, year] = cachedKeys(period, settings.cycle_day);
  return app.periodLoads[year] === "ready" ? "ready" : app.periodLoads[key];
}

/** 加载精确的周期范围，同期去重。整年数据也能满足其下的月周期。 */
export function want(period: string, day = settings.cycle_day): Promise<boolean> {
  const [key, year] = cachedKeys(period, day);
  if (fresh.has(key) || fresh.has(year)) return Promise.resolve(true);
  const existing = inflight.get(key);
  if (existing) return existing.request;

  const { start, end } = periodRange(period, day);
  const overlaps = [...inflight.values()].filter((other) => start < other.end && end > other.start);
  if (overlaps.length) return Promise.all(overlaps.map((other) => other.request)).then(() => want(period, day));

  if (app.periodLoads[key] !== "ready") app.periodLoads[key] = "loading";
  const request = guard(api.entries(start, end))
    .then((r) => {
      if (!r) {
        if (app.periodLoads[key] !== "ready") app.periodLoads[key] = "failed";
        return false;
      }
      app.entries = [
        ...app.entries.filter((e) => {
          const t = Date.parse(e.occurred_at);
          return t < start.getTime() || t >= end.getTime();
        }),
        ...r.entries,
      ].sort(newestFirst);
      settings.earliest = r.earliest;
      fresh.add(key);
      app.periodLoads[key] = "ready";
      return true;
    })
    .finally(() => inflight.delete(key));
  inflight.set(key, { start, end, request });
  return request;
}

/** 服务器返回的一笔放进内存（有就替换）。 */
function place(e: Entry) {
  app.entries = [...app.entries.filter((x) => x.id !== e.id), e].sort(newestFirst);
  if (!settings.earliest || Date.parse(e.occurred_at) < Date.parse(settings.earliest)) settings.earliest = e.occurred_at;
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

/**
 * 正在看的就是当前期：回到前台时跟着时间走到新的当前期。
 * 由 `app.period` 的变化维护（App.svelte 里的 effect），翻到历史就不跟了。
 */
let following = false;
export const follow = () => (following = app.period === currentPeriod(isYear(app.period)));

/** 已经是最新的周期，不能再往后翻。 */
export const isLatest = (key: string) => key >= currentPeriod(isYear(key));

/** 最早一笔按当前记账周期归属；空账本只有当前期。 */
export function earliestPeriod(year = false): string {
  const latest = currentPeriod();
  const first = settings.earliest ? periodOf(settings.earliest, settings.cycle_day) : latest;
  const earliest = first < latest ? first : latest;
  return year ? earliest.slice(0, 4) : earliest;
}

export const isEarliest = (key: string) => key <= earliestPeriod(isYear(key));

export const canSelectPeriod = (key: string) => key >= earliestPeriod(isYear(key)) && key <= currentPeriod(isYear(key));

export function enforcePeriodBounds() {
  if (!app.period || canSelectPeriod(app.period)) return;
  const year = isYear(app.period);
  app.period = isEarliest(app.period) ? earliestPeriod(year) : currentPeriod(year);
}

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
  enforcePeriodBounds();
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

/** 点底部标签：切到那一页；已经在那一页就滚回顶部（iOS 的习惯）。 */
export function selectTab(tab: Tab, button: HTMLElement) {
  if (app.tab !== tab) return void (app.tab = tab);
  button.closest(".base")?.querySelector<HTMLElement>(".page")?.scrollTo({ top: 0, behavior: "smooth" });
}

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

/**
 * 手记一笔，记上了返回 "new"；这个 `id` 之前已记上（上次响应丢了）返回 "existed"，以服务器那笔为准；失败为空。
 * 记完跳到它所在的周期。
 */
export async function addEntry(id: string, fields: EntryInput): Promise<"new" | "existed" | null> {
  const r = await guard(api.addEntry(id, fields));
  if (!r) return null;
  const e = r.entry;
  place(e);
  void refreshTotals();
  app.tab = "list";
  app.stack = [];
  app.period = periodOf(e.occurred_at, settings.cycle_day);
  return r.existed ? "existed" : "new";
}

/**
 * 改一笔：先改内存让界面立刻跟上，再以服务器返回为准；失败就改回去。
 * 支出、收入互换后原分类不再适用：换成「其他」，接着让用户选分类（同快捷指令）。转账、平账不能换类型。
 */
export function update(id: string, patch: EntryPatch) {
  const e = find(id);
  if (!e) return;
  if (patch.kind && patch.kind !== e.kind) {
    patch = { kind: patch.kind, category_id: fallbackOf(patch.kind as FlowKind).id };
    app.sheet = { id, field: "category_id" };
  }
  const before = Object.fromEntries(Object.keys(patch).map((k) => [k, e[k as keyof EntryPatch]]));
  Object.assign(e, patch);
  const run = async () => {
    const next = await guard(api.patchEntry(id, patch));
    // 后面还有排着的修改：中间结果不往界面上放，免得盖掉更新的选择；失败只撤回这次改的字段。
    const last = queue.get(id) === p;
    if (last) queue.delete(id);
    if (!next) {
      const cur = find(id);
      if (cur) for (const k of Object.keys(patch) as (keyof EntryPatch)[]) if (cur[k] === patch[k]) Object.assign(cur, { [k]: before[k] });
      return;
    }
    if (last) place(next);
    void refreshTotals();
  };
  // 同一笔的修改一个接一个发：连点两个标签、换类型后马上选分类，服务器按点的顺序写。
  const p: Promise<void> = (queue.get(id) ?? Promise.resolve()).then(run);
  queue.set(id, p);
  return p;
}
const queue = new Map<string, Promise<void>>();

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

/** 同类型、同上级的分类，按优先级。 */
export const siblingsOf = (c: Pick<Category, "kind" | "parent_id">) => app.catalog.categories.filter((s) => s.kind === c.kind && s.parent_id === c.parent_id);

/** 同级内拖动排序：先在本地挪好，再以服务器的顺序为准。 */
export async function moveCategory(c: Category, to: number) {
  const sibs = siblingsOf(c).filter((s) => s.id !== c.id);
  sibs.splice(to, 0, c);
  let i = 0;
  app.catalog.categories = app.catalog.categories.map((x) => (x.kind === c.kind && x.parent_id === c.parent_id ? sibs[i++] : x));
  const r = await guard(api.moveCategory(c.id, to));
  if (r) app.catalog.categories = r.categories;
  else void reloadCatalog();
}

/** 新建（`id` 为空）或修改分类；失败返回给人看的原因。 */
export async function saveCategory(id: number | null, input: CategoryInput): Promise<string | null> {
  const r = await attempt(id === null ? api.addCategory(input) : api.putCategory(id, input));
  if (r instanceof ApiError) return r.message;
  app.catalog.categories = r.categories;
  if (id === null) toast(`已添加 ${input.name.trim()}`);
  return null;
}

/** 删除分类；有账目、有二级、兜底分类服务器不让删，返回原因。 */
export async function removeCategory(id: number): Promise<string | null> {
  const r = await attempt(api.deleteCategory(id));
  if (r instanceof ApiError && r.status !== 404) return r.message;
  app.catalog.categories = app.catalog.categories.filter((c) => c.id !== id);
  toast("已删除");
  return null;
}

/** 新建（`id` 为空）或改名标签；失败返回原因。 */
export async function saveTag(id: number | null, name: string): Promise<string | null> {
  const r = await attempt(id === null ? api.addTag(name) : api.putTag(id, name));
  if (r instanceof ApiError) return r.message;
  app.catalog.tags = r.tags;
  if (id === null) toast(`已添加 ${name.trim()}`);
  return null;
}

/** 删除标签；还有账目在用时返回原因。 */
export async function removeTag(id: number): Promise<string | null> {
  const r = await attempt(api.deleteTag(id));
  if (r instanceof ApiError && r.status !== 404) return r.message;
  app.catalog.tags = app.catalog.tags.filter((t) => t.id !== id);
  toast("已删除");
  return null;
}

/** 平账：填账户实际余额，差额记成一笔平账；失败返回原因。 */
export async function adjust(account: number, balance_cents: number): Promise<string | null> {
  const r = await attempt(api.adjust(account, balance_cents));
  if (r instanceof ApiError) return r.message;
  if (r.entry) place(r.entry);
  app.balances[account] = r.balance_cents;
  toast(r.entry ? `已平账 ${money(r.entry.amount_cents, true)}` : "余额一致，不用调整");
  void refreshTotals();
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
