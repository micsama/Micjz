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
micjz category move   支出 '🍜 餐饮/☕️ 咖啡饮品' 1  # 挪到同级第 1 位

micjz account list                               # 按优先级列出：名称、卡类型和完整卡号、账目数
micjz account add  现金
micjz account add  招商银行信用卡 信用卡 "6225 8888 0000 1234"
micjz account move 招商银行信用卡 1                    # 第 1 位是识别不出账户时的默认账户
micjz account rename 现金 零钱 | rm 零钱
micjz tag     list | add 出差 | rename 出差 差旅 | rm 差旅

micjz /path/to/config.toml category list         # 指定配置文件
```

- 类型写 `支出` 或 `收入`；分类路径是 `一级` 或 `一级/二级`，名称要写全（含开头的 emoji），带空格就加引号。
- 分类名开头的 emoji 是名称的一部分：快捷指令原样显示，网页取它当图标。改 emoji 就是改名：`micjz category rename 支出 '🍜 餐饮' '🍚 餐饮'`。
- 每个分类必须有 key：小写英文、数字或下划线，全局唯一。模型只看叶子分类（没有二级的一级、二级）的 key。
- 有账目的分类、账户、标签不能删；一级下还有二级时要先删二级；兜底的「其他」（按 key 认）不能删，可以改名；最后一个账户不能删。
- 新加的分类、账户排在同级最后，用 `move` 调整优先级：决定菜单顺序和模型分不清时的先后。名称不能含 `/` 和换行。
- 银行账户用银行全称起名（如「招商银行信用卡」）：截图里显示的是全称，模型比对更准；网页可单独设置机构，决定 logo；模型会看到机构和卡尾号。
- 卡号完整存在库里，`account list` 可复制核对；发给模型的只有卡类型和尾号。

## 部署

服务器上用 screen 常驻（同 micnext）：

```sh
screen -S micjz
./build.sh run              # 构建、按需转换数据库、前台运行；Ctrl-A D 挂到后台
```

更新：`git pull && ./build.sh`（构建期间旧服务照跑），然后 `screen -r micjz`，Ctrl-C 停掉旧服务，`./build.sh start` 启动新版本。

- `./build.sh [debug] [build|run|start] [config.toml]`：`build` 只构建并检查库；`run` 构建后启动；`start` 不构建直接启动。每次启动前都备份成 `data/micjz.db.v<版本>-<时间>.bak`（不自动清理），版本落后再转换。
- 首次部署：`config.example.toml` 复制为 `config.toml`；启动前 `export DEEPSEEK_API_KEY=...`。需要 Rust 工具链、Python ≥ 3.11、Bun 和 `rsvg-convert`（Debian/Ubuntu 装 `librsvg2-bin`）。构建脚本先安装前端锁定依赖、构建网页、从 `icon.svg` 生成主屏 PNG 图标，再把静态资源嵌入二进制；运行二进制时无需 Bun。

## 初始化 / 重建库

首次部署或调试期删库重建时，按这个顺序（以服务器上为例）：

```sh
git pull && ./build.sh                  # 构建；旧服务照跑，库检查报版本落后可忽略
screen -r micjz                         # Ctrl-C 停掉旧服务
rm -f data/micjz.db data/micjz.db-wal data/micjz.db-shm   # 删库：账目、分类、账户、标签全清空

alias micjz=./target/release/micjz     # 下面的命令都在项目目录下执行
micjz category list                     # 第一次执行就按当前结构建出新库，带默认数据
```

新库自带 `src/seed.sql` 里的默认分类、账户（「微信」「银行卡」）和标签（「恋爱」），在默认数据上改成自己的：

```sh
# 账户：银行卡用全称，带卡类型和完整卡号
micjz account add  招商银行储蓄卡 储蓄卡 "6214 8888 0000 1234"
micjz account add  招商银行信用卡 信用卡 "6225 8888 0000 5678"
micjz account rm   银行卡                   # 删掉占位账户（没有账目才能删）
micjz account move 招商银行储蓄卡 1           # 第 1 位是识别不出账户时的默认账户
micjz account list                         # 核对顺序和卡号

# 分类、标签：增删改、调顺序
micjz category add  支出 宠物 pet
micjz category move 支出 餐饮/咖啡饮品 1
micjz category list
micjz tag add 出差
```

最后启动服务：`./build.sh start`，Ctrl-A D 挂到后台。服务运行期间也能随时用命令行改，立即生效，不用重启。

- 删库前手机上没走完的流程，提交时会提示「草稿不存在」，重新截图即可；快捷指令不用改。
- 正式使用后不再删库：改表结构走转换脚本，见下。

数据库版本存在 `PRAGMA user_version`，程序版本在 `src/schema_version`，不一致就拒绝启动。改表结构时版本加一并写转换脚本 `deploy/db/<N>-*.sql`（v(N-1) → vN），`deploy/db/convert.py` 在一个事务里执行，出错整体回滚。调试期也可以直接删库（连同 `-wal`、`-shm`），启动时用当前结构重建；正式使用后每次改表结构都必须带转换脚本。

## 网页账号

先执行 `micjz [config.toml] password`，按提示输入两次密码（至少 8 位，终端不回显，也可用管道输入）。网页用户名固定为 `dzmfg`；密码存 Argon2id 哈希，不使用快捷指令的 token。服务运行中也可通过这个命令重设密码，旧会话立即失效。

登录会话有效期 30 天，使用超过一天后自动续期；连续 30 天未使用需要重新登录。设置页可以修改密码，其他设备需重新登录，当前设备保持登录。表结构 v6 同时增加账户机构和网页账号；通过 `./build.sh start` 备份、转换 v5 数据库，不要重建已有账本。
