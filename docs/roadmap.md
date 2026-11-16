# micjz 需求与版本规划

## 一句话

支付完成后按一下：截图 → AI 识别 → 手机上一眼核对、一下确认 → 账目落到自己的服务器。先把数据攒起来，展示和分析以后再说。

## 原则

- **少点几下**：识别正确时，触发后只点一次「确认」；常用修改最多两步。
- **快捷指令做薄**：快捷指令只是通用渲染器（显示摘要、弹选项、收输入）；分类、标签、摘要格式、菜单内容全由服务器决定。改业务不用重搭快捷指令。
- 手机端只用 iOS 快捷指令，不做 App、不开网页。
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

- 主菜单：`确认 / 改分类 / 改金额 / 改账户 / 加标签 / 更多`；「更多」里是商家、时间、备注、收支类型。
- 两级分类：先返回一级选项，选完再返回二级选项；快捷指令不知道有几层。
- 取消：用系统选择框自带的取消，直接结束。
- 无状态：`draft` 在快捷指令里来回传，服务器不存草稿。`draft` 里带服务器生成的 ID，确认时以它为主键入账，重复提交只记一笔。
- 当前已选的标签写在 `summary` 里（快捷指令的多选框没法预选）。
- 出错（识别失败、不是支付页、入库失败）都返回 `done` + 明确的失败文案，绝不显示「已记账」。

## 数据

```text
entries
  id           TEXT PK      draft 里的 ID（幂等键）
  kind         TEXT         expense | income
  amount_cents INTEGER      金额，单位分，人民币
  category     TEXT         一级分类
  subcategory  TEXT NULL    二级分类（收入没有二级）
  account      TEXT NULL    账户，从固定列表选（截图看不出就空）
  merchant     TEXT NULL
  occurred_at  TEXT         交易时间；截图没有则用确认时间
  note         TEXT NULL
  created_at   TEXT

entry_tags
  entry_id     TEXT
  tag          TEXT         纯展示/筛选用的标签，如「恋爱」
  PRIMARY KEY (entry_id, tag)
```

分类、账户、标签列表 v0/v1 写在代码常量里（账户先只有「微信」「银行卡」），v2 挪进 SQLite 由网页管理。

识别规则（写进 prompt，服务器再校验）：

- 看不到的字段返回 null，不许编。
- 多个金额时取实付金额，不取原价、优惠、余额。
- 分类、账户只能从给定列表里选；标签不由 AI 打。
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
- `build.sh` + `deploy/micjz.service`，服务器上编译运行。
- 20～30 张真实截图（微信 / 支付宝、优惠、多金额、退款、非成功页）验收，记录金额准确率和人工修改比例。

**交付**：日常能用，数据开始积累。快捷指令理想情况下不改。

### v2 能看能改

- 极简网页：按月列表、编辑、删除、CSV 导出。
- 分类、账户、标签挪进 SQLite，网页管理。

### v3 简单统计

- 月度收支、分类占比、账户汇总、按标签筛选。

### 以后再看（有真实需求再做）

- 报销：垫付与报销款的对应关系，属于带业务逻辑的标记，和普通标签分开设计。
- 疑似重复提示（截图哈希 / 交易号）。
- 手动记账入口（不截图，直接填）。
- 转账 / 退款单独建模。
- 截图留存。
- 隐私模式：快捷指令本地 OCR，只把文字发给模型。
- 预算、资产余额。

## 技术栈

依赖与 micnext 对齐，经验和写法直接复用。

- **Rust 单 crate**（不建 workspace）：`tokio` + `axum` + `serde` + `rusqlite`（bundled）+ `reqwest`（rustls）+ `thiserror` / `anyhow` + `tracing`。
- **DeepSeek**：OpenAI 兼容接口，`reqwest` 直接调，不引 SDK；模型名放配置（gpt 文档说是 `deepseek-flash`，接入时按官方文档再核实）。
- **网页（v2 起）**：`web/` 下 Bun + Vite + Svelte，构建产物用 `rust-embed` 嵌进二进制。
- **部署**：服务器上 `git pull` → `./build.sh` 编译 → systemd 托管单个二进制；HTTPS 交给已有反代（没有就用 Caddy）。
