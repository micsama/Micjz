// 假后端：照 docs/blueprints/web-api.md 在内存里实现 /api，`bun run demo` 不连真服务器时用。
// 只为前端开发和截图验收，规则按契约写但不求穷尽；以真后端为准。登录口令是 demo。
import type { Account, Catalog, Entry, FlowKind, Kind } from "../src/lib/types";
import { effect } from "../src/lib/kinds";
import { CATALOG, seed, stamp, type Shot } from "./seed";

const TOKEN = "demo";
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
    const signed = e.kind === "adjust";
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

  function insert(fields: Omit<Entry, "id" | "recognition_id" | "created_at">): Entry {
    const e: Entry = { ...fields, id: newId(), recognition_id: null, created_at: stamp(new Date()) };
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
    return { name, brand, card: card ? { type: card.type, number: card.number.replace(/\s/g, "") } : null };
  }

  const fullSettings = () => ({ ...settings, version: "mock", entry_count: entries.length });

  async function route(method: string, path: string[], q: URLSearchParams, b: Record<string, unknown>): Promise<Response> {
    const [head, id, sub] = path;
    const key = `${method} ${head}${id === undefined ? "" : "/:id"}${sub ? `/${sub}` : ""}`;
    switch (key) {
      case "POST logout":
        return json(204, undefined, { "Set-Cookie": "micjz=; Max-Age=0; Path=/" });
      case "GET catalog":
        return json(200, catalog);
      case "GET entries": {
        const from = Date.parse(time(q.get("from")));
        const to = Date.parse(time(q.get("to")));
        if (from >= to) throw new Fail(400, "from 要早于 to");
        const list = entries.filter((e) => Date.parse(e.occurred_at) >= from && Date.parse(e.occurred_at) < to);
        const earliest = entries.reduce<string | null>((m, e) => (m === null || e.occurred_at < m ? e.occurred_at : m), null);
        return json(200, { entries: list, earliest });
      }
      case "GET balances":
        return json(200, { balances: catalog.accounts.map((a) => ({ account_id: a.id, balance_cents: balance(a.id) })) });
      case "GET settings":
        return json(200, fullSettings());
      case "PUT settings": {
        const day = b.cycle_day as number;
        const budget = b.budget_cents as number | null;
        if (!Number.isInteger(day) || day < 1 || day > 28) throw new Fail(400, "记账周期是 1～28 号");
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
        if (!["expense", "income", "transfer"].includes(b.kind as string)) throw new Fail(400, "类型只能是支出、收入、转账");
        const e = insert({
          kind: b.kind as Kind,
          amount_cents: b.amount_cents as number,
          category_id: b.category_id as number | null,
          account_id: b.account_id as number,
          to_account_id: b.to_account_id as number | null,
          merchant: text(b.merchant),
          occurred_at: time(b.occurred_at),
          note: text(b.note),
          tag_ids: (b.tag_ids as number[]) ?? [],
        });
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
      case "POST accounts/:id/adjust": {
        const a = account(Number(id));
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
        if (b.token !== TOKEN) throw new Fail(401, "口令不对");
        return json(204, undefined, { "Set-Cookie": `micjz=${TOKEN}; HttpOnly; SameSite=Strict; Path=/; Max-Age=31536000` });
      }
      const cookie = req.headers.get("cookie") ?? "";
      if (!cookie.split(/;\s*/).includes(`micjz=${TOKEN}`) && req.headers.get("authorization") !== `Bearer ${TOKEN}`) throw new Fail(401, "请先登录");
      return await route(req.method, path, url.searchParams, b);
    } catch (e) {
      if (e instanceof Fail) return json(e.status, { error: e.message });
      return json(500, { error: String(e) });
    }
  };
}
