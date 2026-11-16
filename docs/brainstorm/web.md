# 头脑风暴：v2 网页

状态：方向讨论，未定功能，不实现。

## 定位

iPhone Safari 里打开、添加到主屏幕用，看起来像原生 App。桌面能用就行，不专门适配。

## UI 方向：贴近 iOS

- **PWA**：`manifest` + `apple-mobile-web-app-capable`，主屏幕图标、全屏无地址栏；`viewport-fit=cover` + `env(safe-area-inset-*)` 避开刘海和 Home 条。
- **系统观感**：字体 `-apple-system`；配色用 iOS 系统色（背景 `#F2F2F7`、分组卡片白底、分隔线 0.5px），跟随深色模式（`prefers-color-scheme`）。
- **列表即主体**：iOS「设置」式分组圆角列表（inset grouped），左滑删除、点击进详情；顶部大标题，滚动后收成小标题。
- **底部标签栏**：账目 / 统计（v3）/ 设置（分类、账户、标签），拇指可达。
- **编辑用底部弹层**（sheet），原生控件优先：`<input type="date|time">`、`inputmode="decimal"` 调出数字键盘、`<select>` 调出系统滚轮。
- **触感**：点击态（按压变灰）、44pt 最小点击区、`overscroll` 回弹保留、禁用双击缩放。
- 不引 UI 组件库：Svelte + 少量手写 CSS 变量就能做到，体积小、风格可控。

## 鉴权

浏览器不方便带 Bearer：登录页输入同一个 token，换成 HttpOnly Cookie（长期有效，单用户）。待定。

## 候选功能（待你挑）

- 按月账目列表：按天分组，显示当天合计；顶部月份切换和本月收支。
- 账目详情 / 编辑 / 删除（编辑沿用分类、账户、标签选择）。
- 分类、账户、标签管理（替代命令行；有账目的不能删，同现有规则）。
- 识别记录回看：截图、模型原始输出、用户改了哪些字段（帮助调 prompt）。数据已经在库里：`recognitions.origin` 是模型给的草稿，按 `draft_id` 连 `entries` 就是最终值，网页做成「改过的字段」列表即可。
- **更新按钮**（定了要做）：照 micnext 的做法——`git fetch` + `reset --hard @{u}` → `./build.sh` → 构建成功后旧进程退出、以原参数 exec 新程序（screen 里接着跑，库转换和备份走 `build.sh start` 同一路径）；失败时旧服务继续跑，页面显示阶段和输出末尾。
- CSV 导出。
- 搜索 / 按标签筛选（可能放 v3）。

## 开发者诊断（日志、SQL）

micnext 已有「开发者诊断 → 日志 / SQL」。两种复用方式：

- **micnext 读 micjz 的库**（建议先这样）：micnext 的 SQL 页加一个「选库」，配置里列出额外的只读库路径（同一台服务器，`mode=ro` 打开）。micjz 不用写任何诊断代码；改动在 micnext，按它的蓝图流程走。
- **micjz 自带**：照抄 micnext 的只读 SQL 页和日志页。代码要复制一份，两边各维护。

日志：micjz 目前只打到 screen 终端。micnext 的日志页读的是它自己的日志库，要看 micjz 日志得让 micjz 也落库，等真有需要再说。

## 待定

1. 先做哪几项？（建议：列表 + 编辑删除 + 分类管理，三项就是一个可用的 v2）
2. 鉴权方式：token 换 Cookie，还是反代层（Caddy basic auth）兜住？
3. 是否需要离线查看（Service Worker 缓存）？建议不做，和「无网不记账」一致。
