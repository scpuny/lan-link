#!/usr/bin/env bash
# LanLink 可移植二进制构建脚本（双机联调用，无需发包）
#
# 用法：
#   ./scripts/build-portable.sh            # debug 构建（快，联调够用）
#   ./scripts/build-portable.sh --release  # release 构建（体积小、运行快，编译久）
#
# 产物：target/{debug,release}/lanlink-desktop
# 该文件已内嵌前端（web/dist），只依赖 macOS 系统框架，可直接拷到同架构机器运行。
#
# ⚠️ 关键：必须用 `tauri build`，不能用 `cargo build`。
#    debug profile 下直接 `cargo build` 编出的二进制仍指向 devUrl(127.0.0.1:5173)，
#    脱离 Vite 运行时界面白屏。`tauri build` 才会把 frontendDist 真正嵌入。

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PROFILE="debug"
TAURI_FLAGS=(--debug --no-bundle)
if [[ "${1:-}" == "--release" ]]; then
  PROFILE="release"
  TAURI_FLAGS=(--no-bundle)
fi

TAURI_BIN="$ROOT/web/node_modules/.bin/tauri"
if [[ ! -x "$TAURI_BIN" ]]; then
  echo "错误：未找到本地 tauri CLI（$TAURI_BIN）" >&2
  echo "请先在 web 目录执行 npm install" >&2
  exit 1
fi

echo "==> 构建前端 + 嵌入二进制（profile: $PROFILE，tauri build 会自动跑 beforeBuildCommand）"
# tauri build 必须在 crates/lanlink-desktop 下执行（beforeBuildCommand 的 CWD 是 crates/，脚本里用 --prefix ../web）
cd "$ROOT/crates/lanlink-desktop"
"$TAURI_BIN" build "${TAURI_FLAGS[@]}"

BIN="$ROOT/target/$PROFILE/lanlink-desktop"

echo
echo "==================== 构建完成 ===================="
printf '产物路径 : %s\n' "$BIN"
printf '文件体积 : %s\n' "$(du -h "$BIN" | cut -f1)"
printf '当前架构 : %s\n' "$(uname -m)"
printf '二进制架构: %s\n' "$(file -b "$BIN" | sed 's/,.*//')"

# 自检：确认前端已内嵌（应能找到 hashed 资源名）
# 注意：不能用 `strings ... | grep -q`——grep -q 匹配后立即退出会让 strings 收到
# SIGPIPE(141)，配合 set -o pipefail 会把整个管道误判为失败。先落盘再匹配。
STRINGS_DUMP="$(mktemp)"
strings "$BIN" 2>/dev/null > "$STRINGS_DUMP" || true
if grep -qE "assets/index-[A-Za-z0-9_-]+\.js" "$STRINGS_DUMP"; then
  echo "前端内嵌 : ✓ 已嵌入 web/dist 资源"
else
  echo "前端内嵌 : ✗ 未检测到内嵌资源，运行会白屏！" >&2
fi
rm -f "$STRINGS_DUMP"

echo
echo "动态库依赖（应只有系统框架）："
otool -L "$BIN" | tail -n +2 | awk '{print "  " $1}'
echo
echo "拷到另一台【同架构】机器后："
echo "  chmod +x lanlink-desktop && ./lanlink-desktop"
echo "跨架构（如 Intel ↔ Apple Silicon）不可直接拷，需在目标机本地编译或启用 Rosetta。"
echo "=================================================="
