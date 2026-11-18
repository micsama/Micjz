// 一笔账怎么显示：图标、标题、副标题、金额和颜色。列表、详情、两个主题都从这里取，不各自判断类型。
import type { Entry } from "./types";
import { effect, isRefund, KIND_LABEL } from "./kinds";
import type { IconName } from "../ui/Icon.svelte";
import { badge, money } from "./format";
import { accountOf, app, categoryOf, topIndex, topOf } from "./store.svelte";
import { color } from "./look";

/** 分类账目显示分类名开头的 emoji（或首字）；转账、平账不属于分类，用线条图标。 */
export type Mark = { text: string } | { icon: IconName };

export interface View {
  mark: Mark;
  /** 主题色；转账、平账不属于任何分类，用中性色。 */
  color: string;
  title: string;
  /** 副标题的几段，主题自己决定分隔符。 */
  meta: string[];
  tags: string[];
  /** 带不带符号由类型决定：收入、退款 +，平账带自身符号，支出、转账不带。 */
  amount: string;
  tone: "out" | "in" | "plain";
}

export const accountName = (id: number | null) => accountOf(id)?.name ?? "";
export const tagNames = (ids: number[]) => app.catalog.tags.filter((t) => ids.includes(t.id)).map((t) => t.name);

/** 「餐饮 › 咖啡饮品」；一级没有子类就只有它自己。 */
export function categoryLabel(id: number | null): string {
  const c = categoryOf(id);
  if (!c) return "";
  return c.parent_id === null ? c.name : `${topOf(c).name} › ${c.name}`;
}

const NEUTRAL = "var(--label3)";

/** `account` 是在某个账户的明细里看：转账按这个账户是转出还是转入显示正负。 */
export function view(e: Entry, account?: number): View {
  const tags = tagNames(e.tag_ids);
  switch (e.kind) {
    case "expense":
    case "income": {
      const c = categoryOf(e.category_id)!;
      const top = topOf(c);
      // 负的支出是退款：钱回来了，和收入一样显示。
      const refund = isRefund(e);
      const income = e.kind === "income" || refund;
      const name = (x: typeof c) => badge(x.name).text;
      // 一眼先看「花在哪类」，再看「买了什么、在哪买」；账户在详情里看。两样都没填才退回账户，免得副标题空着。
      const meta = [e.note, e.merchant].filter((x) => x !== null);
      if (refund) meta.unshift("退款");
      return {
        mark: { text: badge(c.name).icon },
        color: color(topIndex(c)),
        title: c === top ? name(c) : `${name(top)} · ${name(c)}`,
        meta: meta.length ? meta : [accountName(e.account_id)],
        tags,
        amount: money(refund ? -e.amount_cents : e.amount_cents, income),
        tone: income ? "in" : "out",
      };
    }
    case "transfer": {
      const signed = account === undefined ? 0 : effect(e, account);
      return {
        mark: { icon: "transfer" },
        color: NEUTRAL,
        title: e.note ?? KIND_LABEL.transfer,
        meta: [`${accountName(e.account_id)} → ${accountName(e.to_account_id)}`],
        tags,
        amount: signed ? money(signed, true) : money(e.amount_cents),
        tone: signed > 0 ? "in" : "plain",
      };
    }
    case "adjust":
      return {
        mark: { icon: "scale" },
        color: NEUTRAL,
        title: e.note ?? KIND_LABEL.adjust,
        meta: [KIND_LABEL.adjust, accountName(e.account_id)],
        tags,
        amount: money(e.amount_cents, true),
        tone: "plain",
      };
  }
}
