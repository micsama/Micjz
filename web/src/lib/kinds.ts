// 四类账目的业务规则：进不进收支、对余额的影响、出现在哪个列表。只看账目本身，不碰状态。
import type { Entry, Kind } from "./types";

export const KIND_LABEL: Record<Kind, string> = { expense: "支出", income: "收入", transfer: "转账", adjust: "平账" };

/** 进收支统计（列表合计、统计页、预算）的只有支出和收入。 */
export const isSpend = (e: Pick<Entry, "kind">) => e.kind === "expense";
export const isIncome = (e: Pick<Entry, "kind">) => e.kind === "income";

/** 对某个账户余额的影响，分；和 docs/blueprints/entries.md 的余额查询一致。 */
export function effect(e: Entry, account: number): number {
  switch (e.kind) {
    case "income":
    case "adjust":
      return e.account_id === account ? e.amount_cents : 0;
    case "expense":
      return e.account_id === account ? -e.amount_cents : 0;
    case "transfer":
      return e.account_id === account ? -e.amount_cents : e.to_account_id === account ? e.amount_cents : 0;
  }
}

/** 列表里出不出现：账目页不列平账（只是账户上的校正）；账户明细列所有经过这个账户的账目。 */
export const listed = (e: Entry, account?: number) =>
  account === undefined ? e.kind !== "adjust" : e.account_id === account || e.to_account_id === account;
