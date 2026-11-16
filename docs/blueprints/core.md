# 蓝图：服务端核心（v0 + v1）

状态：已批准；v0、v1 服务端已实现（v1 识别未用真实截图验收），快捷指令联调中

需求与原则见 `docs/roadmap.md`，快捷指令侧见 `shortcuts/README.md`。本文是服务端实现契约。

## 1. 结构

单 crate，扁平：

```text
src/
  main.rs        读配置、建路由、启动
  api.rs         HTTP 处理、鉴权、请求/响应类型
  flow.rs        Draft、Step 状态机、摘要渲染（纯函数，不碰 IO）
  catalog.rs     分类、账户、标签常量
  recognize.rs   v0 返回假数据；v1 调 DeepSeek 并校验
  db.rs          v1：建表、入账
config.toml      不入库，提交 config.example.toml
```

依赖：`tokio`、`axum`（multipart）、`serde`/`serde_json`、`base64`、`uuid`（v7）、`jiff` 或 `chrono`（择一）、`toml`、`thiserror`/`anyhow`、`tracing`；v1 加 `reqwest`（rustls、json）、`rusqlite`（bundled）。

## 2. 配置

```toml
listen = "127.0.0.1:8730"
token = "..."               # Bearer token
db_path = "data/micjz.db"   # v1

[deepseek]                  # v1；整节缺省 = mock 识别（固定假账单，联调用）
model = "deepseek-flash"
timeout_secs = 30
```

DeepSeek API Key 读环境变量 `DEEPSEEK_API_KEY`，仅在配置了 `[deepseek]` 时必需。

时区固定 `Asia/Shanghai`，时间一律带 `+08:00` 的 ISO 8601。

## 3. HTTP 接口

所有请求需 `Authorization: Bearer <token>`。

| 接口 | 请求 | 说明 |
|---|---|---|
| `POST /recognize` | multipart，字段 `image`（≤ 32MB，仅 PNG / JPEG） | 识别截图，返回首个 Step（主菜单） |
| `POST /step` | JSON `{ "draft": str, "field": str, "value": str }` | 提交一步输入，返回下一个 Step |

**所有响应（含错误）都是同一种扁平结构**，快捷指令只需一套处理：

```json
{
  "type": "choose | choose_many | number | text | date | done",
  "summary": "多行摘要文本",
  "draft": "base64url(JSON)，done 时可为空串",
  "field": "回传时用的字段名",
  "options": ["..."],
  "default": "...",
  "message": "done 时的通知文案"
}
```

不用的字段给空串/空数组，不省略（快捷指令取不存在的键会出问题）。

错误：鉴权失败返回 401 + `done`「鉴权失败」；请求格式错误返回 400 + `done`「请求无效：…」；识别失败、非支付页、入库失败返回 200 + `done` + 明确失败文案。**只有入库成功才能出现「已记账」**。

## 4. Draft

```rust
struct Draft {
    id: String,                 // uuid v7，/recognize 时生成，入账主键
    kind: Kind,                 // Expense | Income
    amount_cents: i64,          // > 0
    category: String,
    subcategory: Option<String>,
    account: Option<String>,
    merchant: Option<String>,
    occurred_at: String,        // ISO 8601 +08:00
    note: Option<String>,
    tags: Vec<String>,
    warnings: Vec<String>,      // 识别提示，如「未看到交易时间，已用当前时间」
}
```

编码为 JSON → base64url。`/step` 收到后解码并按 catalog 校验（分类、账户、标签必须在列表内，金额 > 0），不合法返回 400。不签名：单用户且有 token。

## 5. Step 状态机（flow.rs）

`fn step(draft, field, value) -> Response`，纯函数；只有「确认」需要调用方入库。

| field | value | 处理 | 下一步 |
|---|---|---|---|
| `action` | 确认 | 入库（v0 跳过） | `done`「已记账 ¥32.50 · 餐饮/咖啡饮品」（v0 文案加「[v0 未入库]」） |
| `action` | 改分类 | — | `choose` field=`category`，当前 kind 的一级分类 |
| `action` | 改金额 | — | `number` field=`amount`，default=元，如 `32.50` |
| `action` | 改账户 | — | `choose` field=`account` |
| `action` | 加标签 | — | `choose_many` field=`tags` |
| `action` | 更多 | — | `choose` field=`more`：商家 / 时间 / 备注 / 收支类型 |
| `category` | 一级名 | 设 category，清空 subcategory | 有二级 → `choose` field=`subcategory`；否则主菜单 |
| `subcategory` | 二级名 | 设置 | 主菜单 |
| `amount` | 元 | 解析为分，≤0 或非法 → 重新返回 `number` 并在 summary 首行提示 | 主菜单 |
| `account` | 账户 | 设置 | 主菜单 |
| `tags` | 换行分隔 | 整体替换；空串 = 清空 | 主菜单 |
| `more` | 商家/时间/备注 | — | `text`/`date`/`text`，field=`merchant`/`occurred_at`/`note` |
| `more` | 收支类型 | — | `choose` field=`kind`：支出 / 收入 |
| `merchant`/`note` | 文本 | 设置，空串 = 清空 | 主菜单 |
| `occurred_at` | ISO 8601 | 解析并转为 +08:00 | 主菜单 |
| `kind` | 支出/收入 | 设置；与原值不同时清空分类 | 不同 → `choose` field=`category`；相同 → 主菜单 |

主菜单 = `choose` field=`action`，options：`确认 / 改分类 / 改金额 / 改账户 / 加标签 / 更多`。

未知 field 或不在 options 内的 value → 400。

## 6. 摘要格式

```text
支出 ¥32.50 · 瑞幸咖啡
餐饮/咖啡饮品 · 微信
11-16 13:42  #恋爱
备注：……
⚠ 未看到交易时间，已用当前时间
```

- 商家、账户缺失显示「—」；无标签、无备注、无提示的行省略。
- 时间：今年内显示 `MM-DD HH:mm`，跨年显示完整日期。

## 7. catalog.rs（v0/v1 常量）

```text
支出：
  餐饮：三餐 · 咖啡饮品 · 零食水果 · 聚餐
  交通：公交地铁 · 打车 · 加油充电 · 停车 · 火车飞机
  购物：日用 · 服饰 · 数码 · 家居
  居住：房租 · 水电燃气 · 话费网费 · 物业
  娱乐：会员订阅 · 游戏 · 电影演出 · 旅行
  医疗：药品 · 门诊
  人情：红包 · 礼物 · 请客
  其他：（无二级）
收入（无二级）：工资 · 奖金 · 红包 · 退款 · 理财 · 其他
账户：微信 · 银行卡
标签：恋爱
```

分类、账户、标签只在这里定义一次；prompt、校验、选项都从这里取。

## 8. 识别（recognize.rs）

**v0**：忽略图片内容，返回固定 Draft：支出 ¥32.50 · 瑞幸咖啡 · 餐饮/咖啡饮品 · 微信 · 当前时间。

**v1**：

- `POST {base_url}/chat/completions`，图片以 `data:image/...;base64,` 传入，`response_format: json_object`，`temperature: 0`，非思考模式，超时取配置，不自动重试。
- prompt 带上 catalog，要求输出：

  ```json
  { "ok": true, "reason": null, "kind": "expense", "amount": "32.50",
    "merchant": "瑞幸咖啡", "account": "微信", "occurred_at": "2026-11-16T13:42:00+08:00",
    "category": "餐饮", "subcategory": "咖啡饮品" }
  ```

- 规则：看不到返回 null，不许编；多金额取实付；分类、账户只能从列表选；非支付成功/账单详情页 → `ok: false` + `reason`。
- 服务端校验（模型输出不可信）：
  - `ok: false` → `done`「未识别到支付信息：{reason}」。
  - 金额缺失或非法 → `done`「未识别到金额」。
  - 时间缺失 → 用当前时间，加 warning。
  - 分类不在列表 → 设为「其他」，加 warning；账户不在列表 → null。
- 日志只记耗时、状态、usage，不记图片和完整账单内容。

## 9. 存储（db.rs，v1）

```sql
CREATE TABLE IF NOT EXISTS entries (
  id           TEXT PRIMARY KEY,
  kind         TEXT NOT NULL CHECK (kind IN ('expense','income')),
  amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
  category     TEXT NOT NULL,
  subcategory  TEXT,
  account      TEXT,
  merchant     TEXT,
  occurred_at  TEXT NOT NULL,
  note         TEXT,
  created_at   TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS entry_tags (
  entry_id TEXT NOT NULL REFERENCES entries(id),
  tag      TEXT NOT NULL,
  PRIMARY KEY (entry_id, tag)
);
```

- 启动时执行上述建表；不做迁移框架。
- 确认：同一事务内插入 entries（`ON CONFLICT(id) DO NOTHING`）和 entry_tags。已存在 → 仍返回「已记账」（重复提交是同一笔）。
- rusqlite 是同步的，用 `spawn_blocking` 或 `Mutex<Connection>` 包一层即可，单用户不需要连接池。

## 10. 验收

v0（服务器本地）：

```sh
curl -s -H "Authorization: Bearer $T" -F image=@any.png localhost:8730/recognize
curl -s -H "Authorization: Bearer $T" -H 'Content-Type: application/json' \
  -d '{"draft":"<上一步的 draft>","field":"action","value":"改分类"}' localhost:8730/step
```

- 走通：改分类（两级）、改金额（含非法输入）、改账户、加标签、更多里的四项、切换收支类型、确认。
- 无 token / 错 token → 401 + done。
- `cargo fmt`、`cargo clippy -- -D warnings` 通过；`flow.rs` 写状态机功能测试。

v1 追加：

- 同一 draft 确认两次只有一条记录。
- `samples/` 下真实截图逐张跑，记录金额准确率、人工修改比例、识别耗时。

Mac 对接（human）：按 `shortcuts/README.md` 搭快捷指令，手机实测。

## 11. 不做

草稿存储、离线队列、自动重试、报销、截图留存、网页（v2）、迁移框架。
