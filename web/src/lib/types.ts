// 和服务端 Draft 字段同名。
export type Kind = "expense" | "income";

export interface Category {
  name: string;
  subs: string[];
}

export interface Account {
  name: string;
  card: { type: "debit" | "credit"; number: string } | null;
}

export interface Catalog {
  expense: Category[];
  income: Category[];
  accounts: Account[];
  tags: string[];
}

export interface Fields {
  kind: Kind;
  amount_cents: number;
  category: string;
  subcategory: string | null;
  account: string;
  merchant: string | null;
  occurred_at: string;
  note: string | null;
  tags: string[];
}

export interface Entry extends Fields {
  id: string;
  created_at: string;
  /** 模型给的草稿；字段为 null 表示模型没给。 */
  origin: Partial<Fields>;
  image: string;
}

export type FieldKey = "amount_cents" | "kind" | "category" | "account" | "merchant" | "occurred_at" | "tags" | "note";
