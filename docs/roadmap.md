# micjz 需求与版本规划

## 一句话

支付完成后按一下：截图 → AI 识别 → 手机上一眼核对、一下确认 → 账目落到自己的服务器。先把数据攒起来，展示和分析以后再说。

## 原则

- **少点几下**：识别正确时，触发后只点一次「确认」；常用修改最多两步。
- **快捷指令做薄**：快捷指令只是通用渲染器（显示摘要、弹选项、收输入）；分类、标签、摘要格式、菜单内容全由服务器决定。改业务不用重搭快捷指令。
- 截图记账走 iOS 快捷指令；查看、修改、手填（转账、平账等）走网页（v2 起）。不做 App。
- 服务器自建，账本归自己；截图会交给 DeepSeek 识别，这一点接受。
- 个人单用户、单账本，一个 Bearer Token 鉴权。
- 无网不记账，失败就明确提示，不做离线队列。

## 交互协议

快捷指令只认几种步骤，循环执行直到 `done`：

```text
截屏 → POST /recognize（multipart: image）→ 响应
重复：
  按 step.type 弹出对应界面，提示语用 summary，得到 value
  done 则显示通知并结束
  POST /step { draft, field, value } → 新响应
```

响应是扁平 JSON（快捷指令取嵌套字段麻烦），请求 `/step` 的三个字段都是文本：

```json
{
  "summary": "支出 ¥32.50 · 瑞幸咖啡\n餐饮/咖啡饮品 · 微信\n今天 13:42",
  "draft": "<不透明字符串，快捷指令原样回传>",
  "type": "choose",
  "field": "action",
  "options": ["确认", "改分类", "..."]
}
```

| type | 快捷指令动作 | 附带字段 | 回传的 value |
|---|---|---|---|
| `choose` | 从列表中选取 | `options` | 选中项 |
| `choose_many` | 从列表中选取（选择多个） | `options` | 选中项按换行合并；整体替换原值 |
| `number` | 要求输入：数字 | `default` | 数字文本 |
| `text` | 要求输入：文本 | `default` | 输入文本 |
| `date` | 要求输入：日期和时间 | `default` | ISO 8601 |
| `done` | 显示通知，停止 | `message` | — |

- 主菜单平铺金额、类型、分类、账户、商家、时间、标签、备注和确认；显示 emoji 与当前值，原样回传，无「更多」层。
- 两级分类：先返回一级选项，选完再返回二级选项；快捷指令不知道有几层。
- 取消：用系统选择框自带的取消，直接结束。
- 无状态：`draft` 在快捷指令里来回传，服务器不存草稿。`draft` 里带服务器生成的 ID，确认时以它为幂等键入账，重复提交只记一笔。
- 当前已选的标签写在 `summary` 里（快捷指令的多选框没法预选）。
- 识别永不失败：调用出错、不是支付页、字段不合规都给草稿，看不出的字段留空并在菜单提示，用户补完再确认；金额为空时不能确认。
- 请求错误、入库失败返回 `done` + 明确的失败文案，绝不显示「已记账」。

## 数据

```text
entries
  id             TEXT PK      账目自己的 ID（UUIDv7）
  recognition_id TEXT NULL    → recognitions，截图来的才有；确认的幂等键
  kind           TEXT         expense | income | transfer | adjust
  amount_cents   INTEGER      金额，单位分，人民币；平账是带符号的差额
  category_id    INTEGER NULL → categories，一级或二级（有二级也可只选一级）；支出、收入必填，其余为空
  account_id     INTEGER      → accounts（截图看不出就用优先级第一的账户）；转账的转出方
  to_account_id  INTEGER NULL → accounts，转账的转入方
  merchant       TEXT NULL
  occurred_at    TEXT         交易时间，UTC；截图没有则用确认时间
  note           TEXT NULL
  created_at     TEXT         入账时刻，UTC

entry_tags
  entry_id     TEXT         删账目时一起删
  tag_id       INTEGER      → tags，纯展示/筛选用的标签，如「恋爱」
  PRIMARY KEY (entry_id, tag_id)

categories   (id, position, kind, parent_id, name, key)   两级分类，key 给模型
accounts     (id, position, name, brand, card_type, card_number)   卡号完整存，只把尾号给模型
tags         (id, name)
recognitions (draft_id, ...)   一次识别的输入输出和原始草稿；entries.recognition_id 指向它
requests     (id, at, ...)     通过鉴权的请求日志
settings     (id=1, cycle_day, budget_cents)   单行用户设置：记账周期、每期预算
```

所有表 `STRICT`，时间一律存 UTC、按东八区显示。完整表结构见 [表结构梳理](blueprints/single-source.md) 和 [账目蓝图](blueprints/entries.md)：每个事实只存一处，表之间用外键连。

分类、账户、标签存数据库，网页或命令行增删改，有账目引用的不能删；见 [分类进库](blueprints/categories-db.md)。提示词从 `prompt.txt` 加载。

识别规则（写进 prompt，服务器再校验）：

- 补全分三层：模型只填看得到的字段，看不到的省略，不许编；服务器对不合规字段当作没给，再按规则补默认值（分类「其他」、账户取优先级第一、时间取识别时刻）；补不了的（金额）留空，用户补完再确认。不让整次识别失败。
- 多个金额时取实付金额，不取原价、优惠、余额。
- 分类、账户只能从给定列表里选；标签不由 AI 打。
- 截图里可见的商品名作为备注默认值；看不到就留空，不拿商家名充数。
- 不是支付成功 / 账单详情页 → 返回失败原因。

## 版本

服务端实现契约见 `docs/blueprints/core.md`。

### v0 交互骨架

- Rust 服务：`/recognize` 收图但返回写死的假账单；`/step` 完整走通菜单、两级分类、改金额、加标签等；确认时不入库，只返回 `done`。
- 在 Mac 上搭一次通用快捷指令（见 `shortcuts/README.md`），同步到手机。
- 验证：操作按钮 / 背面轻点触发、摘要显示、点击次数是否够少、取消是否干净。

**交付**：手机上能完整走一遍假账单流程 + 交互结论。

### v1 真实闭环

- `/recognize` 接 DeepSeek 视觉，校验输出。
- 确认时写 SQLite（`ON CONFLICT(id) DO NOTHING`）。
- `build.sh`，服务器上编译、screen 常驻；启动前按版本备份、转换旧库。
- 20～30 张真实截图（微信 / 支付宝、优惠、多金额、退款、非成功页）验收，记录金额准确率和人工修改比例。

**交付**：日常能用，数据开始积累。快捷指令理想情况下不改。

### v2 能看能改

- 极简网页：按记账周期列表、编辑、删除、CSV 导出。交互原型在 `web/`（假数据）。
- 分类、账户、标签的网页管理，见 [网页分类、标签管理](blueprints/categories-web.md)。已实现（2026-11-17）。
- 设置表：记账周期、每期预算，见 [设置表](blueprints/settings.md)。表已建（v4），网页读写待做。
- 表结构 v5：账目不再依赖截图、加转账和平账、所有表改 `STRICT`，见 [账目蓝图](blueprints/entries.md)。已实现（2026-11-16）。
- 表结构 v7：草稿和识别留存只认 ID、记账周期可设「上月 N 号起」、网页手填按 ID 防重，见 [表结构 v7](blueprints/schema-7.md)。已实现（2026-11-17）。
- 网页记一笔（支出、收入、转账）；财富页显示真实余额，左滑账户平账。
- Web 接口：前后端按 [Web 接口蓝图](blueprints/web-api.md) 对接（后端已实现，2026-11-17；含 v6 账户机构字段及独立的网页登录账号）。
- 原型里已有的页面都上线（含统计页：收支、分类占比、标签、账户汇总，原 v3 并入），范围和接口草稿见 [网页头脑风暴](brainstorm/web.md)。

### 以后再看（有真实需求再做）

- 报销：垫付与报销款的对应关系，属于带业务逻辑的标记，和普通标签分开设计。
- 疑似重复（截图哈希 / 交易号 / 金额相同时间相近）：截图识别时发现和已有账目重复，不是简单拦下，而是问用户「是否用这次的截图和识别出的信息更新那一笔」。用途：换一张更清楚的截图，或给以前手填的账补上截图、商家、商品等。具体交互和契约后面再定。
- 退款单独建模（指向原消费，带业务逻辑，不用标签）。
- 转账的截图识别（转账、平账先只从网页手填）。
- 信用卡分期（自己暂不用，优先级最低）。
- 截图留存已实现，见识别留存契约。
- 隐私模式：快捷指令本地 OCR，只把文字发给模型。

## 技术栈

依赖与 micnext 对齐，经验和写法直接复用。

- **Rust 单 crate**（不建 workspace）：`tokio` + `axum` + `serde` + `rusqlite`（bundled）+ `reqwest`（rustls）+ `thiserror` / `anyhow` + `tracing`。
- **DeepSeek**：OpenAI 兼容接口，`reqwest` 直接调，不引 SDK；模型名放配置（gpt 文档说是 `deepseek-flash`，接入时按官方文档再核实）。
- **网页（v2 起）**：`web/` 下 Bun + Vite + Svelte，构建产物用 `rust-embed` 嵌进二进制。
- **部署**：服务器上 `git pull` → `./build.sh` 编译 → screen 里 `./build.sh start`；HTTPS 交给已有反代（没有就用 Caddy）。
