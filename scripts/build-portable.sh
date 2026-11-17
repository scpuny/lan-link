#!/usr/bin/env bash
# LanLink 可移植二进制构建脚本（双机联调用，无需发包）
#
# 用法：
#   ./scripts/build-portable.sh            # debug 构建（快，联调够用）
#   ./scripts/build-portable.sh --release  # release 构建（体积小、运行快，编译久）
#
# 产物：target/{debug,release}/lanlink-desktop
# 该文件已内嵌前端（web/dist），只依赖 macOS 系统框架，可直接拷到同架构机器运行。

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

PROFILE="debug"
CARGO_FLAGS=()
if [[ "${1:-}" == "--release" ]]; then
  PROFILE="release"
  CARGO_FLAGS+=(--release)
fi

echo "==> [1/2] 构建前端（web/dist）"
npm --prefix web run build

echo "==> [2/2] 构建桌面二进制（profile: $PROFILE）"
cargo build -p lanlink-desktop "${CARGO_FLAGS[@]}"

BIN="$ROOT/target/$PROFILE/lanlink-desktop"

echo
echo "==================== 构建完成 ===================="
printf '产物路径 : %s\n' "$BIN"
printf '文件体积 : %s\n' "$(du -h "$BIN" | cut -f1)"
printf '当前架构 : %s\n' "$(uname -m)"
printf '二进制架构: %s\n' "$(file -b "$BIN" | sed 's/,.*//')"
echo
echo "动态库依赖（应只有系统框架）："
otool -L "$BIN" | tail -n +2 | awk '{print "  " $1}'
echo
echo "拷到另一台【同架构】机器后："
echo "  chmod +x lanlink-desktop && ./lanlink-desktop"
echo "跨架构（如 Intel ↔ Apple Silicon）不可直接拷，需在目标机本地编译或启用 Rosetta。"
echo "=================================================="
