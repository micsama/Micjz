# 蓝图：分类、账户、标签进库

状态：已实现。管理命令见 `readme.md`。

## 表结构

```sql
categories (id INTEGER PK, kind, parent_id → categories NULL=一级, name, key NOT NULL UNIQUE)
  UNIQUE (kind, coalesce(parent_id, 0), name)
accounts   (id INTEGER PK, name UNIQUE)
tags       (id INTEGER PK, name UNIQUE)
entries.category_id → categories（最细一级）
entries.account_id  → accounts NULL
entry_tags (entry_id, tag_id → tags)
```

- 账目按 id 引用：改名不影响历史账目。
- 外键兜底：有账目或二级的分类、有账目的账户和标签删不掉；命令行先查，给出「X 下有 N 笔账」。
- 顺序按 `position`（同级内的优先级，`move` 调整），即菜单顺序，也是模型分类模糊时的优先级；账户的第一位是识别不出账户时的默认值。见[表结构梳理](single-source.md)。

## 规则

- 每个分类必须有 key（小写英文 / 数字 / 下划线，全局唯一）；只有叶子的 key 写进 prompt。
- 「其他」（收支各一）是兜底分类，不可删、不可改名。
- 名称不能含 `/`（路径分隔）和换行（多选结果按换行合并）。
- 每个请求读一份快照，不缓存；改了立即生效。
- 草稿里存名称：草稿期间分类被改名或删除，`/step` 返回 400，重新截图即可。

## 初始数据与迁移

- 新库：`categories` 为空时执行 `src/seed.sql`（编进二进制）。数据库是唯一来源，没有配置文件。

## 管理入口

命令行：`micjz [config.toml] category|account|tag ...`，ssh 上去执行。v2 网页复用 `catalog.rs` 里同一套操作。
