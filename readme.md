# micjz

个人 AI 截图记账：iOS 快捷指令截图 → 服务端调 DeepSeek 识别 → 手机确认 → SQLite 入账。需求与规划见 `docs/roadmap.md`。

## 管理分类、账户、标签

数据都在数据库里，在项目目录下用命令行改，改完立即生效（不用重启）。第一个参数是配置文件，缺省读当前目录的 `config.toml`。

```sh
micjz category list                              # 树状列出：名称、key、账目数
micjz category add    支出 学习 study            # 加一级分类
micjz category add    支出 学习/书籍 books        # 加二级分类
micjz category rename 支出 学习/书籍 图书
micjz category rm     支出 学习/图书

micjz account list | add 现金 | rename 现金 零钱 | rm 零钱
micjz tag     list | add 出差 | rename 出差 差旅 | rm 差旅

micjz /path/to/config.toml category list         # 指定配置文件
```

- 类型写 `支出` 或 `收入`；分类路径是 `一级` 或 `一级/二级`。
- 每个分类必须有 key：小写英文、数字或下划线，全局唯一。模型只看叶子分类（没有二级的一级、二级）的 key。
- 有账目的分类、账户、标签不能删；一级下还有二级时要先删二级；「其他」不能删、不能改名。
- 新加的分类排在最后；名称不能含 `/` 和换行。

## 部署

服务器上用 screen 常驻（同 micnext）：

```sh
screen -S micjz
./build.sh run              # 构建、按需转换数据库、前台运行；Ctrl-A D 挂到后台
```

更新：`git pull && ./build.sh`（构建期间旧服务照跑），然后 `screen -r micjz`，Ctrl-C 停掉旧服务，`./build.sh start` 启动新版本。

- `./build.sh [debug] [build|run|start] [config.toml]`：`build` 只构建并检查库；`run` 构建后启动；`start` 不构建直接启动。每次启动前都备份成 `data/micjz.db.v<版本>-<时间>.bak`（不自动清理），版本落后再转换。
- 首次部署：`config.example.toml` 复制为 `config.toml`；启动前 `export DEEPSEEK_API_KEY=...`。需要 Rust 工具链和 Python ≥ 3.11。

数据库版本存在 `PRAGMA user_version`，程序版本在 `src/schema_version`，不一致就拒绝启动。改表结构时版本加一并写转换脚本 `deploy/db/<N>-*.sql`（v(N-1) → vN），`deploy/db/convert.py` 在一个事务里执行，出错整体回滚。调试期也可以直接删库（连同 `-wal`、`-shm`），启动时用当前结构重建；正式使用后每次改表结构都必须带转换脚本。
