import type { Entry, FieldKey } from "./types";
import type { IconName } from "../ui/Icon.svelte";
import { categoryLabel, dateTime, money } from "./format";

export type Editor = "number" | "kind" | "category" | "account" | "text" | "date" | "tags";

export interface FieldDef {
  key: FieldKey;
  label: string;
  /** 和快捷指令主菜单一致；软糖等主题用。 */
  emoji: string;
  /** 线条图标；晶彩的字段行用。 */
  icon: IconName;
  editor: Editor;
  show: (e: Entry) => string;
}

const orNone = (v: string | null) => v ?? "";

// 字段的唯一定义：顺序、名称、emoji 与快捷指令主菜单一致。
export const FIELDS: FieldDef[] = [
  { key: "amount_cents", label: "金额", emoji: "💰", icon: "yen", editor: "number", show: (e) => money(e.amount_cents) },
  { key: "kind", label: "类型", emoji: "↔️", icon: "arrows", editor: "kind", show: (e) => (e.kind === "expense" ? "支出" : "收入") },
  { key: "category", label: "分类", emoji: "🗂️", icon: "grid", editor: "category", show: (e) => categoryLabel(e) },
  { key: "account", label: "账户", emoji: "💳", icon: "card", editor: "account", show: (e) => e.account },
  { key: "merchant", label: "商家", emoji: "🏪", icon: "store", editor: "text", show: (e) => orNone(e.merchant) },
  { key: "occurred_at", label: "时间", emoji: "🕒", icon: "clock", editor: "date", show: (e) => dateTime(e.occurred_at) },
  { key: "tags", label: "标签", emoji: "🏷️", icon: "tag", editor: "tags", show: (e) => e.tags.join("、") },
  { key: "note", label: "备注", emoji: "📝", icon: "note", editor: "text", show: (e) => orNone(e.note) },
];

export const field = (key: FieldKey) => FIELDS.find((f) => f.key === key)!;

/** 用户改过的字段：模型给的值和最终值不同。 */
export function changes(e: Entry): { def: FieldDef; from: string; to: string }[] {
  const out = [];
  for (const def of FIELDS) {
    const before = { ...e, ...originFor(e, def.key) } as Entry;
    const from = def.show(before);
    const to = def.show(e);
    if (from !== to) out.push({ def, from: from || "未识别", to: to || "空" });
  }
  return out;
}

function originFor(e: Entry, key: FieldKey): Partial<Entry> {
  const o = e.origin;
  switch (key) {
    case "category":
      return { category: o.category ?? "其他", subcategory: o.subcategory ?? null, kind: o.kind ?? e.kind };
    case "amount_cents":
      return { amount_cents: o.amount_cents ?? NaN };
    default:
      return { [key]: o[key] ?? (key === "tags" ? [] : null) } as Partial<Entry>;
  }
}
