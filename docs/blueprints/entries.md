# 蓝图：账目独立于识别，加转账和平账，所有表改 STRICT

状态：已批准（2026-11-16），待实现。表结构版本 4 → 5，一次转换做完三件事；快捷指令、提示词和草稿格式不变。排在 v2 的 Web 接口蓝图之前：网页上的账目 ID、记一笔、财富页余额都建立在这里。

## 动机

1. **账目不止来自截图。** 现在 `entries.id` 就是草稿 ID，外键指向 `recognitions`，没截图的账插不进去。但以后还有网页手动记一笔，和一次性导入历史账单。伪造识别记录会让「这笔账怎么来的」变成假数据。
2. **财富页要真实余额。** 余额要靠转账和平账才算得出来：
   - 还信用卡如果记成支出，消费会被算两次；
   - 不记的话，两边账户都对不上。
   
   这两类往往没有截图，只从网页手填。
3. **类型要由数据库保证。** 普通表的 `INTEGER` 列存得进小数，`entries.amount_cents = 0.5`、`settings.cycle_day = 1.5` 都能写入，CHECK 只检查范围。导入脚本会绕过二进制直接写库，「数据库约束已保证」必须真的成立。

`entries` 要为 1、2 重建一次，3 要求所有表重建，所以合成一次转换。

## 契约

### `entries`

```sql
-- id 是账目自己的 ID（UUIDv7，入账时生成）。截图来的 recognition_id 指向那次识别，也是确认时的幂等键。
-- 支出、收入有分类；转账从 account_id 转到 to_account_id；平账的 amount_cents 是差额，可正可负。
CREATE TABLE entries (
  id             TEXT PRIMARY KEY,
  recognition_id TEXT UNIQUE REFERENCES recognitions(draft_id),
  kind           TEXT NOT NULL CHECK (kind IN ('expense', 'income', 'transfer', 'adjust')),
  amount_cents   INTEGER NOT NULL CHECK (amount_cents > 0 OR (kind = 'adjust' AND amount_cents <> 0)),
  category_id    INTEGER REFERENCES categories(id),
  account_id     INTEGER NOT NULL REFERENCES accounts(id),
  to_account_id  INTEGER REFERENCES accounts(id),
  merchant       TEXT,
  occurred_at    TEXT NOT NULL,
  note           TEXT,
  created_at     TEXT NOT NULL,
  CHECK ((kind IN ('expense', 'income')) = (category_id IS NOT NULL)),
  CHECK ((kind = 'transfer') = (to_account_id IS NOT NULL)),
  CHECK (to_account_id <> account_id)
) STRICT;
CREATE INDEX idx_entries_occurred ON entries(occurred_at);
```

**四类账目**

| kind | 金额 | 分类 | 账户 | 进收支统计 | 对余额 |
|---|---|---|---|---|---|
| `expense` 支出 | > 0 | 必填 | `account_id` | 是 | − |
| `income` 收入 | > 0 | 必填 | `account_id` | 是 | + |
| `transfer` 转账 | > 0 | 无 | 从 `account_id` 到 `to_account_id` | 否 | 转出 −、转入 + |
| `adjust` 平账 | ≠ 0，带符号 | 无 | `account_id` | 否 | + 差额 |

- **余额** = 该账户所有账目按上表加总。每个账户的第一笔平账就是期初余额，不另设字段。信用卡欠款是负数，还款是「工资卡 → 信用卡」的转账。
- **平账存差额**：之后改动老账目，余额会跟着变，不会被某次平账「冻住」。
  - **差额由服务器算**：网页只提交实际余额，服务器在写入的那个事务里按当前余额算差额再存。网页显示的差额只是预览：从打开到保存之间，快捷指令可能又确认了一笔，用网页上的旧余额算会把余额平错。
  - 响应返回实际写入的差额和平账后的余额，网页按它显示结果，和预览不一样也照实显示。
  - 差额为 0 时不写入，返回「余额已一致」。
- **截图来的账**：
  - 确认时新生成 `id`，`recognition_id` 填草稿 ID；
  - 用 `ON CONFLICT(recognition_id) DO NOTHING`，重复确认只记一笔，标签用新 `id` 写；
  - 识别和快捷指令照旧只出支出、收入。
- **没有截图的账**（手动、导入）：`recognition_id` 为空，SQLite 的 `UNIQUE` 允许多个 NULL。
- **不加 `source` 列**：有没有截图看 `recognition_id` 就能判断。手动和导入目前没有需要区分的地方。
- **商家、备注、标签**：四类都可以有，不加限制。
- **分类和账目类型是否匹配**：照旧由入口保证。快捷指令按当前类型只给对应的分类，网页同理。

### 时间

- **存**：所有时间列（`entries.occurred_at`、`entries.created_at`、`requests.at`）一律存 UTC，写成 `2026-11-16T05:42:00Z`，精确到秒，由一个函数格式化。按时间段查询直接比较字符串。
- **用**：显示、记账周期、年份起止按东八区算。时区只在一处定义（服务器 `flow::tz()`，网页一个常量），以后加时区设置只改这一处，数据不用转换。
- **进**：快捷指令传来的时间带偏移，换成 UTC 存；截图上的时间没有时区，按东八区理解。

### 所有表改 `STRICT`

`categories`、`accounts`、`tags`、`recognitions`、`entry_tags`、`requests`、`settings` 的列、约束、索引都不变，只在末尾加 `STRICT`。唯一例外：`entry_tags.entry_id` 加 `ON DELETE CASCADE`，删账目时标签跟着删。

- 现有列类型只有 `INTEGER` 和 `TEXT`，都是 `STRICT` 允许的类型。
- `STRICT` 表里，`2.0` 这种能无损转成整数的值存成整数，`1.5` 直接报错。
- 要求 SQLite ≥ 3.37：rusqlite 自带 SQLite；服务器上 Python 带的 SQLite 已确认够新，不另加检查。
- 以后新建的表也一律 `STRICT`（已写进 CLAUDE.md）。

### 转换：`deploy/db/5-entries.sql`

由 `./build.sh start` 在服务停止后备份、执行。`convert.py` 已经在关闭外键的事务里跑，结束时做 `foreign_key_check`，任何一步失败都整体回滚。

- 每张表：按新定义建 `<表>_new`，`INSERT INTO <表>_new SELECT ...` 复制数据，删旧表，再改名回原名。外键写的是最终表名，改名后自然对上。
- `entries` 复制时，`recognition_id` 取旧的 `id`，`to_account_id` 为空。旧账目的 `id` 原样保留，所以 `entry_tags` 不用改数据。
- 时间换成 UTC：`strftime('%Y-%m-%dT%H:%M:%SZ', x)`。现有值带 `+08:00` 偏移、有的带小数秒，SQLite 都能解析。
- 最后重建 `idx_categories_name`、`idx_entries_occurred`、`idx_requests_draft`。
- 旧库里要是已经有小数，复制那一步会报错，转换回滚，库保持原样。

## 影响

- **快捷指令、提示词、草稿格式、`/recognize`、`/step`**：行为不变。`Draft` 的 `Kind` 仍只有支出、收入；数据库允许的类型比草稿多，这是有意的。
- **代码**：
  - `src/db.rs`：
    - `SCHEMA` 换成新表结构；
    - `save` 生成 `id`，冲突键改成 `recognition_id`。
  - 时间统一用一个函数格式化（见「时间」）。
  - 兜底分类按 key（`other`、`other_in`）找，不再按名称「其他」找；命令行只禁止删除它，可以改名。
  - `src/catalog.rs`：
    - 判断「账户有账目在用、不能删」时，要同时看 `account_id` 和 `to_account_id`；
    - `account list` 里的笔数同样要算两列。
  - `src/schema_version`：4 → 5。
- **文档**：
  - roadmap「数据」的 `entries` 改成新结构，「交互协议」里「以它为主键入账」改成「以它为幂等键」；
  - roadmap「以后再看」去掉已建模的转账、平账；
  - [表结构梳理](single-source.md)的关系图和常用查询改成 `e.recognition_id = r.draft_id`；
  - [设置表](settings.md)里「随 v5 改 STRICT」改成已完成；
  - [balance.md](../brainstorm/balance.md) 标明改用平账账目。
- **Web 接口（之后的蓝图）**：
  - `/api/entries/:id` 用 `entries.id`；
  - 详情页的截图和模型原始值通过 `recognition_id` 取，为空就不显示「识别」那一块；
  - 网页记一笔和截图确认共用一个写入函数，各自把输入转成完整账目再写入；网页不复用 `Draft` 的校验（草稿允许金额为空、按名称引用、只有支出收入）；
  - 记一笔支持支出、收入、转账；平账是账户上的操作（财富页左滑账户），提交的是实际余额（`balance_cents`），不是差额；
  - 余额由服务器按下面的查询算。

## 常用查询

```sql
-- 各账户余额
SELECT a.name, coalesce(sum(CASE
         WHEN e.kind IN ('income', 'adjust') THEN e.amount_cents
         WHEN e.kind = 'expense' THEN -e.amount_cents
         WHEN e.account_id = a.id THEN -e.amount_cents   -- 转出
         ELSE e.amount_cents                              -- 转入
       END), 0) AS balance_cents
FROM accounts a LEFT JOIN entries e ON a.id IN (e.account_id, e.to_account_id)
GROUP BY a.id ORDER BY a.position;

-- 模型 vs 最终（调 prompt）
SELECT r.origin, e.* FROM entries e JOIN recognitions r ON r.draft_id = e.recognition_id;
```

## 不在本蓝图（结论先记下）

- **疑似重复**：截图记账和手动记一笔时，金额相同、时间相近的提示一下，由人决定，不自动去重。做的时候再写契约。
- **退款**：指向原来那笔消费，带业务逻辑，不用标签做，以后单独设计。
- **转账、平账的截图识别**：先只从网页手填。以后要识别「还款」「提现」，会改提示词和快捷指令菜单，单独定契约。
- **历史账单导入**：一次性脚本，不进二进制，不加接口和网页，不入库。本蓝图实现后再跑：
  1. 读微信、支付宝导出的账单。还信用卡、零钱提现这类两边账户都对得上的，记成转账；余额宝转入转出等对不上的跳过。
  2. 按支付方式里的卡号尾号对上账户。商家和商品名成批交给 DeepSeek 归到现有分类，只发文字；不在列表里的归「其他」。商品名作为备注。
  3. 去重：和库里已有的账比，金额相同、时间相近的列成一张核对表，人工逐行标「跳过 / 保留」。这一步只属于导入，和系统里的「疑似重复」无关。
  4. 退款模型做好之前：全额退款的原消费和退款都不导入；部分退款只导入净额，备注写「已退 ¥x」。
  5. 停服务，备份库，直接写 `entries`（`recognition_id` 为空）和 `entry_tags`。要在第一次平账之前导入：平账存的是差额，先平账再导入，余额会多算。
