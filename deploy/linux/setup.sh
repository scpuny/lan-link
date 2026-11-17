#!/usr/bin/env bash
# ==============================================================================
# LanLink Linux 运行环境准备（容器方式）
#
# 适用场景：目标机 webkit2gtk 版本不达标（如 Ubuntu 18.04 只有 4.0），
#          而 Tauri v2 要求 4.1 + libsoup3。用 ubuntu:22.04 容器兜住。
#
# 用法（在目标机上，用户需在 docker 组）：
#   curl -fsSL https://raw.githubusercontent.com/scpuny/lan-link/dev/deploy/linux/setup.sh | bash
# 或指定自定义产物地址：
#   bash setup.sh <tar.gz 的 URL 或本地路径>
#
# 前置：docker 已安装且当前用户可直接 `docker ps`
# ==============================================================================
set -euo pipefail

REPO_RAW="${REPO_RAW:-https://raw.githubusercontent.com/scpuny/lan-link/dev/deploy/linux}"
ARTIFACT="${1:-https://github.com/scpuny/lan-link/releases/download/linux-latest/lanlink-linux-x86_64.tar.gz}"
WORK="${WORK:-$HOME/.local/share/lanlink}"
IMAGE="${IMAGE:-lanlink:linux}"

info() { printf '\033[1;36m[lanlink]\033[0m %s\n' "$*"; }
fail() { printf '\033[1;31m[error]\033[0m %s\n' "$*" >&2; exit 1; }

command -v docker >/dev/null || fail "未安装 docker"
docker ps >/dev/null 2>&1 || fail "当前用户无权访问 docker daemon（把用户加进 docker 组后重新登录）"

mkdir -p "$WORK"
cd "$WORK"

info "工作目录 $WORK"

info "下载运行脚本与 Dockerfile"
curl -fsSL "$REPO_RAW/Dockerfile"     -o Dockerfile
curl -fsSL "$REPO_RAW/entrypoint.sh"  -o entrypoint.sh
chmod 644 Dockerfile entrypoint.sh

info "获取产物：$ARTIFACT"
rm -f lanlink.tar.gz
if [[ "$ARTIFACT" == http* ]]; then
  curl -fL --progress-bar "$ARTIFACT" -o lanlink.tar.gz
else
  cp -f "$ARTIFACT" lanlink.tar.gz
fi
tar -xzf lanlink.tar.gz
[[ -x ./lanlink-desktop ]] || { chmod +x ./lanlink-desktop 2>/dev/null || true; }
[[ -f ./lanlink-desktop ]] || fail "压缩包里没有 lanlink-desktop"
ls -lh lanlink-desktop

info "构建镜像 $IMAGE（首次约 1-3 分钟）"
docker build -t "$IMAGE" .

info "写入启动器 run.sh"
cat > run.sh <<'RUNEOF'
#!/usr/bin/env bash
# 在图形会话里启动 LanLink 容器
#   --network host  ：容器直接使用宿主网络，UDP 42211 广播 / TCP 42210 才能正常工作
#   -v $HOME        ：文件选择对话框里能看到并读写宿主的家目录
set -euo pipefail
IMAGE="${IMAGE:-lanlink:linux}"
: "${DISPLAY:?没有 DISPLAY，请在图形会话（远程桌面/本地桌面）里运行}"
XAUTH="${XAUTHORITY:-$HOME/.Xauthority}"
[[ -r "$XAUTH" ]] || { echo "读不到 XAUTHORITY 文件：$XAUTH" >&2; exit 1; }

exec docker run --rm -i \
  --name lanlink \
  --network host \
  --shm-size=512m \
  --user "$(id -u):$(id -g)" \
  -e DISPLAY="$DISPLAY" \
  -e HOME=/host-home \
  -e XAUTHORITY=/xauth \
  -v /tmp/.X11-unix:/tmp/.X11-unix \
  -v "$HOME:/host-home" \
  -v "$XAUTH:/xauth:ro" \
  "$IMAGE" "$@"
RUNEOF
chmod 755 run.sh

info "写入桌面快捷方式"
mkdir -p "$HOME/.local/share/applications" "$HOME/Desktop"
cat > "$HOME/.local/share/applications/lanlink.desktop" <<DESKEOF
[Desktop Entry]
Type=Application
Version=1.0
Name=LanLink
Name[zh_CN]=LanLink 局域网传输
Comment=局域网 / 跨网 P2P 文件传输
Exec=$WORK/run.sh
Icon=network-transmit-receive
Terminal=false
Categories=Network;FileTransfer;
StartupNotify=true
DESKEOF
chmod 755 "$HOME/.local/share/applications/lanlink.desktop"
cp -f "$HOME/.local/share/applications/lanlink.desktop" "$HOME/Desktop/lanlink.desktop"
chmod 755 "$HOME/Desktop/lanlink.desktop"
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
fi

echo
info "完成。启动方式二选一："
echo "  1) 图形会话里双击桌面图标 / 应用菜单里的 LanLink"
echo "  2) 终端执行：DISPLAY=\${DISPLAY:-:10} $WORK/run.sh"
echo
info "自检：docker images | grep lanlink"
