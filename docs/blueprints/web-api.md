# 蓝图：Web 接口（v2）

状态：已实现（2026-11-17）。前后端分工：**后端**改 `src/`、`deploy/`、`build.sh`，**前端**改 `web/`。双方只按本文的契约对接；契约不够用或要改，先改本文，再改代码。

背景、取舍和没选的方案见 [网页头脑风暴](../brainstorm/web.md)「账目按需取」「接口草稿」；表结构见 [账目蓝图](entries.md)、[设置表](settings.md)。前端原型的数据形状在 `web/src/lib/types.ts`，本文的 JSON 与它一致。

## 1. 范围

本蓝图包括：登录、网页托管、读目录 / 账目 / 余额 / 设置 / 识别记录、记一笔、改、删、平账、账户管理、设置修改、CSV 导出。分类、标签管理见 §6.3。

不在本蓝图（之后单独写）：

- **网页上「更新到最新版本」**：涉及进程自己重启，单独设计。这一版设置页只显示版本号。
- **Face ID（Passkey）登录**：以后有空再看，这一版用密码。
- **服务器推送**：不做，见 §3.4。
- **疑似重复、退款、报销**：见 roadmap「以后再看」。

快捷指令的 `/recognize`、`/step` 不变。

## 2. 公共约定

### 2.1 路径与托管

- 接口一律在 `/api/` 下；网页静态文件在 `/` 下。线上由 Caddy 按 `/micjz` 前缀剥掉后转发，服务器看到的路径不带前缀。
- 前端所有请求用相对路径（`api/entries`，不带开头的 `/`），Vite `base: "./"` 已设。
- **静态文件**：后端用 `rust-embed` 把 `web/dist` 嵌进二进制（CLAUDE.md 已定的依赖）。
  - `GET /` 返回 `index.html`，`Cache-Control: no-cache`。
  - `GET /assets/*` 是带哈希的文件，`Cache-Control: public, max-age=31536000, immutable`。
  - `index.html` 之外的根目录文件（`icon.svg`、`manifest.webmanifest` 等）原样返回，`no-cache`。
  - 不存在的路径 404。网页没有前端路由，不需要回退到 `index.html`。
  - 静态文件不需要鉴权。
- `build.sh` 在 `cargo build` 前先在 `web/` 下 `bun install --frozen-lockfile && bun run build`。服务器上要装 Bun。
- 本地开发：前端用 Vite dev server，把 `/api` 代理到 `127.0.0.1:8730`（前端负责配置）。

### 2.2 鉴权

快捷指令和网页分开认证，和 micnext 的网页登录基本一致：

- **快捷指令**：`/recognize`、`/step` 仍用 `config.toml` 的 token（`Authorization: Bearer <token>`），不变。
- **网页**：`/api/*` 只认登录后的会话 Cookie（§4），不接受 Bearer。token 泄露不会连带网页数据，忘了密码也不影响快捷指令记账。
- 不受保护的只有静态文件和 `POST /api/login`。
- 未通过：`/api/*` 返回 401「未登录或登录已过期」；`/recognize`、`/step` 保持现状（`done` 响应）。
- **写请求只接受本站页面发起**：`/api/*` 的非 GET 请求，有 `Sec-Fetch-Site` 时必须是 `same-origin`；没有时如果带 `Origin`，它的主机必须等于 `Host`；两个都没有的是非浏览器客户端，放行。不符合返回 403。同站的其他来源（同 IP 的其他端口、兄弟子域名）也会带上 Cookie，`SameSite` 挡不住。

### 2.3 数据格式

- 请求、响应都是 JSON（`Content-Type: application/json`），导出和图片除外。请求体有未知字段 → 400（`deny_unknown_fields`）。
- **金额**：整数分。支出、收入、转账 > 0；平账 ≠ 0、带符号。绝对值上限 `99_999_999_999`（11 位，前端键盘的上限，也在 JS 安全整数内）。
- **时间**：
  - 响应里一律是 UTC、精确到秒：`"2026-11-16T05:42:00Z"`（就是库里存的字符串，由 `flow::stamp` 产出）。
  - 请求里接受任意 RFC 3339（带 `Z` 或偏移，可带毫秒），服务器换成 UTC、截掉毫秒后存。前端直接发 `Date.toISOString()`。
- **文本**（商家、备注）：去掉首尾空白，空串当作 `null`。长度上限 200 字符，超了 400。
- **ID**：账目是 UUIDv7 字符串（服务器生成）；分类、账户、标签是整数。
- **`null` 和「不出现」**：只有 PATCH 区分两者（§5.3），其他请求里可空字段必须写出来。

### 2.4 错误

所有非 2xx 的 `/api/*` 响应：

```json
{ "error": "还有 3 笔账目在用，不能删除" }
```

`error` 是给人看的中文，前端直接显示，不解析。状态码：

| 状态码 | 什么时候 | 前端怎么做 |
|---|---|---|
| 400 | 请求格式或取值不对（缺字段、金额 0、时间解析不了、类型与分类不匹配） | 显示文案。正常使用不该出现，出现即前端 bug |
| 401 | 未登录或登录已过期（登录接口自己的 401 是密码不对，见 §4） | 回登录页 |
| 403 | 写请求不是从本站页面发起的（§2.2） | 显示文案 |
| 404 | 账目、账户、识别记录不存在 | 显示文案；删除时的 404 当成功 |
| 409 | 和当前数据冲突：重名、还在用不能删、引用的分类 / 账户 / 标签已不存在 | 显示文案，并重取目录（§3.1） |
| 429 | 登录失败太多次（§4） | 在登录表单上显示文案 |
| 500 | 写库等内部错误 | 显示文案；**不自动重试** |

服务端按 CLAUDE.md：失败在一处记完整原因链，响应里只放给人看的那句。

### 2.5 请求日志（改 [请求记录进库](request-log.md) 的契约）

网页每次打开会有一批 GET，静态文件也走中间件，全记进 `requests` 表会淹没快捷指令的记录。改为：

- **终端**：仍是每个请求一行（方法、路径、状态码、耗时）。
- **库**：只记 `/recognize`、`/step`，以及 `/api/` 下的写请求（POST、PUT、PATCH、DELETE）。`POST /api/login`、`PUT /api/password` 不记请求体。

## 3. 读

### 3.1 `GET /api/catalog`

```json
{
  "categories": [{ "id": 1, "kind": "expense", "parent_id": null, "name": "🍜 餐饮", "key": "food" }],
  "accounts": [{ "id": 1, "name": "微信", "brand": "微信", "card": null },
               { "id": 3, "name": "工资卡", "brand": "招商银行", "card": { "type": "debit", "number": "6225880012341234" } }],
  "tags": [{ "id": 1, "name": "恋爱" }]
}
```

- `categories`：平铺，一级和二级都在里面；同一上级之下按 `position` 排。
- `accounts`：按 `position` 排，第一个是默认账户。`card` 为空或 `{type, number}`，`number` 是完整卡号。
- `tags`：按 `id` 排。
- 前端打开时取一次；收到 409、回到前台时重取（§3.4）。

### 3.2 `GET /api/entries?from=<UTC>&to=<UTC>`

按 `occurred_at` 取 `[from, to)` 的账目。两个参数必填，格式同 §2.3 的请求时间，`from < to`，否则 400。

```json
{
  "entries": [{
    "id": "0193…", "recognition_id": "0193…", "kind": "expense", "amount_cents": 3250,
    "category_id": 12, "account_id": 1, "to_account_id": null,
    "merchant": "瑞幸咖啡", "occurred_at": "2026-11-16T05:42:00Z", "note": "生椰拿铁",
    "created_at": "2026-11-16T05:43:10Z", "tag_ids": [1]
  }],
  "earliest": "2023-04-01T02:00:00Z"
}
```

- `entries` 按 `occurred_at` 倒序，同一时间再按 `id` 倒序。`tag_ids` 升序。
- `earliest`：**全库**最早的 `occurred_at`，没有账目为 `null`。前端用它决定跳转面板能翻到哪一年。
- 前端每次取一个东八区自然年（如 2026 年取 `2025-12-31T16:00:00Z` 到 `2026-12-31T16:00:00Z`），服务器不关心是不是整年。

### 3.3 `GET /api/balances`

```json
{ "balances": [{ "account_id": 1, "balance_cents": 123456 }] }
```

每个账户一项（没有账目的是 0），按 `position` 排。查询见 [账目蓝图](entries.md)「常用查询」。前端用它显示余额和净资产，不再从已取的账目里算。

### 3.4 数据怎么保持新鲜（不推送）

快捷指令记账时网页通常不在前台，单用户也很少两台设备同时改。所以不做 SSE / WebSocket，改为：

- 网页回到前台（`visibilitychange` 变成 `visible`）时，前端重取目录、余额、设置和已取过的年份。
- 前端自己的写操作以响应为准（§5），不需要重取。

后端不用为此做任何事。

### 3.5 `GET /api/settings`

```json
{ "cycle_day": 1, "budget_cents": 300000, "version": "a1b2c3d", "entry_count": 1234 }
```

- `budget_cents` 为空表示不设预算。
- `version`：`build.sh` 编译时把 `git rev-parse --short HEAD` 写进环境变量 `MICJZ_VERSION`，代码里 `option_env!("MICJZ_VERSION").unwrap_or("dev")`。
- `entry_count`：全库账目笔数。

### 3.6 `GET /api/recognitions/:id`

`:id` 是账目的 `recognition_id`。返回那次识别「用户在快捷指令里第一眼看到的草稿」（`recognitions.origin`），详情页用它标出哪些字段被人改过。

```json
{
  "id": "0193…",
  "outcome": "ok",
  "reason": null,
  "first": {
    "kind": "expense", "amount_cents": 3250, "category_id": 12, "account_id": 1, "to_account_id": null,
    "merchant": "瑞幸咖啡", "occurred_at": "2026-11-16T05:42:00Z", "note": "生椰拿铁", "tag_ids": []
  }
}
```

- `outcome`：`ok | rejected | error`；`reason` 是不像支付页或识别失败的原因，没有为 `null`。
- `origin` 存的就是 id（v7 起，见 [表结构 v7](schema-7.md)），原样返回，分类、账户改名不影响。v7 之前的记录在升级时按当时的名字换成 id，对不上的 `category_id`、`account_id` 为 `null`（网页显示「当时的已不在」），标签直接去掉。
- `amount_cents` 没识别出来为 `null`。
- 不存在 404。

### 3.7 `GET /api/recognitions/:id/image`

返回留存的原截图，`Content-Type` 按扩展名为 `image/png` 或 `image/jpeg`，`Cache-Control: private, max-age=31536000, immutable`（文件名是内容哈希，不会变）。识别记录不存在，或文件没留存下来（后台写入失败）时 404。前端直接用作 `<img src>`，Cookie 会自动带上。

## 4. 登录

照 micnext 的 [网页登录](../../../micnext/docs/blueprints/web-login.md)：单用户，用户名固定，密码哈希存库，会话 Cookie 签名、服务器不存会话。

### 4.1 行为

- **登录页**：用户名、密码、「登录」。用户名固定为 `dzmfg`（常量），表单上照样填，iPhone 钥匙串才会一起记住、自动填。
- **有效期 30 天，从最后一次使用算起**：30 天内打开过就一直保持登录；连续 30 天没打开，回到登录页。
- **登录失败**：401「用户名或密码不正确」，不区分哪一项错。
- **限速**：进程内只计全局的登录失败次数，10 分钟内满 10 次后，这个窗口剩下的时间里登录一律 429「登录尝试过多，请稍后再试」。成功登录不清零。服务经公网访问，密码可能不够强，需要这层保护。
- **退出登录**：设置页底部。
- **改密码**：设置页「修改密码」，填当前密码和新密码（至少 8 位）。改完其他设备都要重新登录，当前设备保持登录。
- **初始密码和找回**：在服务器上运行 `micjz [config.toml] password`，按提示输两次新密码（终端里不回显，也可以从管道输入），效果同网页改密码。服务运行中也能执行，和现有管理命令同一个入口。
  - 还没设过密码时，登录接口返回 409「尚未设置密码：在服务器上运行 micjz password」，登录页照常显示这句。

### 4.2 存储（表结构 5 → 6，和 §6.1 同一次转换）

```sql
CREATE TABLE web_admin (
  id            INTEGER PRIMARY KEY CHECK (id = 1),
  password_hash TEXT NOT NULL,   -- Argon2id 的 PHC 字符串
  session_key   BLOB NOT NULL    -- 32 字节随机值，签 Cookie 用
) STRICT;
```

- 单行表，没设密码时没有这一行。
- 每次设密码（命令行或网页）都重新生成 `session_key`，旧 Cookie 随之全部失效。
- 密码和会话都存库不进 `config.toml`：网页上能改，命令行也能改，不用重启。

### 4.3 会话 Cookie

- 名称 `micjz_session`，值 `<到期毫秒>.<签名>`；签名是用 `session_key` 对到期时间做的 HMAC-SHA256，base64url 无填充。服务器不存会话。
- 属性：`HttpOnly; SameSite=Strict; Path=/; Max-Age=2592000`；请求头 `X-Forwarded-Proto: https`（Caddy 会带）时加 `Secure`，本地 http 开发不加（Safari 在 http 上不收带 `Secure` 的 Cookie）。
- **续期**：鉴权通过的请求，Cookie 签发已超过 1 天时，在响应里重新下发一张 30 天的。最多每天续一次，不用每个请求都写。
- 用户名、有效期、续期间隔、限速参数都是代码里的常量。

### 4.4 接口

| 接口 | 请求 | 成功 | 失败 |
|---|---|---|---|
| `POST /api/login`（不需要登录） | `{ "username", "password" }` | 204，下发 Cookie | 401 用户名或密码不正确；409 还没设密码；429 尝试过多 |
| `POST /api/logout` | — | 204，`Set-Cookie: micjz_session=; Max-Age=0; Path=/` | — |
| `PUT /api/password` | `{ "current", "new" }` | 204，给当前设备下发新 Cookie | 400 当前密码不正确；400 新密码少于 8 位 |

- 登录接口的错误只在登录表单上显示，不触发「回登录页」。
- 改密码时当前密码不对给 400 而不是 401，免得前端当成登录过期。
- Argon2 校验耗 CPU，放到阻塞线程里做。

### 4.5 依赖

- 新增 `argon2`（RustCrypto，纯 Rust）：密码哈希。
- 新增直接依赖 `ring`：HMAC 和随机数。它已在依赖树里（rustls 用的就是它），不增加编译内容。
- 不加不回显输入的库：命令行先看标准输入是不是终端，是就用 `stty -echo` 关回显，不是就直接读管道（同 micnext 的 `read_hidden`）。

## 5. 账目的写

### 5.1 写入只有一条路

后端要求：截图确认（`/step` 点确认）、记一笔、改、平账，都先在入口把输入解析成一个完整、强类型的账目，再交给 `db.rs` 里**同一个**写入函数（插入）或同一套校验（修改）。规则只写一份：

- `kind` 是 `expense | income` 时 `category_id` 必填，且该分类的 `kind` 与账目一致（一级、二级都可以）；`transfer | adjust` 时为 `null`。
- `kind` 是 `transfer` 时 `to_account_id` 必填且 ≠ `account_id`；其他类型为 `null`。
- 引用的分类、账户、标签在**写入的那个事务里**确认存在，不存在 → 409「分类已不存在，请刷新后重选」等。
- 金额范围见 §2.3。
- `tag_ids` 去重；账目和标签在同一事务里写。

不复用 `Draft`：草稿允许金额为空、按名称引用、只有支出收入，那是快捷指令的交互状态。

### 5.2 `POST /api/entries`：记一笔

```json
{
  "id": "0193…",
  "kind": "expense", "amount_cents": 3250, "category_id": 12,
  "account_id": 1, "to_account_id": null,
  "merchant": null, "occurred_at": "2026-11-17T04:30:00.000Z", "note": "午饭",
  "tag_ids": []
}
```

- `kind` 只能是 `expense | income | transfer`；平账走 §5.5，这里给 `adjust` → 400。
- 所有字段都要写出来（可空的写 `null`）。
- `id` 必填，UUIDv7 文本，由网页在打开记账面板、点「再记一笔」时生成，保存失败重试沿用；格式不对 400。服务器生成 `created_at`，`recognition_id` 为 `null`。
- 新建成功 201，返回完整账目（形状同 §3.2 的一项）。
- **按 `id` 防重**：这个 `id` 已入账就返回那一笔，状态码 200，不比较内容。网页以返回的那笔为准展示，提示「这笔之前已保存，没有重复记」。

### 5.3 `PATCH /api/entries/:id`：改

请求体只放要改的字段，可以是 `kind`、`amount_cents`、`category_id`、`account_id`、`to_account_id`、`merchant`、`occurred_at`、`note`、`tag_ids` 中的一个或几个：

```json
{ "note": null, "tag_ids": [1, 3] }
```

- **字段不出现 = 不改；出现且为 `null` = 清空**。只有 `merchant`、`note` 能清空，其他字段给 `null` → 400。空对象 → 400。
- `tag_ids` 整体替换。
- 服务器把改动合并到当前这笔上，再按 §5.1 校验整笔。
- **改类型**：只允许支出 ↔ 收入，且同一请求里必须带新类型的 `category_id`，否则 400。转账、平账不能改类型，别的类型也不能改成它们。（前端换类型时先发「新类型 + 兜底分类」，再让用户选分类，和快捷指令一致。）
- 平账能改金额（带符号、≠ 0），也能改时间、备注；改的就是存下的差额，不会重新按余额计算。
- `id`、`recognition_id`、`created_at` 不能改（出现 → 400，未知字段规则已覆盖）。
- 成功 200，返回改后的完整账目。不存在 404。

### 5.4 `DELETE /api/entries/:id`：删

- 删 `entries`，`entry_tags` 级联删除；`recognitions` 和截图保留。
- 成功 204；不存在 404（前端当成功）。
- 「撤销」只在前端：先从列表移走，几秒后才真正发 `DELETE`，没有恢复接口。

### 5.5 `POST /api/accounts/:id/adjust`：平账

请求：

```json
{ "balance_cents": 123456, "occurred_at": "2026-11-17T04:30:00.000Z" }
```

`balance_cents` 是用户输入的**实际余额**，可正可负可为 0。`occurred_at` 可省略或为 `null`，用服务器当前时间。

服务器在**同一个事务**里：

1. 按 §3.3 的查询算这个账户当前余额；
2. 差额 = 实际余额 − 当前余额；
3. 差额 ≠ 0 时插入一笔 `adjust`（金额 = 差额，无分类、商家、标签）。

响应 200：

```json
{ "entry": { "...": "新写入的平账，形状同 §3.2" }, "balance_cents": 123456 }
```

- 差额为 0：`entry` 为 `null`，不写入；前端提示「余额已一致」。
- 前端按响应显示结果，和自己的预览不一样也照实显示（预览之后快捷指令可能又记了账）。
- 账户不存在 404。

## 6. 账户

账户名规则同命令行的 `check_name`（去首尾空白、非空、不含 `/` 和换行）；卡号规则同 `check_card_number`（去空白后至少 4 位数字）。

### 6.1 新字段 `accounts.brand`（表结构 5 → 6）

```sql
ALTER TABLE accounts ADD COLUMN brand TEXT;   -- 所属机构；空为「其他」
```

- 转换脚本 `deploy/db/6-web.sql` 做上面这一句和 §4.2 的建表；`src/schema_version` 改成 6，`src/db.rs` 的 `SCHEMA` 同步。
- **机构列表只在前端定义**（`web/src/lib/logos.ts`，决定标志和颜色）。服务器把 `brand` 当普通文本：去首尾空白、空串当 `null`、长度 ≤ 20，不校验是否在列表里；前端遇到不认识的按「其他」显示。
- 给模型的账户行带上机构（`catalog.rs` 的 `account_lines`）：
  - 有卡：`工资卡（招商银行 储蓄卡 尾号 1234）`；
  - 无卡、机构和名称不同：`零钱通（微信）`；
  - 机构为空或和名称相同：只写名称。
- 命令行 `account add` 不变（不设机构），之后在网页上补。

### 6.2 接口

| 接口 | 请求 | 成功 | 失败 |
|---|---|---|---|
| `POST /api/accounts` | `{ "name", "brand", "card" }`（`card` 为 `null` 或 `{ "type": "debit"\|"credit", "number" }`） | 201，账户（§3.1 形状），排在最后 | 409「已经有同名账户」 |
| `PUT /api/accounts/:id` | 同上，整体替换 | 200，账户 | 404；409 重名 |
| `DELETE /api/accounts/:id` | — | 204 | 404；409「还有 N 笔账目在用，不能删除」（`account_id` 或 `to_account_id` 引用都算）；409「至少保留一个账户」 |
| `POST /api/accounts/:id/move` | `{ "to": 0 }`，从 0 开始的目标位置，超出范围夹到两端 | 200，`{ "accounts": [...] }` 全部账户的新顺序 | 404 |
| `POST /api/accounts/:id/adjust` | 见 §5.5 | | |

排序复用 `catalog.rs` 的 `reorder`；增删改和命令行共用同一套校验函数。

### 6.3 分类、标签

契约和规则见 [网页分类、标签管理](categories-web.md)，这里只列接口：

| 接口 | 成功 |
|---|---|
| `POST /api/categories`、`PUT /api/categories/:id`、`POST /api/categories/:id/move` | 201 / 200，`{ "categories": [...] }` 全部分类 |
| `DELETE /api/categories/:id` | 204 |
| `POST /api/tags`、`PUT /api/tags/:id` | 201 / 200，`{ "tags": [...] }` 全部标签 |
| `DELETE /api/tags/:id` | 204 |

## 7. 设置

### `PUT /api/settings`

请求 `{ "cycle_day": 25, "budget_cents": null }`，两个字段都要写。`cycle_day` 为 1～28（本月 N 号起算本月）或 −28～−2（上月 N 号起算本月），见 [表结构 v7](schema-7.md)；`budget_cents` 为 `null` 或 > 0（上限同金额）。成功 200，返回同 §3.5 的整个设置。

## 8. 导出

### `GET /api/export.csv`

- `Content-Type: text/csv; charset=utf-8`，`Content-Disposition: attachment; filename="micjz-20261117.csv"`（东八区当天日期）。
- UTF-8 带 BOM（Excel 和 Numbers 直接打开不乱码），逗号分隔，按 RFC 4180 加引号。
- 全部账目，按 `occurred_at` 正序。列：

| 列 | 内容 |
|---|---|
| 时间 | 东八区 `2026-11-16 13:42` |
| 类型 | 支出 / 收入 / 转账 / 平账 |
| 金额 | 元，两位小数；平账带符号，其余为正 |
| 一级分类、二级分类 | 名称；只记了一级的二级为空 |
| 账户、转入账户 | 名称 |
| 商家、备注 | |
| 标签 | 名称，用「、」连接 |
| 来源 | 截图 / 手记（看 `recognition_id`） |
| ID | 账目 ID |

前端只放一个链接：`<a href="api/export.csv" download>`。

## 8.1 一键更新

`GET /api/update`、`POST /api/update`，见 [一键更新](self-update.md)。

## 9. 后端验收

- `src/api/tests.rs` 补接口功能测试（临时库），至少覆盖：登录成功 204、密码错 401、没设密码 409、第 11 次失败 429；无 Cookie、签名被改、已过期 401，超过 1 天的 Cookie 被续期；改密码后旧 Cookie 401、新 Cookie 可用；`/api/*` 带 Bearer 401；跨站写请求 403；记一笔三种类型和各自的 400；改类型的规则；PATCH 的 `null` 与不出现；删除后再删 404；平账的差额和差额为 0；账户重名、在用、最后一个不能删；`move` 后顺序；`/step` 确认后 `GET /api/entries` 能看到，且走的是同一写入函数。
- 用 curl 在本地跑一遍每个接口，贴出请求和响应样例给前端。
- `cargo fmt`、`cargo clippy --all-targets -- -D warnings`、`cargo test`。
- 改完把本文状态改成「已实现（日期）」，[请求记录进库](request-log.md) 同步 §2.5。

建议实现顺序（每步完成即可交给前端联调）：

1. 托管 + 登录 + `GET catalog / entries / settings / balances`：网页能显示真实数据；
2. 记一笔、改、删；
3. 账户（含 `brand` 转换）、平账；
4. 设置修改、识别记录和截图、导出。

验收请求与响应见 [curl 实跑记录](web-api-curl.txt)（临时库、Mock 识别、示例密码及 token）。

## 10. 前端要做的（参考，前端负责）

- `lib/store.svelte.ts` 里的写入函数（`addEntry`、`update`、`remove`、`saveAccount`、`removeAccount`、`moveAccount`、`setPrefs`）改成调接口，以响应为准；页面不改。
- 加一个 `lib/api.ts`：相对路径、带 Cookie、统一处理 §2.4 的错误（401 回登录、409 重取目录）。
- 账目按自然年缓存，见头脑风暴「账目按需取」；余额改用 §3.3。
- 加登录页（用户名、密码）；设置页加「修改密码」「退出登录」，「更新」先只显示版本号。
- 记账周期和预算从 `localStorage` 改成 §3.5 / §7；主题、隐藏金额仍留在浏览器。
- 原型的假数据改成 `web/mock/` 下按本契约实现的内存假后端，只在 `bun run demo`（`vite --mode mock`）时挂上，不进生产包；`bun run dev` 代理到本机 `127.0.0.1:8730`。登录用 `dzmfg` / `demo`。
