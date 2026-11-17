#!/bin/sh
# LanLink 容器入口：准备运行环境后拉起应用
set -e

# 容器里没有 pam_systemd，/run/user/UID 不一定存在，用 /tmp 兜底
if [ -d "/run/user/$(id -u)" ] && [ -w "/run/user/$(id -u)" ]; then
  : "${XDG_RUNTIME_DIR:=/run/user/$(id -u)}"
else
  : "${XDG_RUNTIME_DIR:=/tmp/runtime-$(id -u)}"
fi
export XDG_RUNTIME_DIR
mkdir -p "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"

# 容器内没有自己的 D-Bus 会话总线，用 dbus-run-session 包一层
exec dbus-run-session -- /usr/local/bin/lanlink-desktop "$@"
