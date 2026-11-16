# 蓝图：请求记录进库

状态：已实现（2026-11-16）。之后 `reply` 列已删除，表结构以[表结构梳理](single-source.md)为准。

## 动机

日志只打在 screen 终端，翻不回去，也查不了。改成通过鉴权的请求每次在库里记一行，micnext 的 SQL 页（选库读 micjz）直接查，micjz 不再需要单独的日志页。

## 契约

新表，表结构版本 1 → 2。现在是调试期，不写转换脚本，删库重建。

```sql
CREATE TABLE requests (
  id         INTEGER PRIMARY KEY,
  at         TEXT NOT NULL,     -- 收到请求的时刻，RFC 3339 +08:00
  method     TEXT NOT NULL,
  path       TEXT NOT NULL,     -- /recognize、/step，或 404 的原路径
  status     INTEGER NOT NULL,
  elapsed_ms INTEGER NOT NULL,
  draft_id   TEXT,              -- /recognize 新生成的；/step 解析出的；解析失败为空
  field      TEXT,              -- /step 请求的 field
  value      TEXT,              -- /step 请求的 value
  reply      TEXT,              -- 响应 JSON，去掉 draft 字段（draft 能由前后请求重建，太长）
  error      TEXT               -- 失败原因链，与终端日志同一句
);
CREATE INDEX requests_draft ON requests(draft_id);
```

- **只记通过鉴权的请求**：token 不对的（401）只在终端记一行，不进库，以免被扫描的请求刷满。
- **每个请求一行**，在访问日志中间件里写，响应发出前写完；处理器把 draft_id、field、value 填进请求扩展里的槽，失败原因由 `ApiError` 挂在响应扩展上。写库失败只记终端日志，不影响响应，不让本该成功的记账显示失败。
- **和 `recognitions` 的分工**：`recognitions` 继续存识别的完整输入输出（prompt、模型原文、原始草稿）。`requests` 只存交互过程，两边按 `draft_id` 关联。一次记账的完整过程：
  ```sql
  SELECT at, path, status, elapsed_ms, field, value, reply, error
  FROM requests WHERE draft_id = ? ORDER BY id;
  ```
- **终端日志保留**：仍是每个请求一行。screen 里看实时日志，库里查历史。
- 不记图片内容和 token。`value` 和 `reply` 里有账单信息，按现有规则允许记录。
- 不自动清理。一次记账只有几行，量很小。

## 对快捷指令的影响

无。HTTP 接口和响应都不变。

## 不做

- 不做日志等级或框架日志（tracing 的输出）进库。
- 不在 micjz 做查询页，查询交给 micnext 的 SQL 页。
