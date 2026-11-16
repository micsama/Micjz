# 蓝图：表结构梳理（每个事实只存一处，表之间用外键连）

状态：已实现（2026-11-16）。表结构版本 2 → 3；调试期删库重建，快捷指令不变。

## 一共 7 张表

| 表 | 一行是什么 | 什么时候写 |
|---|---|---|
| `categories` | 一个分类（一级或二级） | 命令行增删改 |
| `accounts` | 一个资金账户（银行卡、零钱等） | 命令行增删改 |
| `tags` | 一个标签 | 命令行增删改 |
| `recognitions` | 一次识别（一张截图），主键就是草稿 ID | 收到截图、识别完 |
| `entries` | 一笔账，主键就是草稿 ID | 点「确认记账」 |
| `entry_tags` | 某笔账打了某个标签 | 点「确认记账」，和 `entries` 同一事务 |
| `requests` | 一次通过鉴权的 HTTP 请求（访问日志） | 每个请求结束时 |

前 6 张是业务数据，`requests` 是日志，放同一个库是为了能用外键按草稿 ID 关联。

## 关系

```text
categories ──< categories (parent_id，两级)
categories ──< entries.category_id
accounts   ──< entries.account_id
entries    ──< entry_tags >── tags
recognitions ──1:0..1── entries.id        识别了不一定记账
recognitions ··< requests.draft_id        一次记账过程的所有请求（只按值关联，不设外键）
```

## 各表字段

```sql
CREATE TABLE categories (
  id        INTEGER PRIMARY KEY,
  position  INTEGER NOT NULL,                     -- 优先级：菜单顺序，模型分不清时靠前优先
  kind      TEXT NOT NULL CHECK (kind IN ('expense', 'income')),
  parent_id INTEGER REFERENCES categories(id),   -- 空为一级
  name      TEXT NOT NULL,
  key       TEXT NOT NULL UNIQUE                  -- 给模型的 key
);
-- 同类型、同父级下名称唯一

CREATE TABLE accounts (
  id          INTEGER PRIMARY KEY,
  position    INTEGER NOT NULL,   -- 优先级：菜单顺序；识别不出账户时默认第一个
  name        TEXT NOT NULL UNIQUE,                                    -- 如「招商银行储蓄卡」「微信零钱」
  card_type   TEXT CHECK (card_type IN ('debit', 'credit')),          -- 储蓄卡 / 信用卡；非银行卡为空
  card_number TEXT                                                     -- 完整卡号，方便复制核对；非银行卡为空
);

CREATE TABLE tags (
  id   INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE
);

CREATE TABLE recognitions (
  draft_id     TEXT PRIMARY KEY,
  outcome      TEXT NOT NULL CHECK (outcome IN ('ok', 'rejected', 'error')),
  model        TEXT NOT NULL,
  upload_ms    INTEGER NOT NULL,   -- 收图耗时
  model_ms     INTEGER NOT NULL,   -- 模型耗时
  image_sha256 TEXT NOT NULL,
  image_bytes  INTEGER NOT NULL,
  image_path   TEXT NOT NULL,
  prompt       TEXT NOT NULL,
  parameters   TEXT NOT NULL,      -- 请求模型的固定参数
  raw          TEXT NOT NULL,      -- 模型原始响应；调用失败时是错误链
  reason       TEXT,               -- rejected / error 的原因
  origin       TEXT NOT NULL       -- 用户第一眼看到的草稿（JSON）
);

CREATE TABLE entries (
  id           TEXT PRIMARY KEY REFERENCES recognitions(draft_id),
  kind         TEXT NOT NULL CHECK (kind IN ('expense', 'income')),
  amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
  category_id  INTEGER NOT NULL REFERENCES categories(id),   -- 最细一级
  account_id   INTEGER NOT NULL REFERENCES accounts(id),
  merchant     TEXT,
  occurred_at  TEXT NOT NULL,     -- 交易时间
  note         TEXT,
  created_at   TEXT NOT NULL      -- 确认时刻
);

CREATE TABLE entry_tags (
  entry_id TEXT NOT NULL REFERENCES entries(id),
  tag_id   INTEGER NOT NULL REFERENCES tags(id),
  PRIMARY KEY (entry_id, tag_id)
);

CREATE TABLE requests (
  id         INTEGER PRIMARY KEY,
  at         TEXT NOT NULL,       -- 收到请求的时刻
  method     TEXT NOT NULL,
  path       TEXT NOT NULL,
  status     INTEGER NOT NULL,
  elapsed_ms INTEGER NOT NULL,
  draft_id   TEXT,                -- 请求里解析出的草稿 ID，原样记；解析不出为空。不设外键
  field      TEXT,                -- /step 的 field
  value      TEXT,                -- /step 的 value
  error      TEXT                 -- 失败原因
);
```

## 相对现在删掉的重复

| 删掉 | 理由 | 需要时这样取 |
|---|---|---|
| `entries.draft_id` | 和 `entries.id` 恒等 | `entries.id` |
| `recognitions.confirmed_at` | 记了账就是确认了 | `entries` 有没有这行；时刻是 `created_at` |
| `recognitions.at` | 和 `/recognize` 请求的时刻重复 | `requests.at`（`path = '/recognize'`） |
| `recognitions.elapsed_ms`、`timings` | 模型耗时存了两份 | `upload_ms`、`model_ms` 两列 |
| `requests.reply` | 由服务器当时的草稿算出，不是独立事实 | 不再保留；`requests` 只用于查看操作记录 |

## 账户

- **卡号只给模型看尾号**：截图里一般显示「招商银行储蓄卡(1234)」。prompt 里每个账户一行 `名称（储蓄卡 尾号 1234）`，模型按尾号和卡类型选账户；完整卡号不发给 DeepSeek，也不进日志。
- **优先级**：`position` 越小越靠前，决定主菜单里账户的顺序。模型没给出账户、或给的不在列表里时，服务器给草稿填第一个账户，用户在主菜单能看到并改。账户因此不会为空：`entries.account_id` 改为 NOT NULL，命令行不允许删掉最后一个账户。
- **补全分三层**：
  1. 模型只填截图里看得到的字段，看不到的留空，不猜。
  2. 服务器按规则给能定默认值的字段补默认值：账户取优先级第一，分类取「其他」，时间取识别时刻。
  3. 没有合理默认值的字段（目前只有金额）留空，在菜单里显示「请补充」，用户填完才能确认。

  CLAUDE.md「看不到的字段必须是 null」一句改成这三层的说法，roadmap 同步。
- **分类也加 `position`**：现在分类按 id 排序，想调整顺序只能删了重加。加上这一列，命令行可以 `category move`。账户同理有 `account move`。
- 命令行：`account add <名称> [储蓄卡|信用卡 <卡号>]`、`account move <名称> <位置>`；`account list` 显示完整卡号。

## 为什么 `entry_tags` 不并进 `entries`

一笔账可以有多个标签，一个标签也会用在多笔账上，是多对多关系。并进 `entries` 只能把标签存成文本（如 `恋爱,出差`），那样：

- 标签名在每笔账里各存一份。标签改名得改所有账目，违背每个事实只存一处。
- 外键失效，删标签时没法检查有没有账在用。
- 按标签筛选要做字符串匹配，不能走索引。

所以保留一张关联表，这是关系库表达多对多的标准做法。它和 `entries` 在同一个事务里写，不会出现账目有了、标签没有的情况。

## 行为变化

- `/step` 先检查草稿 ID 在 `recognitions` 里存在，不存在就返回 400「草稿不存在」。`entries.id` 的外键因此成立，删库前留下的旧草稿也会被明确拒掉。
- `requests.draft_id` 不设外键：日志要如实记下收到了什么，包括不存在的草稿 ID、识别记录没存成功的请求；设了外键，这些请求的日志会写不进去。
- 入账只写 `entries` 和 `entry_tags`，不再更新 `recognitions`。

## 常用查询

```sql
-- 模型 vs 最终（调 prompt）
SELECT r.origin, e.* FROM recognitions r JOIN entries e ON e.id = r.draft_id;

-- 一次记账的全过程
SELECT at, path, status, elapsed_ms, field, value, error
FROM requests WHERE draft_id = ? ORDER BY id;

-- 识别了但没记账
SELECT * FROM recognitions r WHERE NOT EXISTS (SELECT 1 FROM entries e WHERE e.id = r.draft_id);
```

## 代价

- 每一步给用户看了什么，库里不存，也不能可靠重建：草稿由快捷指令回传，重试或用旧草稿再提交都会让日志顺序和实际状态对不上。`requests` 只用来查看用户做了哪些操作。调 prompt 只需要比较 `origin` 和 `entries`，不受影响。
- 请求记录写库失败时（只记终端日志，不影响响应），那次识别的时刻会丢，概率很低。
