import type { Entry, FieldKey, Fields, Kind, Recognition } from "./types";
import type { IconName } from "../ui/Icon.svelte";
import { KIND_LABEL } from "./kinds";
import { dateTime, money } from "./format";
import { accountName, categoryLabel, tagNames } from "./view";

export type Editor = "number" | "kind" | "category" | "account" | "text" | "date" | "tags";

export interface FieldDef {
  key: FieldKey;
  label: string;
  /** 和快捷指令主菜单一致；软糖等主题用。 */
  emoji: string;
  /** 线条图标；晶彩的字段行用。 */
  icon: IconName;
  editor: Editor;
  /** 哪几类账目有这个字段。 */
  kinds: Kind[];
  /** 某类账目下换个叫法。 */
  as?: Partial<Record<Kind, string>>;
  show: (f: Fields) => string;
}

const orNone = (v: string | null) => v ?? "";
const FLOW: Kind[] = ["expense", "income"];
const ALL: Kind[] = ["expense", "income", "transfer", "adjust"];

// 字段的唯一定义：顺序、名称、emoji 与快捷指令主菜单一致。
export const FIELDS: FieldDef[] = [
  { key: "amount_cents", label: "金额", emoji: "💰", icon: "yen", editor: "number", kinds: ALL, show: (f) => money(f.amount_cents, f.kind === "adjust") },
  { key: "kind", label: "类型", emoji: "↔️", icon: "arrows", editor: "kind", kinds: FLOW, show: (f) => KIND_LABEL[f.kind] },
  { key: "category_id", label: "分类", emoji: "🗂️", icon: "grid", editor: "category", kinds: FLOW, show: (f) => categoryLabel(f.category_id) },
  { key: "account_id", label: "账户", emoji: "💳", icon: "card", editor: "account", kinds: ALL, as: { transfer: "转出" }, show: (f) => accountName(f.account_id) },
  { key: "to_account_id", label: "转入", emoji: "📥", icon: "card", editor: "account", kinds: ["transfer"], show: (f) => accountName(f.to_account_id) },
  { key: "merchant", label: "商家", emoji: "🏪", icon: "store", editor: "text", kinds: FLOW, show: (f) => orNone(f.merchant) },
  { key: "occurred_at", label: "时间", emoji: "🕒", icon: "clock", editor: "date", kinds: ALL, show: (f) => dateTime(f.occurred_at) },
  { key: "tag_ids", label: "标签", emoji: "🏷️", icon: "tag", editor: "tags", kinds: ["expense", "income", "transfer"], show: (f) => tagNames(f.tag_ids).join("、") },
  { key: "note", label: "备注", emoji: "📝", icon: "note", editor: "text", kinds: ALL, show: (f) => orNone(f.note) },
];

export const field = (key: FieldKey) => FIELDS.find((f) => f.key === key)!;

/** 这笔账有的字段，按菜单顺序。 */
export const fieldsOf = (e: Pick<Entry, "kind">) => FIELDS.filter((f) => f.kinds.includes(e.kind));

export const labelOf = (def: FieldDef, e: Pick<Entry, "kind">) => def.as?.[e.kind] ?? def.label;

/** 快捷指令里最初显示的值和现在不同的字段。最初显示的是服务器补过默认值的草稿，不全是模型认出来的。 */
export function changes(e: Entry, r: Recognition): { def: FieldDef; from: string; to: string }[] {
  const out = [];
  for (const def of fieldsOf(e)) {
    const from = r.first.amount_cents === null && def.key === "amount_cents" ? "" : def.show(r.first as Fields);
    const to = def.show(e);
    if (from !== to) out.push({ def, from: from || "空", to: to || "空" });
  }
  return out;
}
