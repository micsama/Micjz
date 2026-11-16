#!/usr/bin/env python3
"""把旧库转换到当前表结构；由 build.sh 调用，二进制本身只认当前结构。

版本存在库的 PRAGMA user_version，目标版本读 src/schema_version；
v(N-1) → vN 的脚本是本目录下的 N-*.sql，按顺序在一个事务里执行，任一步失败全部回滚。
--apply 每次都先备份（不论是否需要转换），备份放在库旁边，不自动清理。
用法：convert.py [--config config.toml] (--check | --apply)
"""
import argparse
import sqlite3
import sys
import time
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def current(conn):
    """库的版本；None 表示空库（二进制会直接建）。"""
    tables = {r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type = 'table'")}
    if not tables:
        return None
    return conn.execute("PRAGMA user_version").fetchone()[0]


def plan(actual, target):
    if actual > target:
        raise RuntimeError(f"库 v{actual} 高于程序 v{target}，不支持反向转换")
    steps = []
    for version in range(actual + 1, target + 1):
        found = sorted(HERE.glob(f"{version}-*.sql"))
        if len(found) != 1:
            raise RuntimeError(
                f"缺少 v{version - 1} → v{version} 的转换脚本 {version}-*.sql；"
                "调试期的库可以直接删掉（含 -wal/-shm），启动时重建"
            )
        steps.append((version, found[0]))
    return steps


def statements(path):
    pending = ""
    for line in path.read_text().splitlines(keepends=True):
        pending += line
        if sqlite3.complete_statement(pending):
            yield pending
            pending = ""
    if pending.strip():
        raise RuntimeError(f"{path.name} 有未结束的 SQL")


def backup(conn, db, version):
    dest = db.with_name(f"{db.name}.v{version}-{time.strftime('%Y%m%d-%H%M%S')}.bak")
    with sqlite3.connect(dest) as out:
        conn.backup(out)
    out.close()
    print(f"已备份 {dest}", flush=True)


def convert(conn, steps):
    # 重建带外键的表需要关闭约束；提交前检查整个结果。
    conn.execute("PRAGMA foreign_keys = OFF")
    conn.execute("BEGIN IMMEDIATE")
    try:
        for version, path in steps:
            print(f"转换 v{version - 1} → v{version}（{path.name}）", flush=True)
            for sql in statements(path):
                conn.execute(sql)
            conn.execute(f"PRAGMA user_version = {version}")
        violations = conn.execute("PRAGMA foreign_key_check").fetchall()
        if violations:
            raise RuntimeError(f"转换后外键检查失败：{violations}")
        conn.execute("COMMIT")
    except BaseException:
        conn.execute("ROLLBACK")
        raise


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--config", default="config.toml")
    mode = ap.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true", help="只报告，不改库")
    mode.add_argument("--apply", action="store_true", help="备份后转换")
    args = ap.parse_args()

    db = Path(tomllib.loads(Path(args.config).read_text())["db_path"])
    target = int((ROOT / "src/schema_version").read_text())
    if not db.exists():
        print(f"数据库 {db} 不存在，启动时新建 v{target}")
        return
    conn = sqlite3.connect(db, isolation_level=None)
    actual = current(conn)
    if actual is None:
        print(f"数据库 {db} 是空库，启动时新建 v{target}")
        return
    steps = plan(actual, target)
    if steps:
        print(f"数据库 {db}：v{actual} → v{target}，待转换 {len(steps)} 步", flush=True)
    else:
        print(f"数据库 {db}：v{actual}，无需转换", flush=True)
    if args.check:
        return
    backup(conn, db, actual)
    if not steps:
        return
    convert(conn, steps)
    print(f"转换完成：v{target}")


if __name__ == "__main__":
    try:
        main()
    except Exception as e:  # noqa: BLE001 —— 给人看的一行原因
        print(f"数据库转换失败：{e}", file=sys.stderr)
        sys.exit(1)
