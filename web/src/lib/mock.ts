// 原型用的假数据：分类、账户、标签照 seed.sql，账目随机但固定。
import type { Catalog, Category, Entry, FlowKind, Recognition } from "./types";
import { effect } from "./kinds";
import { money } from "./format";

type Node = [name: string, key: string];
const TREE: [FlowKind, Node, Node[]][] = [
  ["expense", ["🍜 餐饮", "food"], [["🍱 三餐", "meal"], ["☕️ 咖啡饮品", "coffee"], ["🍓 零食水果", "snack"], ["🍲 聚餐", "dining"]]],
  ["expense", ["🚇 交通", "transport"], [["🚌 公交地铁", "transit"], ["🚕 打车", "taxi"], ["⛽️ 加油充电", "fuel"], ["🅿️ 停车", "parking"], ["✈️ 火车飞机", "rail"]]],
  ["expense", ["🛍️ 购物", "shopping"], [["🧻 日用", "daily"], ["👕 服饰", "clothing"], ["🎧 数码", "digital"], ["🛋️ 家居", "home"]]],
  ["expense", ["🏠 居住", "housing"], [["🔑 房租", "rent"], ["💡 水电燃气", "utilities"], ["📶 话费网费", "telecom"], ["🏢 物业", "property"]]],
  ["expense", ["🎮 娱乐", "fun"], [["📺 会员订阅", "subscription"], ["🕹️ 游戏", "game"], ["🎬 电影演出", "show"], ["🧳 旅行", "travel"]]],
  ["expense", ["🏥 医疗", "medical"], [["💊 药品", "medicine"], ["🩺 门诊", "clinic"]]],
  ["expense", ["🎁 人情", "social"], [["🧧 红包", "redpacket"], ["🎀 礼物", "gift"], ["🍻 请客", "treat"]]],
  ["expense", ["✨ 其他", "other"], []],
  ["income", ["💰 工资", "salary"], []],
  ["income", ["🏆 奖金", "bonus"], []],
  ["income", ["🧧 红包", "redpacket_in"], []],
  ["income", ["↩️ 退款", "refund"], []],
  ["income", ["📈 理财", "investment"], []],
  ["income", ["✨ 其他", "other_in"], []],
];

const categories: Category[] = [];
for (const [kind, [name, key], subs] of TREE) {
  const parent = categories.length + 1;
  categories.push({ id: parent, kind, parent_id: null, name, key });
  for (const [name, key] of subs) categories.push({ id: categories.length + 1, kind, parent_id: parent, name, key });
}

export const catalog: Catalog = {
  categories,
  accounts: [
    { id: 1, name: "微信", brand: "微信", card: null },
    { id: 2, name: "工资卡", brand: "招商银行", card: { type: "debit", number: "6214888800001234" } },
    { id: 3, name: "招行信用卡", brand: "招商银行", card: { type: "credit", number: "6225888800005678" } },
    { id: 4, name: "支付宝", brand: "支付宝", card: null },
  ],
  tags: [
    { id: 1, name: "恋爱" },
    { id: 2, name: "出差" },
    { id: 3, name: "家庭" },
  ],
};

const byKey = (key: string) => categories.find((c) => c.key === key)!.id;
const [WECHAT, SALARY_CARD, CREDIT, ALIPAY] = [1, 2, 3, 4];

type Seed = [merchant: string, key: string, min: number, max: number, note?: string];

const SPEND: Seed[] = [
  ["瑞幸咖啡", "coffee", 990, 3250, "生椰拿铁"],
  ["星巴克", "coffee", 3000, 4500, "燕麦拿铁 大杯"],
  ["美团外卖", "meal", 1800, 5200],
  ["麦当劳", "meal", 2500, 4800, "麦辣鸡腿堡套餐"],
  ["盒马鲜生", "snack", 2200, 9800],
  ["海底捞火锅", "dining", 26000, 48000],
  ["上海地铁", "transit", 300, 700],
  ["滴滴出行", "taxi", 1500, 6800],
  ["中国石化", "fuel", 25000, 40000],
  ["京东", "digital", 9900, 129900, "AirPods 保护壳"],
  ["优衣库", "clothing", 9900, 39900],
  ["7-ELEVEn", "daily", 800, 3500],
  ["国家电网", "utilities", 8000, 21000],
  ["中国移动", "telecom", 5800, 12800],
  ["哔哩哔哩", "subscription", 1500, 2500, "大会员连续包月"],
  ["万达影城", "show", 4500, 9000],
  ["叮当快药", "medicine", 2300, 8900],
];

/** 各账户此刻的余额，分；信用卡是欠款，为负。期初平账按它倒推。 */
const BALANCES: Record<number, number> = { [WECHAT]: 328640, [SALARY_CARD]: 26821055, [CREDIT]: -412780, [ALIPAY]: 1263000 };

function rng(seed: number) {
  return () => ((seed = (seed * 1664525 + 1013904223) >>> 0) / 2 ** 32);
}

/** 本地时区的 ISO 8601。 */
function iso(d: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  const off = -d.getTimezoneOffset();
  const tz = `${off >= 0 ? "+" : "-"}${p(Math.floor(Math.abs(off) / 60))}:${p(Math.abs(off) % 60)}`;
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}:00${tz}`;
}

type Draft = Omit<Entry, "id" | "recognition_id" | "created_at" | "occurred_at" | "to_account_id" | "merchant" | "note" | "tag_ids"> &
  Partial<Pick<Entry, "to_account_id" | "merchant" | "note" | "tag_ids">>;

export function seed(now = new Date()): { entries: Entry[]; recognitions: Record<string, Recognition> } {
  const r = rng(42);
  const entries: Entry[] = [];
  const recognitions: Record<string, Recognition> = {};
  const pick = <T>(xs: T[]) => xs[Math.floor(r() * xs.length)];
  const id = (at: Date) => `01${at.getTime().toString(36)}${Math.floor(r() * 1e6).toString(36)}`;

  /** 手填的账没有截图；`first` 给了就是截图来的，`first` 是快捷指令里第一眼看到的草稿相对最终值的差异。 */
  function add(at: Date, d: Draft, first?: Partial<Recognition["first"]>) {
    const e: Entry = { to_account_id: null, merchant: null, note: null, tag_ids: [], ...d, id: id(at), recognition_id: null, occurred_at: iso(at), created_at: iso(at) };
    if (first) {
      e.recognition_id = id(at);
      const shown = { ...e, ...first };
      recognitions[e.recognition_id] = { id: e.recognition_id, image: screenshot(shown), first: shown };
    }
    entries.push(e);
  }

  for (let day = 0; day < 400; day++) {
    const date = new Date(now);
    date.setDate(now.getDate() - day);
    const n = Math.floor(r() * 4);
    for (let i = 0; i < n; i++) {
      const [merchant, key, min, max, note] = pick(SPEND);
      const at = new Date(date);
      at.setHours(8 + Math.floor(r() * 14), Math.floor(r() * 60));
      if (day === 0 && at > now) at.setTime(now.getTime() - (i + 1) * 47 * 60000);
      const amount_cents = Math.round((min + r() * (max - min)) / 10) * 10;
      const account_id = pick([WECHAT, WECHAT, SALARY_CARD, CREDIT, ALIPAY]);
      const tag_ids = r() < 0.12 ? [1] : [];
      const d: Draft = { kind: "expense", amount_cents, category_id: byKey(key), account_id, merchant, note: note ?? null, tag_ids };
      // 大多截图来的；少数几笔模拟用户改过：模型分错类、没认出账户（服务器补了默认账户）、没认出金额。
      const roll = r();
      if (roll < 0.06) add(at, d);
      else if (roll < 0.14) add(at, d, { category_id: byKey("other") });
      else if (roll < 0.2) add(at, d, { account_id: WECHAT });
      else if (roll < 0.23) add(at, d, { amount_cents: null });
      else add(at, d, {});
    }
    const at = (h: number, m: number) => {
      const t = new Date(date);
      t.setHours(h, m);
      return t;
    };
    if (date.getDate() === 10) {
      add(at(10, 2), { kind: "income", amount_cents: 2350000, category_id: byKey("salary"), account_id: SALARY_CARD, merchant: "某某科技有限公司" });
    }
    if (date.getDate() === 5) {
      const amount_cents = Math.round(150000 + r() * 100000);
      add(at(20, 15), { kind: "transfer", amount_cents, category_id: null, account_id: SALARY_CARD, to_account_id: CREDIT, note: "还信用卡" });
    }
    if (day === 3) {
      add(at(21, 30), { kind: "adjust", amount_cents: -1280, category_id: null, account_id: WECHAT, note: "对账，零钱少了" });
    }
  }

  // 每个账户的第一笔平账是期初余额：按此刻余额倒推。
  const start = new Date(now);
  start.setDate(now.getDate() - 400);
  for (const a of catalog.accounts) {
    const opening = BALANCES[a.id] - entries.reduce((s, e) => s + effect(e, a.id), 0);
    if (opening) add(start, { kind: "adjust", amount_cents: opening, category_id: null, account_id: a.id, note: "期初余额" });
  }

  entries.sort((a, b) => b.occurred_at.localeCompare(a.occurred_at));
  return { entries, recognitions };
}

/** 仿支付成功页的截图（SVG）。 */
function screenshot(f: Recognition["first"]): string {
  const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;");
  const amount = money(f.amount_cents ?? 0).replace("¥", "");
  const account = catalog.accounts.find((a) => a.id === f.account_id)?.name ?? "";
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 390 844">
<rect width="390" height="844" fill="#EDEDED"/>
<rect y="0" width="390" height="96" fill="#EDEDED"/>
<text x="195" y="78" font-size="17" text-anchor="middle" font-family="-apple-system" fill="#111">支付成功</text>
<circle cx="195" cy="190" r="34" fill="#07C160"/>
<path d="M178 190 l12 12 l22 -24" stroke="#fff" stroke-width="6" fill="none" stroke-linecap="round" stroke-linejoin="round"/>
<text x="195" y="262" font-size="17" text-anchor="middle" font-family="-apple-system" fill="#111">${esc(f.merchant ?? "")}</text>
<text x="195" y="318" font-size="44" font-weight="600" text-anchor="middle" font-family="-apple-system" fill="#111">¥${amount}</text>
<rect x="16" y="370" width="358" height="168" rx="10" fill="#fff"/>
<text x="32" y="410" font-size="15" font-family="-apple-system" fill="#888">支付方式</text>
<text x="358" y="410" font-size="15" text-anchor="end" font-family="-apple-system" fill="#111">${esc(account === "微信" ? "零钱" : account)}</text>
<text x="32" y="454" font-size="15" font-family="-apple-system" fill="#888">商品</text>
<text x="358" y="454" font-size="15" text-anchor="end" font-family="-apple-system" fill="#111">${esc(f.note ?? "—")}</text>
<text x="32" y="498" font-size="15" font-family="-apple-system" fill="#888">交易单号</text>
<text x="358" y="498" font-size="15" text-anchor="end" font-family="-apple-system" fill="#111">4200001988202611</text>
<rect x="120" y="700" width="150" height="44" rx="8" fill="#07C160"/>
<text x="195" y="728" font-size="17" text-anchor="middle" font-family="-apple-system" fill="#fff">完成</text>
</svg>`;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}
