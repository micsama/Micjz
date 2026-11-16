// 原型用的假数据：分类、账户、标签照 seed.sql，账目随机但固定。
import type { Catalog, Entry, Fields, Kind } from "./types";
import { money } from "./format";

export const catalog: Catalog = {
  expense: [
    { name: "餐饮", subs: ["三餐", "咖啡饮品", "零食水果", "聚餐"] },
    { name: "交通", subs: ["公交地铁", "打车", "加油充电", "停车", "火车飞机"] },
    { name: "购物", subs: ["日用", "服饰", "数码", "家居"] },
    { name: "居住", subs: ["房租", "水电燃气", "话费网费", "物业"] },
    { name: "娱乐", subs: ["会员订阅", "游戏", "电影演出", "旅行"] },
    { name: "医疗", subs: ["药品", "门诊"] },
    { name: "人情", subs: ["红包", "礼物", "请客"] },
    { name: "其他", subs: [] },
  ],
  income: [
    { name: "工资", subs: [] },
    { name: "奖金", subs: [] },
    { name: "红包", subs: [] },
    { name: "退款", subs: [] },
    { name: "理财", subs: [] },
    { name: "其他", subs: [] },
  ],
  accounts: [
    { name: "微信", brand: "微信", card: null },
    { name: "工资卡", brand: "招商银行", card: { type: "debit", number: "6214888800001234" } },
    { name: "招行信用卡", brand: "招商银行", card: { type: "credit", number: "6225888800005678" } },
    { name: "支付宝", brand: "支付宝", card: null },
  ],
  tags: ["恋爱", "出差", "家庭"],
};

type Seed = [merchant: string, cat: string, sub: string | null, min: number, max: number, note?: string];

const SPEND: Seed[] = [
  ["瑞幸咖啡", "餐饮", "咖啡饮品", 990, 3250, "生椰拿铁"],
  ["星巴克", "餐饮", "咖啡饮品", 3000, 4500, "燕麦拿铁 大杯"],
  ["美团外卖", "餐饮", "三餐", 1800, 5200],
  ["麦当劳", "餐饮", "三餐", 2500, 4800, "麦辣鸡腿堡套餐"],
  ["盒马鲜生", "餐饮", "零食水果", 2200, 9800],
  ["海底捞火锅", "餐饮", "聚餐", 26000, 48000],
  ["上海地铁", "交通", "公交地铁", 300, 700],
  ["滴滴出行", "交通", "打车", 1500, 6800],
  ["中国石化", "交通", "加油充电", 25000, 40000],
  ["京东", "购物", "数码", 9900, 129900, "AirPods 保护壳"],
  ["优衣库", "购物", "服饰", 9900, 39900],
  ["7-ELEVEn", "购物", "日用", 800, 3500],
  ["国家电网", "居住", "水电燃气", 8000, 21000],
  ["中国移动", "居住", "话费网费", 5800, 12800],
  ["哔哩哔哩", "娱乐", "会员订阅", 1500, 2500, "大会员连续包月"],
  ["万达影城", "娱乐", "电影演出", 4500, 9000],
  ["叮当快药", "医疗", "药品", 2300, 8900],
];

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

function entry(r: () => number, at: Date, f: Omit<Fields, "occurred_at">, origin: Partial<Fields>): Entry {
  const id = `01${at.getTime().toString(36)}${Math.floor(r() * 1e6).toString(36)}`;
  const fields = { ...f, occurred_at: iso(at) };
  return { ...fields, id, created_at: iso(at), origin: { ...fields, ...origin }, image: screenshot({ ...fields, ...origin }) };
}

/** 各账户此刻的余额，分；信用卡是欠款，为负。 */
export const balances: Record<string, number> = { 微信: 328640, 工资卡: 4821055, 招行信用卡: -412780, 支付宝: 1263000 };

export function entries(now = new Date()): Entry[] {
  const r = rng(42);
  const out: Entry[] = [];
  const pick = <T>(xs: T[]) => xs[Math.floor(r() * xs.length)];
  for (let day = 0; day < 400; day++) {
    const date = new Date(now);
    date.setDate(now.getDate() - day);
    const n = Math.floor(r() * 4);
    for (let i = 0; i < n; i++) {
      const [merchant, category, sub, min, max, note] = pick(SPEND);
      const at = new Date(date);
      at.setHours(8 + Math.floor(r() * 14), Math.floor(r() * 60));
      if (day === 0 && at > now) at.setTime(now.getTime() - (i + 1) * 47 * 60000);
      const amount = Math.round((min + r() * (max - min)) / 10) * 10;
      const account = pick(["微信", "微信", "工资卡", "招行信用卡", "支付宝"]);
      const tags = r() < 0.12 ? ["恋爱"] : [];
      const f = { kind: "expense" as Kind, amount_cents: amount, category, subcategory: sub, account, merchant, note: note ?? null, tags };
      // 少数几笔模拟用户改过：模型分错类、没认出账户。
      const roll = r();
      const origin: Partial<Fields> =
        roll < 0.1 ? { category: "其他", subcategory: null } : roll < 0.16 ? { account: "微信" } : {};
      out.push(entry(r, at, f, origin));
    }
    if (date.getDate() === 10) {
      const at = new Date(date);
      at.setHours(10, 2);
      const f = { kind: "income" as Kind, amount_cents: 2350000, category: "工资", subcategory: null, account: "工资卡", merchant: "某某科技有限公司", note: null, tags: [] };
      out.push(entry(r, at, f, {}));
    }
  }
  return out.sort((a, b) => b.occurred_at.localeCompare(a.occurred_at));
}

/** 仿支付成功页的截图（SVG）。 */
function screenshot(f: Partial<Fields>): string {
  const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;");
  const amount = money(f.amount_cents ?? 0).replace("¥", "");
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
<text x="358" y="410" font-size="15" text-anchor="end" font-family="-apple-system" fill="#111">${esc(f.account === "微信" ? "零钱" : (f.account ?? ""))}</text>
<text x="32" y="454" font-size="15" font-family="-apple-system" fill="#888">商品</text>
<text x="358" y="454" font-size="15" text-anchor="end" font-family="-apple-system" fill="#111">${esc(f.note ?? "—")}</text>
<text x="32" y="498" font-size="15" font-family="-apple-system" fill="#888">交易单号</text>
<text x="358" y="498" font-size="15" text-anchor="end" font-family="-apple-system" fill="#111">4200001988202611</text>
<rect x="120" y="700" width="150" height="44" rx="8" fill="#07C160"/>
<text x="195" y="728" font-size="17" text-anchor="middle" font-family="-apple-system" fill="#fff">完成</text>
</svg>`;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}
