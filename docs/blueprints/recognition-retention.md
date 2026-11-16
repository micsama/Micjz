# 识别留存与收尾

## 当前契约

已并入 `docs/blueprints/core.md`（识别、存储两节）。本文只保留上线迁移步骤。

## 手动迁移（本次未执行生产迁移、未重启）

已核实实际 `data/micjz.db` 只有旧版 `entries`、`entry_tags`，没有 `recognitions`、`edits`、`v_bills`。对应脚本为 `deploy/migrate-retention.sql`：为旧账补 `draft_id=id`，创建识别表，保留原账与标签。脚本仅执行一次，不是通用版本兼容器；其他结构不得直接执行。

正式切换时：

1. 人工停止旧服务，避免备份后继续写入。使用 SQLite backup API 生成完整快照（不要只复制 WAL 模式下的 .db 文件），另保留配置与图片目录。
2. 用 sqlite3 对生产库执行 `.read deploy/migrate-retention.sql`，执行 `PRAGMA integrity_check; PRAGMA foreign_key_check;`，核对账目、标签数量及 `draft_id=id`。
3. `cargo build --release`；确认服务 WorkingDirectory 指向项目，`prompt.txt` 随版本部署且可读，再人工启动新二进制。不要在迁移前启动新版本；程序不自动迁移旧库。
4. 快捷指令实机识别并确认一笔，检查账目与识别关联、图片路径。真实识别准确率仍须人工验收。

回滚：停止新版本，恢复迁移前完整备份及旧二进制；切换后产生的数据应先另行备份，不能直接覆盖丢弃。

仓库无现成 systemd unit 或 build.sh；本次只补迁移文件，不假定服务管理方式。
