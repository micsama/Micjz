# 蓝图：服务端核心

状态：已实现；真实截图识别准确率待验收。需求与原则见 `docs/roadmap.md`，快捷指令侧见 `shortcuts/README.md`。

## 1. 结构

```text
src/
  main.rs        读配置、启动服务或执行管理命令
  api.rs         HTTP：鉴权、入口校验、访问日志、图片留存
  flow.rs        Draft、主菜单与步骤（纯函数，不碰 IO）
  catalog.rs     分类、账户、标签：读快照、初始数据、命令行增删改
  recognize.rs   调 DeepSeek，逐字段校验成草稿；mock 返回假账单
  db.rs          建表、入账、识别留存
  seed.sql       新库的初始分类、账户、标签
prompt.txt       提示词模板，占位符 {now} {categories} {accounts}
config.toml      不入库，提交 config.example.toml
```

`prompt.txt` 从进程工作目录读取，启动时校验，有误直接退出。分类、账户、标签每个请求从库里读，命令行改完立即生效，见 `docs/blueprints/categories-db.md`。

## 2. 配置

```toml
listen = "127.0.0.1:8730"
token = "..."               # Bearer token
db_path = "data/micjz.db"   # 图片留存在同目录 images/

[deepseek]                  # 整节缺省 = mock 识别（固定假账单，联调用）
model = "deepseek-flash"
timeout_secs = 30
```

DeepSeek API Key 读环境变量 `DEEPSEEK_API_KEY`，仅在配置了 `[deepseek]` 时必需。时区固定 +08:00。

## 3. HTTP 接口

所有请求需 `Authorization: Bearer <token>`。

| 接口 | 请求 | 说明 |
|---|---|---|
| `POST /recognize` | multipart，字段 `image`（≤ 32MB，PNG / JPEG） | 识别截图，返回主菜单 |
| `POST /step` | JSON `{ "draft": str, "field": str, "value": str }` | 提交一步输入，返回下一步 |

所有响应（含错误）都是同一种扁平结构，字段全部输出，不用的给空串 / 空数组：

```json
{ "type": "choose | choose_many | number | text | date | done",
  "summary": "提示语", "draft": "base64url(JSON)", "field": "回传字段名",
  "options": ["..."], "default": "...", "message": "done 时的通知" }
```

错误：鉴权失败 401；请求格式错误（缺 image、非 PNG/JPEG、draft 解不开、选项不存在）400；入库失败 500。都是 `done` + 明确文案。**只有入库成功才出现「已记账」。**

## 4. 识别：永不失败

识别的结果总是一个草稿，回到主菜单让用户核对：

- 模型输出按固定结构逐字段取值；类型不对、为空、不合规的字段置空（分类回「其他」，时间回识别时刻，类型回支出），并记 warn 日志。
- 不在顶部提示：看不出的字段在菜单里显示「请补充」。金额为空时点确认先转去填金额，数字框预填 0（0 不能入账）。
- 模型说不是支付页 → 照样给草稿，原因只进识别记录和日志。
- 调用失败或返回不是 JSON → 空白草稿；完整错误链进识别记录和日志。
- `item`（商品名）作为备注默认值；分类用库里叶子分类的英文 key，映射回一级 / 二级。
- 交易时间与识别时刻相差不到 30 分钟就用识别时刻（截图只到分钟）。
- 请求参数（`parameters()`）只有一份，发请求和落库都用它。

## 5. Draft

```rust
struct Draft {
    id: String,                  // uuid v7，入账主键
    kind: Kind,                  // expense | income
    amount_cents: Option<i64>,   // 未识别为空，确认前必填，> 0
    category: String,
    subcategory: Option<String>,
    account: Option<String>,
    merchant: Option<String>,
    occurred_at: DateTime,       // +08:00
    note: Option<String>,
    tags: Vec<String>,
}
```

JSON → base64url，快捷指令原样回传；草稿里存名称，入账时换成 id。`/step` 解码后按当前库里的分类、账户、标签校验（分类、账户、标签须在列表内，金额若有须 > 0），不合法 400。

## 6. 主菜单与步骤

主菜单 `choose` field=`action`，选项显示当前值，原样回传：

```text
💰 金额 · 32.50      → number  field=amount
↔️ 类型 · 支出       → choose  field=kind（↘️ 支出 / ↗️ 收入；变了则清空分类并转去选分类）
🗂️ 分类 · 餐饮 / 咖啡饮品 → choose field=category（🗂️ 一级）→ 有二级再 choose field=subcategory（📂 二级）
💳 账户 · 微信       → choose  field=account（💳 账户）
🏪 商家 · 瑞幸咖啡   → text    field=merchant
🕒 时间 · 11-16 13:42 → date   field=occurred_at
🏷️ 标签 · 无         → choose_many field=tags（🏷️ 标签，换行合并，整体替换）
📝 备注 · 生椰拿铁   → text    field=note
✅ 确认记账          → 入账；金额为空则转去填金额
```

- 主菜单 summary 为空（选项里已有当前值）；子步骤的 summary 是多行账单摘要。
- 非法金额重新返回 `number`，summary 首行「⚠ 金额无效」。
- 文本清空即置空；未知 field、不在选项内的 value → 400。

## 7. 存储

`entries` / `entry_tags` 是最终账目，按 id 引用 `categories` / `accounts` / `tags`；`recognitions` 每次识别一行，存图片 hash / 大小 / 留存路径、实际 prompt、参数、DeepSeek 原始响应、原因、原始草稿 JSON、阶段耗时，确认时同一事务写 `confirmed_at`。表结构见 `src/db.rs`。

- 确认：`ON CONFLICT(id) DO NOTHING`，重复确认只记一笔，仍返回「已记账」。
- 图片：识别不等留存；后台任务写 `images/<原图 sha256>.<png|jpg>`，PNG 无损重压更小才用，失败只记日志。
- 只认当前表结构：库版本存 `PRAGMA user_version`，与 `src/schema_version` 不符就拒绝启动。旧库由 `./build.sh run|start` 备份后按 `deploy/db/<N>-*.sql` 逐版转换，见 `readme.md`「部署」。

## 8. 验收

- `cargo test`：接口功能测试（mock 识别 + 临时库）、步骤、识别字段校验。
- `cargo fmt`、`cargo clippy --all-targets -- -D warnings`。
- `samples/` 真实截图人工评估金额准确率、人工修改比例、耗时。

## 9. 不做

草稿存储、离线队列、自动重试、报销、迁移框架。
