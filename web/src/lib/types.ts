// 和服务端表结构同形：账目引用分类、账户、标签都用 id。见 docs/blueprints/entries.md。

/** 四类账目；只有支出、收入有分类、进收支统计。 */
export type Kind = "expense" | "income" | "transfer" | "adjust";
/** 有分类的两类。 */
export type FlowKind = "expense" | "income";

/** 两级分类平铺；`parent_id` 为空是一级。顺序即 `position`。`key` 给模型，不随改名变。 */
export interface Category {
  id: number;
  kind: FlowKind;
  parent_id: number | null;
  name: string;
  key: string;
}

export interface Account {
  id: number;
  name: string;
  /** 所属机构，取 `LOGOS` 里的名称；决定标志，也会告诉模型。空为「其他」。 */
  brand: string | null;
  card: { type: "debit" | "credit"; number: string } | null;
  /** 别人的钱（如公司付）：余额没有意义，不进净资产、不能平账。 */
  external: boolean;
}

export interface Tag {
  id: number;
  name: string;
}

export interface Catalog {
  categories: Category[];
  /** 顺序即 `position`，排第一的是默认账户。 */
  accounts: Account[];
  tags: Tag[];
}

/** `entries` 的一行加它的标签。 */
export interface Entry {
  id: string;
  /** 截图来的才有，指向那次识别。 */
  recognition_id: string | null;
  kind: Kind;
  /** 分；平账是带符号的差额，负的支出是退款，其余大于 0。 */
  amount_cents: number;
  /** 支出、收入必填（最细一级），其余为空。 */
  category_id: number | null;
  /** 转账的转出方。 */
  account_id: number;
  /** 转账的转入方，其余为空。 */
  to_account_id: number | null;
  merchant: string | null;
  occurred_at: string;
  note: string | null;
  created_at: string;
  tag_ids: number[];
}

/** 一次识别：用户在快捷指令里第一眼看到的草稿（服务器补过默认值）。截图另取，见 `imageUrl`。 */
export interface Recognition {
  id: string;
  outcome: "ok" | "rejected" | "error";
  /** 不像支付页、识别出错的原因。 */
  reason: string | null;
  /** 最初显示的字段。金额可能没识别出来；分类、账户后来改名或删掉了就对不上，为空。 */
  first: Omit<Fields, "amount_cents" | "category_id" | "account_id"> & { amount_cents: number | null; category_id: number | null; account_id: number | null };
}

/** 详情里能改的字段。 */
export type Fields = Pick<Entry, "kind" | "amount_cents" | "category_id" | "account_id" | "to_account_id" | "merchant" | "occurred_at" | "note" | "tag_ids">;
export type FieldKey = keyof Fields;
