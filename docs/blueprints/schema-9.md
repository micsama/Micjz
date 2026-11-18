# 蓝图：表结构 v9，退款记负支出、外部账户

状态：已实现（2026-11-18）。表结构版本 8 → 9。方向和取舍见 [消费、付款、回款分开](../brainstorm/spend-flow.md)。快捷指令、提示词、草稿格式不变。

## 契约

### 退款 = 负支出

```sql
amount_cents INTEGER NOT NULL
  CHECK (amount_cents > 0 OR (kind IN ('expense', 'adjust') AND amount_cents <> 0))
```

- 支出可以为负，表示退款：自带分类、账户，**不关联原账**，算在退款那一期。
- 统计、预算、余额的公式不变：按分类求和就是冲减后的净额；余额按「支出 −」算，负的支出就是加回来。
- 收入、转账仍须大于 0；平账照旧带符号。
- 识别和快捷指令只出正的支出、收入，不变。

### 外部账户

```sql
accounts.external INTEGER NOT NULL DEFAULT 0 CHECK (external IN (0, 1))
```

- 外部账户是「别人的钱」，比如「公司付」：餐补记成餐饮支出、占预算，账户选它，不扣自己的余额。
- 它是无底洞：余额照算但没有意义，不进净资产、资产、负债，不能平账（`/api/accounts/{id}/adjust` 返回 400）。
- 其余和普通账户一样：参与识别（模型从账户列表里选）、记账、转账、排序、删除规则。

### HTTP

- `Account` 加 `external: boolean`；`POST /api/accounts`、`PUT /api/accounts/{id}` 的请求体必须带它。
- `EntryInput` 的金额：支出、平账可为负。

## 网页

- 账目列表左滑支出（正数）多一个「退款」：按原账的分类、账户、商家、备注、标签新记一笔负支出，时间取现在，然后打开它的金额面板，默认全额，改成部分退款再点完成。
- 负支出显示为 `+¥x`、收入色，副标题前加「退款」；改金额时输入不带符号的数，保留负号。
- 账户编辑里多一个「外部账户」开关。财富页的外部账户不显示余额、不进净资产，左滑没有平账。

## 转换：`deploy/db/9-refund-external.sql`

- `accounts` 加列（`ALTER TABLE ADD COLUMN`，默认 0）。
- `entries` 按新约束重建，数据原样复制，重建 `idx_entries_occurred`。
- 现有的收入分类「↩️ 退款」不动。（之后已在线上手工移除，收入分类收为工资奖金、红包、二手变卖、回款、其他。）
