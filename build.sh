#!/bin/sh
# 构建并按需转换数据库后启动。用法见 readme.md「部署」。
set -eu
cd "$(dirname "$0")"
repo_dir=$PWD

profile=release
if [ "${1-}" = debug ]; then
    profile=debug
    shift
fi

mode=${1-build}
if [ "$#" -gt 0 ]; then shift; fi
config=${1-config.toml}
bin="$repo_dir/target/$profile/micjz"
convert() { python3 deploy/db/convert.py --config "$config" "$@"; }

build() {
    command -v cargo >/dev/null 2>&1 || { echo "请先手动安装 Rust 工具链（cargo）" >&2; exit 1; }
    command -v python3 >/dev/null 2>&1 || { echo "请先手动安装 Python 3（≥ 3.11）" >&2; exit 1; }
    command -v bun >/dev/null 2>&1 || { echo "请先手动安装 Bun" >&2; exit 1; }
    (cd web && bun install --frozen-lockfile && bun run build)
    MICJZ_VERSION=$(git rev-parse --short HEAD)
    export MICJZ_VERSION
    case "$profile" in
        release) cargo build --release --locked --config 'profile.release.strip="symbols"' ;;
        debug) cargo build --locked ;;
    esac
    printf '\n构建完成：target/%s/micjz\n' "$profile"
}

case "$mode" in
    # 只构建并检查库：旧服务可能还在用库，转换留到 start。
    build)
        build
        convert --check
        echo '上线：在 screen 里 Ctrl-C 停掉旧服务，再 ./build.sh start（转换数据库后启动）'
        ;;
    # 本地：构建、转换、前台运行。
    run)
        build
        convert --apply
        exec "$bin" "$config"
        ;;
    # 不构建，备份并转换后启动：screen 里重启用，网页更新构建完也 exec 它。
    start)
        [ -x "$bin" ] || { echo "缺少 $bin，先运行 ./build.sh" >&2; exit 1; }
        convert --apply
        exec "$bin" "$config"
        ;;
    *)
        printf '用法：%s [debug] [build|run|start] [config.toml]\n' "$0" >&2
        exit 1
        ;;
esac
