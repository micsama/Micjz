# 识别留存与收尾

## 当前契约

- 主菜单无摘要、无「更多」层；直接显示金额、类型、分类、账户、商家、时间、标签、备注及确认。所有选项（含 emoji 与当前值）原样回传，数据字段只保存纯值。`warnings` 不展示。快捷指令 JSON 字段与步骤类型不变。
- `catalog.toml` 是分类、模型英文 key、账户、标签唯一来源；`prompt.txt` 是提示词模板。两者均从进程工作目录读取，修改后下次启动生效，不热加载。模板必须含 `{now}`、`{categories}`、`{accounts}`。
- 模型分类 key 映射到中文一级/二级分类；时间未提供或距当前不超过 30 分钟使用本次识别时间。标签由用户选择。
- 图片解码后尝试无损 PNG 压缩，仅更小时采用；文件在数据库同目录的 `images/`，以原始上传 SHA256 命名。数据库记录原图 hash/大小及留存路径。未解码成功的上传不进识别表。
- `recognitions` 保存每次识别的原始 HTTP 响应（包括 JSON 解析失败）、实际展开提示词、模型参数、阶段耗时、原始草稿 JSON、失败原因及确认时间。请求传输失败时 raw 为错误链；HTTP 非成功时含状态码和响应正文。未确认记录和图片不自动清理。
- `entries` 与 `entry_tags` 保存最终账目，`draft_id` 关联识别；确认入账及更新确认时间在同一事务中，重复确认不重复入账。不再记录逐字段 edits，不使用 v_bills。
- 允许日志包含账单信息，不记录图片正文和密钥。新增 image 依赖仅用于解码、无损压缩。

## 手动迁移（本次未执行生产迁移、未重启）

已核实实际 `data/micjz.db` 只有旧版 `entries`、`entry_tags`，没有 `recognitions`、`edits`、`v_bills`。对应脚本为 `deploy/migrate-retention.sql`：为旧账补 `draft_id=id`，创建识别表，保留原账与标签。脚本仅执行一次，不是通用版本兼容器；其他结构不得直接执行。

正式切换时：

1. 人工停止旧服务，避免备份后继续写入。使用 SQLite backup API 生成完整快照（不要只复制 WAL 模式下的 .db 文件），另保留配置与图片目录。
2. 用 sqlite3 对生产库执行 `.read deploy/migrate-retention.sql`，执行 `PRAGMA integrity_check; PRAGMA foreign_key_check;`，核对账目、标签数量及 `draft_id=id`。
3. `cargo build --release`；确认服务 WorkingDirectory 指向项目，`catalog.toml`、`prompt.txt` 随版本部署且可读，再人工启动新二进制。不要在迁移前启动新版本；程序不自动迁移旧库。
4. 快捷指令实机识别并确认一笔，检查账目与识别关联、图片路径。真实识别准确率仍须人工验收。

回滚：停止新版本，恢复迁移前完整备份及旧二进制；切换后产生的数据应先另行备份，不能直接覆盖丢弃。

仓库无现成 systemd unit 或 build.sh；本次只补迁移文件，不假定服务管理方式。
