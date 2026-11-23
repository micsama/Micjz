# 快捷指令「记一笔」

通用渲染器：只按服务器返回的 `type` 弹界面，不含任何业务内容。协议见 `docs/roadmap.md`「交互协议」。
在 Mac 的「快捷指令」App 里搭，iCloud 同步到手机。业务改动只改服务器，这里理论上搭一次就够。

## 生成（推荐）

```sh
python3 shortcuts/build.py                          # 默认 http://127.0.0.1:8730
python3 shortcuts/build.py http://192.168.1.5:8730  # 手机连 Mac
python3 shortcuts/build.py https://<服务器地址>/micjz --token <服务器 token>  # 服务器（Caddy 按 /micjz 前缀转发）
```

token 须与服务端 `config.toml` 的 `token` 一致；不传 `--token` 时读本地 `config.toml`。生成后也可在快捷指令第二个【文本】动作里直接改。
产物 `shortcuts/记一笔.shortcut`（含 token，已 gitignore），双击导入。
下面的动作清单是生成内容的说明，也可手动对照排查。

## 动作

括号内为动作名，可在右侧动作库搜索。

**准备**

1. 【文本】服务地址 →【设定变量】`base`
   - Mac 本地调试：`http://127.0.0.1:8730`
   - 手机连 Mac：`http://<Mac 局域网 IP>:8730`（`ipconfig getifaddr en0`；`config.toml` 的 listen 改为 `0.0.0.0:8730`）
   - 服务器：`https://<服务器地址>/micjz`（以后换域名只改这里）
2. 【文本】`config.toml` 里的 token →【设定变量】`token`
3. 【截屏】
4. 【获取 URL 内容】
   - URL：`base`/recognize，方法 POST
   - 头部：`Authorization` = `Bearer token`
   - 请求体：表单，键 `image`，类型「文件」，值「截屏」
5. 【设定变量】`resp`

**循环**

6. 【重复】50 次（上限，正常靠 `done` 或取消结束）
7. 　【获取字典值】`type`（字典：`resp`）→【设定变量】`type`
8. 　【获取字典值】`summary`（字典：`resp`）→【设定变量】`summary`
9. 　【如果】`type` 是 `done`
   - 【获取字典值】`message`（`resp`）→【显示通知】
   - 【获取字典值】`draft`（`resp`）→【设定变量】`draft`
   - 【如果】`draft` 有任何值（入账成功才非空），补采位置，失败只丢位置：
     - 【获取当前位置】→【获取位置的详细信息】纬度、经度（各一个动作）
     - 【获取 URL 内容】`base`/location，POST，头部同上，JSON 文本字段 `draft` / `lat` / `lon`；不读响应
   - 【停止此快捷指令】
10. 　【如果】`type` 是 `choose`
    - 【获取字典值】`options`（`resp`）→【从列表中选取】，提示：`summary` →【设定变量】`value`
11. 　【如果】`type` 是 `choose_many`
    - 【获取字典值】`options`（`resp`）→【从列表中选取】打开「选择多个」，提示：`summary`
    - 【合并文本】分隔符「换行」→【设定变量】`value`
12. 　【如果】`type` 是 `number`
    - 【获取字典值】`default`（`resp`）→【要求输入】类型「数字」，提示 `summary`，默认值为上一步 →【设定变量】`value`
13. 　【如果】`type` 是 `text`
    - 同上，类型「文本」
14. 　【如果】`type` 是 `date`
    - 同上，类型「日期和时间」→【格式化日期】ISO 8601 →【设定变量】`value`
15. 　【获取字典值】`draft`（`resp`）→【设定变量】`draft`
16. 　【获取字典值】`field`（`resp`）→【设定变量】`field`
17. 　【获取 URL 内容】
    - URL：`base`/step，方法 POST，头部同上
    - 请求体：JSON，三个文本字段 `draft` / `field` / `value`
18. 　【设定变量】`resp`
19. 【结束重复】

## 触发

- 操作按钮：设置 → 操作按钮 → 快捷指令 → 「记一笔」
- 背面轻点：设置 → 辅助功能 → 触控 → 轻点背面 → 轻点两下 → 「记一笔」

## 待实机确认

- 「从列表中选取」/「要求输入」的提示语能否插入变量、能显示几行。
- 从操作按钮触发时，截屏截到的是否为支付页。
- 首次运行的授权弹窗（网络访问、截屏、位置）是否只出现一次；位置授权在「已记账」通知之后弹，会不会让人以为没记完。
- 拒绝定位、没信号、看到通知立刻锁屏时：账照记，位置能否补上（见 `docs/blueprints/location.md`）。
