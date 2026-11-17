// 和服务器对话的唯一出口：契约见 docs/blueprints/web-api.md。只管收发和把失败变成 ApiError，失败后怎么办由 store 决定。
import type { Account, Catalog, Category, Entry, Recognition, Tag } from "./types";

/** 服务器给的失败；`status` 为 0 是没连上。`message` 是给人看的那句，直接显示。 */
export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

async function call<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await send(method, path, body);
  return (res.status === 204 ? undefined : await res.json()) as T;
}

/** 成功的响应原样给出（需要看状态码时用），失败变成 ApiError。 */
async function send(method: string, path: string, body?: unknown): Promise<Response> {
  let res: Response;
  try {
    res = await fetch(`api/${path}`, {
      method,
      credentials: "same-origin",
      headers: body === undefined ? undefined : { "Content-Type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
      // 写请求在网页切走、关掉时也要送达（撤销期内的删除就在这时发出）。
      keepalive: method !== "GET",
    });
  } catch {
    throw new ApiError(0, "连不上服务器，请检查网络");
  }
  if (res.ok) return res;
  const text = await res.json().then(
    (j: { error?: string }) => j.error,
    () => undefined,
  );
  throw new ApiError(res.status, text ?? `服务器出错（${res.status}）`);
}

export interface Settings {
  cycle_day: number;
  /** 每期支出预算；空为不设。 */
  budget_cents: number | null;
  version: string;
  entry_count: number;
}

/** 一键更新的进度，见 docs/blueprints/self-update.md。 */
export type UpdateStatus =
  | { state: "unavailable" }
  | { state: "idle" | "checking"; output: string }
  | { state: "pulling"; from: string; attempt: number; output: string }
  | { state: "building" | "restarting"; from: string; to: string; output: string }
  | { state: "up_to_date"; commit: string; output: string }
  | { state: "failed"; stage: string; error: string; output: string };

/** 新记一笔要给的字段；ID 由 `newEntryId` 生成并随请求带上，入账时刻由服务器生成。 */
export type EntryInput = Omit<Entry, "id" | "recognition_id" | "created_at">;

/** 手记一笔的 UUIDv7：打开记账面板时生成，重试沿用，服务器据此防重复。 */
export function newEntryId(): string {
  const b = crypto.getRandomValues(new Uint8Array(16));
  const ms = Date.now();
  for (let i = 0; i < 6; i++) b[i] = Math.floor(ms / 2 ** (8 * (5 - i))) & 0xff;
  b[6] = (b[6] & 0x0f) | 0x70;
  b[8] = (b[8] & 0x3f) | 0x80;
  const h = Array.from(b, (x) => x.toString(16).padStart(2, "0")).join("");
  return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
}
/** 改账目：只放要改的字段；`merchant`、`note` 给 `null` 是清空。 */
export type EntryPatch = Partial<EntryInput>;
export type AccountInput = Omit<Account, "id">;
/** 新建、修改分类；`kind` 只在新建时给，`key` 为空时新建自动生成、修改保持原值。 */
export type CategoryInput = Omit<Category, "id" | "kind" | "key"> & { kind?: Category["kind"]; key: string };

const enc = encodeURIComponent;

export const api = {
  login: (username: string, password: string) => call<void>("POST", "login", { username, password }),
  logout: () => call<void>("POST", "logout"),
  changePassword: (current: string, next: string) => call<void>("PUT", "password", { current, new: next }),

  catalog: () => call<Catalog>("GET", "catalog"),
  entries: (from: Date, to: Date) =>
    call<{ entries: Entry[]; earliest: string | null }>("GET", `entries?from=${enc(from.toISOString())}&to=${enc(to.toISOString())}`),
  balances: () => call<{ balances: { account_id: number; balance_cents: number }[] }>("GET", "balances"),
  settings: () => call<Settings>("GET", "settings"),
  recognition: (id: string) => call<Recognition>("GET", `recognitions/${enc(id)}`),

  /** 同一 `id` 已入账时服务器返回那一笔（200），`existed` 为真。 */
  addEntry: async (id: string, e: EntryInput) => {
    const res = await send("POST", "entries", { id, ...e });
    return { entry: (await res.json()) as Entry, existed: res.status === 200 };
  },
  patchEntry: (id: string, patch: EntryPatch) => call<Entry>("PATCH", `entries/${enc(id)}`, patch),
  deleteEntry: (id: string) => call<void>("DELETE", `entries/${enc(id)}`),
  adjust: (account: number, balance_cents: number) =>
    call<{ entry: Entry | null; balance_cents: number }>("POST", `accounts/${account}/adjust`, { balance_cents }),

  addAccount: (a: AccountInput) => call<Account>("POST", "accounts", a),
  putAccount: (id: number, a: AccountInput) => call<Account>("PUT", `accounts/${id}`, a),
  deleteAccount: (id: number) => call<void>("DELETE", `accounts/${id}`),
  moveAccount: (id: number, to: number) => call<{ accounts: Account[] }>("POST", `accounts/${id}/move`, { to }),

  addCategory: (c: CategoryInput) => call<{ categories: Category[] }>("POST", "categories", c),
  putCategory: (id: number, c: CategoryInput) => call<{ categories: Category[] }>("PUT", `categories/${id}`, c),
  deleteCategory: (id: number) => call<void>("DELETE", `categories/${id}`),
  moveCategory: (id: number, to: number) => call<{ categories: Category[] }>("POST", `categories/${id}/move`, { to }),

  addTag: (name: string) => call<{ tags: Tag[] }>("POST", "tags", { name }),
  putTag: (id: number, name: string) => call<{ tags: Tag[] }>("PUT", `tags/${id}`, { name }),
  deleteTag: (id: number) => call<void>("DELETE", `tags/${id}`),

  updateStatus: () => call<UpdateStatus>("GET", "update"),
  /** 202 没有正文，不能走 `call` 解析 JSON。 */
  startUpdate: async () => void (await send("POST", "update")),

  putSettings: (s: Pick<Settings, "cycle_day" | "budget_cents">) => call<Settings>("PUT", "settings", s),
};

/** 截图直接当 `<img src>`，Cookie 自动带上。 */
export const imageUrl = (recognition: string) => `api/recognitions/${enc(recognition)}/image`;
export const EXPORT_URL = "api/export.csv";
