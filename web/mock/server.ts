// 假后端：照 docs/blueprints/web-api.md 在内存里实现 /api，`bun run demo` 不连真服务器时用。
// 只为前端开发和截图验收，规则按契约写但不求穷尽；以真后端为准。登录用 dzmfg / demo。
import type { Account, Catalog, Category, Entry, FlowKind, Kind } from "../src/lib/types";
import { effect } from "../src/lib/kinds";
import { CATALOG, seed, stamp, type Shot } from "./seed";

const USER = "dzmfg";
const MAX = 99_999_999_999;

class Fail extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

const json = (status: number, body?: unknown, headers: Record<string, string> = {}) =>
  new Response(body === undefined ? null : JSON.stringify(body), { status, headers: { "Content-Type": "application/json", ...headers } });

export function createMock(): (req: Request) => Promise<Response | null> {
  const catalog: Catalog = structuredClone(CATALOG);
  const data = seed();
  let entries: Entry[] = data.entries;
  const shots: Record<string, Shot> = data.recognitions;
  const settings = { cycle_day: 1, budget_cents: 300000 as number | null };
  // 会话：Cookie 值是「第几代密码」，改密码换代，旧 Cookie 失效（真后端是签名，效果一样）。
  let password = "demo";
  let generation = 1;
  const session = () => ({ "Set-Cookie": `micjz_session=${generation}; HttpOnly; SameSite=Strict; Path=/; Max-Age=2592000` });

  const time = (s: unknown) => {
    const t = typeof s === "string" ? Date.parse(s) : NaN;
    if (Number.isNaN(t)) throw new Fail(400, "时间格式不对");
    return stamp(new Date(t));
  };
  const text = (s: unknown) => {
    if (s === null) return null;
    if (typeof s !== "string") throw new Fail(400, "文本格式不对");
    const t = s.trim();
    if (t.length > 200) throw new Fail(400, "文字太长，最多 200 个字");
    return t || null;
  };
  const account = (id: number) => {
    const a = catalog.accounts.find((a) => a.id === id);
    if (!a) throw new Fail(404, "账户不存在");
    return a;
  };
  const balance = (id: number) => entries.reduce((s, e) => s + effect(e, id), 0);

  /** §5.1：整笔校验，记一笔和改共用。 */
  function check(e: Entry) {
    const signed = e.kind === "adjust" || e.kind === "expense";
    if (!Number.isSafeInteger(e.amount_cents) || Math.abs(e.amount_cents) > MAX || (signed ? e.amount_cents === 0 : e.amount_cents <= 0))
      throw new Fail(400, "金额不对");
    if (e.kind === "expense" || e.kind === "income") {
      const c = catalog.categories.find((c) => c.id === e.category_id);
      if (!c) throw new Fail(409, "分类已不存在，请刷新后重选");
      if (c.kind !== e.kind) throw new Fail(400, "分类和类型不匹配");
    } else if (e.category_id !== null) throw new Fail(400, "转账、平账没有分类");
    if (!catalog.accounts.some((a) => a.id === e.account_id)) throw new Fail(409, "账户已不存在，请刷新后重选");
    if (e.kind === "transfer") {
      if (e.to_account_id === e.account_id) throw new Fail(400, "转出、转入不能是同一个账户");
      if (!catalog.accounts.some((a) => a.id === e.to_account_id)) throw new Fail(409, "账户已不存在，请刷新后重选");
    } else if (e.to_account_id !== null) throw new Fail(400, "只有转账有转入账户");
    if (e.tag_ids.some((t) => !catalog.tags.some((x) => x.id === t))) throw new Fail(409, "标签已不存在，请刷新后重选");
    e.tag_ids = [...new Set(e.tag_ids)].sort((a, b) => a - b);
  }

  const newest = (a: Entry, b: Entry) => b.occurred_at.localeCompare(a.occurred_at) || b.id.localeCompare(a.id);
  const newId = () => `01${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`;

  function insert(fields: Omit<Entry, "id" | "recognition_id" | "created_at">, id = newId()): Entry {
    const e: Entry = { ...fields, id, recognition_id: null, created_at: stamp(new Date()) };
    check(e);
    entries.push(e);
    entries.sort(newest);
    return e;
  }

  function accountInput(b: Record<string, unknown>, self?: number): Omit<Account, "id"> {
    const name = typeof b.name === "string" ? b.name.trim() : "";
    if (!name || /[/\n]/.test(name)) throw new Fail(400, "账户名不能为空，也不能含 / 和换行");
    if (catalog.accounts.some((a) => a.name === name && a.id !== self)) throw new Fail(409, "已经有同名账户");
    const brand = text(b.brand ?? null);
    const card = b.card as Account["card"];
    if (card && !/^\d{4,}$/.test(card.number.replace(/\s/g, ""))) throw new Fail(400, "卡号至少 4 位数字");
    return { name, brand, card: card ? { type: card.type, number: card.number.replace(/\s/g, "") } : null, external: b.external === true };
  }

  const nameOf = (v: unknown) => {
    const name = typeof v === "string" ? v.trim() : "";
    if (!name || /[/\n]/.test(name)) throw new Fail(400, "名称不能为空，也不能含 / 和换行");
    return name;
  };
  const category = (id: number) => {
    const c = catalog.categories.find((c) => c.id === id);
    if (!c) throw new Fail(404, "分类不存在");
    return c;
  };
  const isFallback = (c: Category) => c.key === (c.kind === "expense" ? "other" : "other_in");
  const sameGroup = (a: Category, b: Pick<Category, "kind" | "parent_id">) => a.kind === b.kind && a.parent_id === b.parent_id;
  /** 分类的整体写入，新建、修改共用；新建排同级末尾（数组顺序即优先级）。 */
  function putCategory(b: Record<string, unknown>, old?: Category) {
    const kind = old?.kind ?? (b.kind as FlowKind);
    if (kind !== "expense" && kind !== "income") throw new Fail(400, "新建分类须给 kind");
    if (old && b.kind !== undefined && b.kind !== kind) throw new Fail(400, "分类的类型不能改");
    const name = nameOf(b.name);
    const parent_id = (b.parent_id as number | null) ?? null;
    let key = typeof b.key === "string" ? b.key.trim() : "";
    if (key && !/^[a-z0-9_]+$/.test(key)) throw new Fail(400, `key 须为小写英文、数字或下划线：${key}`);
    if (!key && old) key = old.key;
    if (old && isFallback(old) && (key !== old.key || parent_id !== null)) throw new Fail(409, "兜底分类必须保持一级，key 不能改");
    if (parent_id !== (old?.parent_id ?? null) && parent_id !== null) {
      const p = catalog.categories.find((c) => c.id === parent_id);
      if (!p || p.kind !== kind || p.parent_id !== null || p === old) throw new Fail(409, "上级须是同类型的一级分类");
      const subs = old ? catalog.categories.filter((c) => c.parent_id === old.id).length : 0;
      if (subs) throw new Fail(409, `下面还有 ${subs} 个二级，不能挪到别的一级下面`);
    }
    if (catalog.categories.some((c) => c !== old && sameGroup(c, { kind, parent_id }) && c.name === name)) throw new Fail(409, `同级已有分类「${name}」`);
    const owner = key && catalog.categories.find((c) => c !== old && c.key === key);
    if (owner) throw new Fail(409, `key ${key} 已被「${owner.name}」使用`);
    if (old && old.parent_id === parent_id) return Object.assign(old, { name, key });
    const id = old?.id ?? Math.max(0, ...catalog.categories.map((c) => c.id)) + 1;
    const next: Category = { id, kind, parent_id, name, key: key || `c${id}` };
    catalog.categories = [...catalog.categories.filter((c) => c !== old), next];
  }

  const earliestEntry = () => entries.reduce<string | null>((m, e) => (m === null || Date.parse(e.occurred_at) < Date.parse(m) ? e.occurred_at : m), null);
  const fullSettings = () => ({ ...settings, version: "mock", entry_count: entries.length, earliest: earliestEntry() });

  async function route(method: string, path: string[], q: URLSearchParams, b: Record<string, unknown>): Promise<Response> {
    const [head, id, sub] = path;
    const key = `${method} ${head}${id === undefined ? "" : "/:id"}${sub ? `/${sub}` : ""}`;
    switch (key) {
      case "POST logout":
        return json(204, undefined, { "Set-Cookie": "micjz_session=; Max-Age=0; Path=/" });
      case "PUT password": {
        if (b.current !== password) throw new Fail(400, "当前密码不正确");
        if (typeof b.new !== "string" || [...b.new].length < 8) throw new Fail(400, "新密码至少 8 位");
        password = b.new;
        generation += 1;
        return json(204, undefined, session());
      }
      case "GET catalog":
        return json(200, catalog);
      case "GET entries": {
        const from = Date.parse(time(q.get("from")));
        const to = Date.parse(time(q.get("to")));
        if (from >= to) throw new Fail(400, "from 要早于 to");
        const list = entries.filter((e) => Date.parse(e.occurred_at) >= from && Date.parse(e.occurred_at) < to);
        return json(200, { entries: list, earliest: earliestEntry() });
      }
      case "GET balances":
        return json(200, { balances: catalog.accounts.map((a) => ({ account_id: a.id, balance_cents: balance(a.id) })) });
      case "GET settings":
        return json(200, fullSettings());
      case "GET update":
        return json(200, { state: "unavailable" });
      case "PUT settings": {
        const day = b.cycle_day as number;
        const budget = b.budget_cents as number | null;
        if (!Number.isInteger(day) || day === 0 || day === -1 || Math.abs(day) > 28) throw new Fail(400, "记账周期是 1～28 或 -28～-2");
        if (budget !== null && !(Number.isSafeInteger(budget) && budget > 0 && budget <= MAX)) throw new Fail(400, "预算要大于 0");
        Object.assign(settings, { cycle_day: day, budget_cents: budget });
        return json(200, fullSettings());
      }
      case "GET recognitions/:id": {
        const s = shots[id];
        if (!s) throw new Fail(404, "识别记录不存在");
        const { image: _, ...rest } = s;
        return json(200, rest);
      }
      case "GET recognitions/:id/image": {
        const s = shots[id];
        if (!s) throw new Fail(404, "截图不存在");
        return new Response(s.image, { headers: { "Content-Type": "image/svg+xml", "Cache-Control": "private, max-age=31536000, immutable" } });
      }
      case "POST entries": {
        const id = b.id as string;
        if (typeof id !== "string" || !/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(id)) throw new Fail(400, "id 须为 UUIDv7");
        const existing = entries.find((e) => e.id === id);
        if (existing) return json(200, existing);
        if (!["expense", "income", "transfer"].includes(b.kind as string)) throw new Fail(400, "类型只能是支出、收入、转账");
        const e = insert(
          {
            kind: b.kind as Kind,
            amount_cents: b.amount_cents as number,
            category_id: b.category_id as number | null,
            account_id: b.account_id as number,
            to_account_id: b.to_account_id as number | null,
            merchant: text(b.merchant),
            occurred_at: time(b.occurred_at),
            note: text(b.note),
            tag_ids: (b.tag_ids as number[]) ?? [],
          },
          id,
        );
        return json(201, e);
      }
      case "PATCH entries/:id": {
        const old = entries.find((e) => e.id === id);
        if (!old) throw new Fail(404, "账目不存在");
        if (!Object.keys(b).length) throw new Fail(400, "没有要改的字段");
        for (const [k, v] of Object.entries(b)) if (v === null && k !== "merchant" && k !== "note") throw new Fail(400, `${k} 不能为空`);
        const next: Entry = { ...old, ...(b as Partial<Entry>) };
        if ("merchant" in b) next.merchant = text(b.merchant);
        if ("note" in b) next.note = text(b.note);
        if ("occurred_at" in b) next.occurred_at = time(b.occurred_at);
        if (next.kind !== old.kind) {
          const flow = (k: Kind): k is FlowKind => k === "expense" || k === "income";
          if (!flow(old.kind) || !flow(next.kind)) throw new Fail(400, "只能在支出和收入之间换类型");
          if (!("category_id" in b)) throw new Fail(400, "换类型要同时给新分类");
        }
        check(next);
        entries = entries.map((e) => (e.id === id ? next : e)).sort(newest);
        return json(200, next);
      }
      case "DELETE entries/:id": {
        if (!entries.some((e) => e.id === id)) throw new Fail(404, "账目不存在");
        entries = entries.filter((e) => e.id !== id);
        return json(204);
      }
      case "POST accounts": {
        const a = { id: Math.max(0, ...catalog.accounts.map((a) => a.id)) + 1, ...accountInput(b) };
        catalog.accounts.push(a);
        return json(201, a);
      }
      case "PUT accounts/:id": {
        const a = account(Number(id));
        Object.assign(a, accountInput(b, a.id));
        return json(200, a);
      }
      case "DELETE accounts/:id": {
        const a = account(Number(id));
        const used = entries.filter((e) => e.account_id === a.id || e.to_account_id === a.id).length;
        if (used) throw new Fail(409, `还有 ${used} 笔账目在用，不能删除`);
        if (catalog.accounts.length === 1) throw new Fail(409, "至少保留一个账户");
        catalog.accounts = catalog.accounts.filter((x) => x !== a);
        return json(204);
      }
      case "POST accounts/:id/move": {
        const a = account(Number(id));
        const list = catalog.accounts.filter((x) => x !== a);
        list.splice(Math.max(0, Math.min(list.length, b.to as number)), 0, a);
        catalog.accounts = list;
        return json(200, { accounts: list });
      }
      case "POST categories":
        putCategory(b);
        return json(201, { categories: catalog.categories });
      case "PUT categories/:id":
        putCategory(b, category(Number(id)));
        return json(200, { categories: catalog.categories });
      case "DELETE categories/:id": {
        const c = category(Number(id));
        if (isFallback(c)) throw new Fail(409, "兜底分类不能删");
        const subs = catalog.categories.filter((x) => x.parent_id === c.id).length;
        if (subs) throw new Fail(409, `下面还有 ${subs} 个二级，先删它们`);
        const used = entries.filter((e) => e.category_id === c.id).length;
        if (used) throw new Fail(409, `还有 ${used} 笔账目在用，不能删除`);
        catalog.categories = catalog.categories.filter((x) => x !== c);
        return json(204);
      }
      case "POST categories/:id/move": {
        const c = category(Number(id));
        const sibs = catalog.categories.filter((x) => x !== c && sameGroup(x, c));
        sibs.splice(Math.max(0, Math.min(sibs.length, b.to as number)), 0, c);
        let i = 0;
        catalog.categories = catalog.categories.map((x) => (sameGroup(x, c) ? sibs[i++] : x));
        return json(200, { categories: catalog.categories });
      }
      case "POST tags":
      case "PUT tags/:id": {
        const name = nameOf(b.name);
        const t = id === undefined ? undefined : catalog.tags.find((t) => t.id === Number(id));
        if (id !== undefined && !t) throw new Fail(404, "标签不存在");
        if (catalog.tags.some((x) => x !== t && x.name === name)) throw new Fail(409, `已经有标签「${name}」`);
        if (t) t.name = name;
        else catalog.tags.push({ id: Math.max(0, ...catalog.tags.map((t) => t.id)) + 1, name });
        return json(t ? 200 : 201, { tags: catalog.tags });
      }
      case "DELETE tags/:id": {
        const t = catalog.tags.find((t) => t.id === Number(id));
        if (!t) throw new Fail(404, "标签不存在");
        const used = entries.filter((e) => e.tag_ids.includes(t.id)).length;
        if (used) throw new Fail(409, `还有 ${used} 笔账目在用，不能删除`);
        catalog.tags = catalog.tags.filter((x) => x !== t);
        return json(204);
      }
      case "POST accounts/:id/adjust": {
        const a = account(Number(id));
        if (a.external) throw new Fail(400, "外部账户不记余额，不能平账");
        const target = b.balance_cents as number;
        if (!Number.isSafeInteger(target) || Math.abs(target) > MAX) throw new Fail(400, "余额不对");
        const diff = target - balance(a.id);
        const at = b.occurred_at === undefined ? stamp(new Date()) : time(b.occurred_at);
        const entry = diff
          ? insert({
              kind: "adjust",
              amount_cents: diff,
              category_id: null,
              account_id: a.id,
              to_account_id: null,
              merchant: null,
              occurred_at: at,
              note: null,
              tag_ids: [],
            })
          : null;
        return json(200, { entry, balance_cents: target });
      }
      case "GET export.csv":
        return new Response("﻿时间,类型,金额\n", {
          headers: { "Content-Type": "text/csv; charset=utf-8", "Content-Disposition": 'attachment; filename="micjz-mock.csv"' },
        });
    }
    throw new Fail(404, "没有这个接口");
  }

  return async (req) => {
    const url = new URL(req.url);
    const at = url.pathname.indexOf("/api/");
    if (at < 0) return null;
    const path = url.pathname
      .slice(at + 5)
      .split("/")
      .map(decodeURIComponent);
    // 模拟网络往返，让加载中、提交中的状态看得见。
    await new Promise((r) => setTimeout(r, 120));
    try {
      const b = req.method === "GET" || req.method === "DELETE" ? {} : await req.json().catch(() => ({}));
      if (path[0] === "login" && req.method === "POST") {
        if (b.username !== USER || b.password !== password) throw new Fail(401, "用户名或密码不正确");
        return json(204, undefined, session());
      }
      const cookie = req.headers.get("cookie") ?? "";
      if (!cookie.split(/;\s*/).includes(`micjz_session=${generation}`)) throw new Fail(401, "未登录或登录已过期");
      return await route(req.method, path, url.searchParams, b);
    } catch (e) {
      if (e instanceof Fail) return json(e.status, { error: e.message });
      return json(500, { error: String(e) });
    }
  };
}
