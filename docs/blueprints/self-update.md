# 蓝图：网页一键更新

状态：已实现（2026-11-17），服务器实机验收待做。参照 micnext 的 self-update（已在服务器验收），按 micjz 现有的 `build.sh` 流程缩小。

## 动机

现在每次更新都要 SSH 上服务器：`git pull && ./build.sh`，`screen -r` 进去 Ctrl-C，再 `./build.sh start`。改成在设置页点一下，服务器自己拉代码、构建，成功后重启成新版本。

## 用户行为

设置页「版本」那一行旁边加「更新」按钮：

- 点击后弹确认框「拉取最新代码并重新构建，成功后服务会重启几秒」。
- 进度依次显示：拉取 → 构建 → 重启。构建要几分钟，期间显示命令输出的最后几行；关掉页面不影响更新。
- 代码没有变化：显示「已是最新（<提交号>）」，不构建，不重启。
- 失败：显示失败的阶段、原因和输出末尾，旧服务继续运行，可以再点。
- 重启后页面自动重载，版本号和目标提交一致就显示「已更新到 <提交号>」。
- 服务不是用 `./build.sh run|start` 启动的（比如直接跑二进制、本地 `bun run dev`），按钮置灰，提示「请用 ./build.sh start 启动服务」。

## 流程

子进程都在仓库目录里执行，继承启动时的环境变量（代理照 micnext 的约定，启动前在 shell 里开好），stdin 为空，stdout 和 stderr 合并进输出缓冲，缓冲只保留最后 64 KiB。

1. **检查**：`git rev-parse @{u}` 失败的话报「当前分支没有 upstream」。`from` 取当前进程构建时的提交（`MICJZ_VERSION`，和设置页的版本号同源）；不是 `build.sh` 构建的为 `dev`，总会重新构建。
2. **拉取**：`git fetch` 最多 5 次，超时依次 20、30、40、50、60 秒；失败或超时立即重试，超时会杀掉 git，5 次都不行才报失败。成功后执行 `git reset --hard @{u}`：服务器上不手改代码，已跟踪文件的改动一律被覆盖，覆盖前先把它们列进输出；未跟踪文件不动。
3. `to` 取 `git rev-parse --short HEAD`；`to == from` 时结束，显示「已是最新」。
4. **构建**：执行一次 `./build.sh build`（会顺带 `convert --check` 报告库版本，不改库）。不设超时：卡住的话去终端 Ctrl-C。退出码非零就报失败，旧服务照常运行；这时源码已经是新版，再点一次会重新构建。
5. **重启**：axum 优雅停止（停止接收新请求，等正在处理的请求结束），释放数据库，然后 `exec ./build.sh start <原配置路径>`。和手动更新完全同一条路：先备份库，需要时转换，再 exec 新二进制。exec 本身失败就打印原因并退出，不自动拉起。

同一时刻只允许一个更新任务，正在更新时再点返回 409。

## 怎么知道仓库在哪

`build.sh` 开头 `export MICJZ_REPO=$repo_dir`，经它 `run`、`start` 启动的服务都带着。服务启动时读到这个变量就启用更新，读不到就不启用。不加配置项，也不靠「二进制恰好在 `target/release/` 下」去推仓库路径。

## 接口（网页会话鉴权，`Cache-Control: no-store`）

`POST /api/update`：不带请求体。成功返回 202。已有任务在跑返回 409；没启用更新返回 503，附上上面那句提示。

`GET /api/update` → 200：

```jsonc
{"state":"unavailable"}
{"state":"idle","output":""}
{"state":"checking","output":"..."}
{"state":"pulling","from":"a1b2c3d","attempt":2,"output":"..."}
{"state":"building","from":"a1b2c3d","to":"e4f5g6h","output":"..."}
{"state":"restarting","from":"a1b2c3d","to":"e4f5g6h","output":"..."}
{"state":"up_to_date","commit":"a1b2c3d","output":"..."}
{"state":"failed","stage":"checking|pulling|building|restarting","error":"中文原因","output":"..."}
```

除 `unavailable` 外都带 `output`。当前版本仍然从 `GET /api/settings` 的 `version` 取，这里不重复。状态只保存在内存里，新进程启动后是 `idle`；`failed`、`up_to_date` 保留到下一次 POST。

网页：任务进行中每秒 GET 一次。见到 `building` 或 `restarting` 时把 `to` 存进 `sessionStorage`；之后请求失败（服务正在重启）就继续轮询 `GET /api/settings`，等它恢复后重载页面，用 `version` 和 `to` 核对。

## 改动清单

| 位置 | 改动 |
|---|---|
| `build.sh` | 开头 `export MICJZ_REPO` |
| `src/update.rs`（新文件） | 任务状态、子进程、输出缓冲 |
| `src/main.rs` | 读 `MICJZ_REPO`；`axum::serve` 加优雅停止；收到重启请求后 exec |
| `src/api/web.rs` | 两条路由 |
| `Cargo.toml` | tokio 加 `process`、`io-util`、`time` 三个 feature，不加新 crate |
| 网页设置页 | 「更新」按钮和进度 |
| `docs/blueprints/web-api.md`、`readme.md` | 补接口和部署说明 |

## 相比 micnext 删掉的

- 不显示二进制大小和前后版本大小对比、提交时间、已运行时长。
- 不需要配置项 `update_repo`，改由 `build.sh` 传环境变量。

## 不做

- 自动定时检查新版本、显示「有新版本」小红点。
- 失败自动回滚。手动恢复办法：`git checkout <from> && ./build.sh && ./build.sh start`。

## 验收

- 本地自动验证：没设 `MICJZ_REPO` 时 POST 返回 503；设了之后，假 upstream 没有新提交时显示「已是最新」。
- 服务器上（交给你）：推一个提交后点更新，依次看到拉取、构建、重启，页面显示「已更新」；构建时故意弄出编译错误，页面显示失败，旧服务照常可用。
