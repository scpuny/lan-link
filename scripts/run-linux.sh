#!/usr/bin/env bash
# LanLink Linux 启动脚本
#
# 用途：在虚拟机 / 无 GPU / 远程桌面（xrdp、Xvfb）等没有可用硬件 OpenGL 的环境里
# 稳定启动 LanLink。程序内部已按 DMI 信息自动识别虚拟机，这里再显式兜一层，
# 同时适用于「宿主机识别不到但确实没有 GL」的场景（例如 xrdp 会话）。
#
# 用法：
#   ./scripts/run-linux.sh                 # 用默认路径查找已安装的 lanlink
#   ./scripts/run-linux.sh /path/to/lanlink
#   FORCE_SOFTWARE_GL=0 ./scripts/run-linux.sh   # 有真实 GPU 时关掉软件渲染

set -euo pipefail

BIN="${1:-}"

if [[ -z "$BIN" ]]; then
  for candidate in \
    "/usr/bin/lanlink" \
    "/usr/local/bin/lanlink" \
    "$HOME/.local/bin/lanlink" \
    "$(dirname "$0")/../target/debug/lanlink-desktop" \
    "$(dirname "$0")/../target/release/lanlink-desktop"
  do
    if [[ -x "$candidate" ]]; then BIN="$candidate"; break; fi
  done
fi

if [[ -z "$BIN" || ! -x "$BIN" ]]; then
  echo "找不到 lanlink 可执行文件，请显式指定路径：$0 /path/to/lanlink" >&2
  exit 1
fi

# WebKitGTK 在无 GPU 时会在合成器/DMABUF 上卡死，这两项是官方认可的关闭开关。
if [[ "${FORCE_SOFTWARE_GL:-1}" != "0" ]]; then
  export WEBKIT_DISABLE_COMPOSITING_MODE=1
  export WEBKIT_DISABLE_DMABUF_RENDERER=1
  export LIBGL_ALWAYS_SOFTWARE=1
fi

# WebKit 的进程沙箱在部分容器/精简发行版里会拒绝启动，按需打开：
#   SANDBOX=0 ./scripts/run-linux.sh
if [[ "${SANDBOX:-1}" == "0" ]]; then
  export WEBKIT_FORCE_SANDBOX=0
fi

echo "[run-linux] bin=$BIN"
echo "[run-linux] WEBKIT_DISABLE_COMPOSITING_MODE=${WEBKIT_DISABLE_COMPOSITING_MODE:-<unset>}"
echo "[run-linux] WEBKIT_DISABLE_DMABUF_RENDERER=${WEBKIT_DISABLE_DMABUF_RENDERER:-<unset>}"

exec "$BIN"
