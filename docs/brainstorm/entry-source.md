# 头脑风暴：账目不依赖截图（手动添加、导入）

状态：方向讨论，不实现。方向定了以后再写蓝图（表结构 4 → 5），建议排在 v2 的 Web 接口蓝图之前：网页上用的账目 ID 取决于这里怎么定。所有表改 `STRICT` 已定（见文末），并进同一次转换。

## 要解决什么

账目现在只有一个来源：截图识别后确认。表结构也是按这个写的：`entries.id` 就是草稿 ID，外键指向 `recognitions`，所以没有截图的账插不进去。

但账目不止从截图来：

- **手动添加**：以前的记账软件全靠手填，这里以后也要有手填入口（roadmap「以后再看」）。
- **导入**：把微信、支付宝导出的历史账单一次性补进来。

这两种都没有识别记录。伪造一条识别记录，会让「这笔账怎么来的」变成假数据。

## 推荐：账目有自己的 ID，截图来的再指向那次识别

```sql
-- id 是账目自己的 ID（UUIDv7，入账时生成）。截图来的账 recognition_id 指向那次识别，
-- 也是确认时的幂等键；手动添加、导入的为空。
CREATE TABLE entries (
  id             TEXT PRIMARY KEY,
  recognition_id TEXT UNIQUE REFERENCES recognitions(draft_id),
  kind           TEXT NOT NULL CHECK (kind IN ('expense', 'income')),
  amount_cents   INTEGER NOT NULL CHECK (amount_cents > 0),
  category_id    INTEGER NOT NULL REFERENCES categories(id),
  account_id     INTEGER NOT NULL REFERENCES accounts(id),
  merchant       TEXT,
  occurred_at    TEXT NOT NULL,
  note           TEXT,
  created_at     TEXT NOT NULL
);
```

- **截图入账**：确认时新生成 `id`，`recognition_id` 填草稿 ID，用 `ON CONFLICT(recognition_id) DO NOTHING` 保证重复确认只记一笔。草稿格式、`/step`、快捷指令都不变。
- **没有截图的账**：`recognition_id` 为空。SQLite 的 `UNIQUE` 允许多个 NULL。
- **不加 `source` 列**：有没有截图，看 `recognition_id` 是否为空就能判断。手动添加和导入目前没有要区分的地方；一批导入的账可以靠相同的 `created_at` 认出来。
- **转换**：重建 `entries`，旧行的 `id` 原样保留，`recognition_id` 取同一个值，`entry_tags` 不用动。
- **Web 接口**：`/api/entries/:id` 用 `entries.id`。详情页的截图和模型原始值通过 `recognition_id` 取，为空就不显示「识别」那一块。

### 另一个方案：只去掉外键

截图账目的 `id` 仍然等于草稿 ID，只把指向 `recognitions` 的外键去掉，手动和导入的账自己生成 ID。

- 好处：少一列，`save` 几乎不用改。
- 坏处：「这笔账来自哪次识别」变成没有约束的隐含约定，要靠「`id` 恰好在 `recognitions` 里」去猜。[表结构梳理](../blueprints/single-source.md)当初去掉 `entries.draft_id`，就是不想留这种隐含关系。

倾向推荐方案。

## 疑似重复

截图记账和手动添加时，金额相同、时间相近的账提示一下，由人决定记不记，不自动去重。这条把 roadmap「以后再看」里的「疑似重复提示」定了下来，提示放在哪一步（快捷指令菜单、网页）、时间窗口多大，做的时候再定。

## 退款

退款要指向原来那笔消费，会影响统计，分类跟着原消费走。它带业务逻辑，不用标签做，和报销一样以后单独设计。

## 历史账单导入（一次性，不做成功能）

不进二进制，不加接口和网页。用一个跑完就扔的脚本，不入库：

1. 读微信、支付宝导出的账单，跳过转账、还信用卡、余额宝转入转出这类不该算收支的行。
2. 按支付方式里的卡号尾号对上账户。商家和商品名成批交给 DeepSeek 归到现有分类，只发文字；不在列表里的归「其他」。商品名作为备注。
3. 去重：和库里已有的账比，金额相同、时间相近的列成一张核对表，人工逐行标「跳过 / 保留」。这一步只属于导入，和系统里的「疑似重复」无关。
4. 退款在退款模型做好之前：全额退款的原消费和退款都不导入；部分退款只导入净额，备注写「已退 ¥x」。
5. 停服务，备份库，直接写 `entries`（`recognition_id` 为空）和 `entry_tags`。

前提是表结构已经允许账目没有识别记录。分错的分类以后在网页上改。

## 同一次转换：所有表改 `STRICT`（已定）

普通表的 `INTEGER` 列存得进小数：`settings.cycle_day = 1.5`、`entries.amount_cents = 0.5` 都能写入，存成 `real`，CHECK 只检查范围。二进制写库时一律用 `i64`，所以目前不会出错；但导入脚本会直接写库，「数据库约束已保证」也得真的成立。

- 所有表（`categories`、`accounts`、`tags`、`recognitions`、`entries`、`entry_tags`、`requests`、`settings`）末尾加 `STRICT`。列类型现在只有 `INTEGER` 和 `TEXT`，不用改。
- `STRICT` 表里，`2.0` 这种能无损转成整数的值会存成整数，`1.5` 直接报错。
- 转换脚本逐张表建新表、复制数据、删旧表、改名，再重建索引（`idx_categories_name`、`idx_entries_occurred`、`idx_requests_draft`）。旧库里要是已经有小数，复制那一步会报错，整个转换回滚，库保持原样。
- 读写库的程序都要支持 `STRICT`（SQLite ≥ 3.37）。rusqlite 自带 SQLite；`convert.py` 和导入脚本用服务器上 Python 带的 SQLite，已确认版本够新，不另加检查。
- 改完以后，类型由数据库保证，代码读取时不再复查；网页等入口照样在边界解析请求。
