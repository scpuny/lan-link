#!/usr/bin/env bash
# ==============================================================================
# install-xrdp-xfce.sh
# 为 Ubuntu 18.04 (bionic) 无头服务器安装 xrdp + Xfce 远程桌面
#
# 用法（在 192.168.1.11 上执行）：
#   sudo bash install-xrdp-xfce.sh <远程登录用户名>
# 例：
#   sudo bash install-xrdp-xfce.sh yangbo
#
# 特性：
#   - 幂等：可重复执行，已装的包不会重复安装
#   - bionic 已 EOL，自动探测并按需把 apt 源切到 old-releases.ubuntu.com
#   - 装 Xfce 轻量桌面（约 400MB 内存）+ 中文字体
#   - 修 startwm.sh 的经典黑洞（dbus / XDG_RUNTIME_DIR / 直接 exec startxfce4）
#   - 关屏保+DPMS，避免远程看着看着黑屏
#   - 自动放行 3389（ufw / iptables）
#   - 结束自检：监听端口、服务状态、连接命令
# ==============================================================================
set -euo pipefail

C_RESET='\033[0m'; C_INFO='\033[1;36m'; C_WARN='\033[1;33m'; C_ERR='\033[1;31m'; C_OK='\033[1;32m'
log()  { printf "${C_INFO}[xrdp]${C_RESET} %s\n" "$*"; }
ok()   { printf "${C_OK}[ ok ]${C_RESET} %s\n" "$*"; }
warn() { printf "${C_WARN}[warn]${C_RESET} %s\n" "$*"; }
die()  { printf "${C_ERR}[fail]${C_RESET} %s\n" "$*" >&2; exit 1; }

# ---------------------------------------------------------------- 前置检查
[[ ${EUID} -eq 0 ]] || die "请以 root 执行：sudo bash $0 <用户名>"

TARGET_USER="${1:-${SUDO_USER:-}}"
if [[ -z "${TARGET_USER}" || "${TARGET_USER}" == "root" ]]; then
  die "用法: sudo bash $0 <远程登录用户名>（不要用 root，xrdp 以普通用户会话运行）"
fi
id "${TARGET_USER}" >/dev/null 2>&1 || die "用户 ${TARGET_USER} 不存在，请先 useradd"
TARGET_HOME="$(getent passwd "${TARGET_USER}" | cut -d: -f6)"
[[ -d "${TARGET_HOME}" ]] || die "取不到 ${TARGET_USER} 的家目录"

# shellcheck disable=SC1091
. /etc/os-release
log "系统 ${PRETTY_NAME:-unknown} / 架构 $(uname -m) / 会话用户 ${TARGET_USER} (${TARGET_HOME})"
case "${VERSION_ID:-}" in
  18.04) : ;;
  *) warn "本脚本按 18.04 验证过，当前 ${VERSION_ID:-?} 上仍会尝试执行，但 xrdp 包名可能有差异" ;;
esac

export DEBIAN_FRONTEND=noninteractive

# ---------------------------------------------------------------- apt 源
# 注意：不能只看 apt-get update 的退出码。
# 机器上任意一个第三方源（如 download.docker.com）坏掉就会让 update 返回非零，
# 但那不代表主源不可用。真正要确认的是「xrdp 有没有可安装的候选版本」。
pkg_available() {
  local cand
  cand="$(apt-cache policy "$1" 2>/dev/null | awk '/Candidate:|候选/{print $2; exit}')"
  [[ -n "${cand}" && "${cand}" != "(none)" && "${cand}" != "（无）" ]]
}

apt_refresh() {
  apt-get update -o Acquire::http::Timeout=10 -o Acquire::Retries=1 >/tmp/xrdp-apt-update.log 2>&1 || true
}

apt_refresh
if pkg_available xrdp; then
  ok "apt 源可用，xrdp 候选版本 $(apt-cache policy xrdp | awk '/Candidate:|候选/{print $2; exit}')"
  if grep -qE "^(E|W):" /tmp/xrdp-apt-update.log; then
    warn "部分第三方源报错（不影响安装，已忽略）："
    grep -E "^(E|W):" /tmp/xrdp-apt-update.log | head -5 | sed 's/^/       /'
  fi
else
  warn "主源取不到 xrdp，把源切到 old-releases.ubuntu.com 后重试"
  BAK=/var/backups/xrdp-install
  mkdir -p "${BAK}"
  cp -a /etc/apt/sources.list "${BAK}/sources.list.$(date +%Y%m%d%H%M%S)"
  case "$(uname -m)" in
    x86_64|amd64)
      sed -i -E 's#https?://(archive|security|cn\.archive)\.ubuntu\.com/ubuntu/?#http://old-releases.ubuntu.com/ubuntu#g' \
        /etc/apt/sources.list
      ;;
    aarch64|arm64|armv7l)
      sed -i -E 's#https?://ports\.ubuntu\.com/ubuntu-ports#http://old-releases.ubuntu.com/ubuntu#g' \
        /etc/apt/sources.list
      ;;
  esac
  apt_refresh
  pkg_available xrdp || die "切换源后仍取不到 xrdp，请检查网络/代理（原 sources.list 已备份到 ${BAK}）"
  ok "已切换到 old-releases.ubuntu.com"
fi

# ---------------------------------------------------------------- 安装
log "安装 xrdp / xorgxrdp / Xfce 桌面 / 中文字体（首次约 300-500MB，耐心等）"
apt-get install -y --no-install-recommends \
  -o Dpkg::Options::=--force-confold \
  -o Acquire::Retries=3 \
  xrdp \
  xorgxrdp \
  dbus-x11 \
  x11-xserver-utils \
  xfce4-session \
  xfwm4 \
  xfce4-panel \
  xfdesktop4 \
  xfce4-settings \
  xfce4-terminal \
  xfce4-notifyd \
  xfce4-power-manager \
  xfce4-whiskermenu-plugin \
  thunar \
  gnome-keyring \
  fonts-noto-cjk \
  fonts-dejavu-core \
  || warn "apt 安装过程中有报错（postinst 里 xrdp 启动失败很常见），继续执行修复步骤"

ok "软件包安装完成"

# 关键修复 1：postinst 里 systemctl start 失败会让 dpkg 停在半配置状态
if ! dpkg -l xrdp 2>/dev/null | awk '/^ii/{ok=1} END{exit !ok}'; then
  warn "xrdp 未处于已配置状态，执行 dpkg --configure -a 修复"
  dpkg --configure -a >/dev/null 2>&1 || true
fi

# 关键修复 2：历史 remove 残留会让 conffile 落地成 0 字节，
# 此时 xrdp 一启动就 exit 1 且不打印任何日志（本次实测踩到）。
repair_conffile() {
  local name="$1" dst="/etc/xrdp/$1" deb tmpd
  [[ -s "${dst}" ]] && return 0
  warn "${dst} 缺失或为 0 字节，从 deb 包恢复默认配置"
  deb="$(ls -t /var/cache/apt/archives/xrdp_*.deb 2>/dev/null | head -1)"
  if [[ -z "${deb}" ]]; then
    apt-get install --reinstall --download-only -y xrdp >/dev/null 2>&1 || true
    deb="$(ls -t /var/cache/apt/archives/xrdp_*.deb 2>/dev/null | head -1)"
  fi
  if [[ -z "${deb}" ]]; then
    warn "取不到 xrdp deb，无法恢复 ${name}"
    return 1
  fi
  tmpd="$(mktemp -d)"
  dpkg-deb -x "${deb}" "${tmpd}"
  cp -f "${tmpd}/etc/xrdp/${name}" "${dst}"
  chmod 644 "${dst}"
  rm -rf "${tmpd}"
  ok "已恢复 ${dst}（来自 $(basename "${deb}")）"
}

repair_conffile xrdp.ini || true
repair_conffile sesman.ini || true

# ---------------------------------------------------------------- xrdp 会话脚本
STAMP="$(date +%Y%m%d%H%M%S)"
if [[ -f /etc/xrdp/startwm.sh && ! -f /etc/xrdp/startwm.sh.orig ]]; then
  cp -a /etc/xrdp/startwm.sh /etc/xrdp/startwm.sh.orig
fi

# 注意：这里是 unquoted heredoc 之外的写法——用 quoted 'XEOF' 保证变量在远端 runtime 展开
cat > /etc/xrdp/startwm.sh <<'XEOF'
#!/bin/sh
# 由 lanLink 的 install-xrdp-xfce.sh 生成：直接拉起 Xfce 会话
if [ -r /etc/default/locale ]; then
  . /etc/default/locale
  export LANG LANGUAGE
fi

export XDG_SESSION_TYPE=x11
export XDG_CURRENT_DESKTOP=XFCE
export XDG_SESSION_DESKTOP=xfce

# XDG_RUNTIME_DIR：xrdp 会话不经过 pam_systemd，/run/user/UID 可能不存在
if [ -d "/run/user/$(id -u)" ]; then
  XDG_RUNTIME_DIR="/run/user/$(id -u)"
else
  XDG_RUNTIME_DIR="/tmp/runtime-$(id -u)"
fi
mkdir -p "$XDG_RUNTIME_DIR" 2>/dev/null || true
chmod 700 "$XDG_RUNTIME_DIR" 2>/dev/null || true
export XDG_RUNTIME_DIR

unset DBUS_SESSION_BUS_ADDRESS
unset SESSION_MANAGER
unset SESSION_MANAGER_ARGS

# 关掉黑屏/DPMS（xrdp 会话里 X 已就绪，DISPLAY 由 xrdp 注入）
xset s off -dpms >/dev/null 2>&1 || true

exec dbus-launch --exit-with-session startxfce4
XEOF
chmod 755 /etc/xrdp/startwm.sh
ok "已重写 /etc/xrdp/startwm.sh（原文件备份为 startwm.sh.orig）"

# 用户侧 .xsession 双保险
printf '#!/bin/sh\nexec startxfce4\n' > "${TARGET_HOME}/.xsession"
chmod 755 "${TARGET_HOME}/.xsession"
chown "${TARGET_USER}:$(id -gn "${TARGET_USER}")" "${TARGET_HOME}/.xsession"
# 清掉可能残留的错误权限认证文件（xrdp 常见黑洞之一）
for f in .Xauthority .ICEauthority; do
  if [[ -e "${TARGET_HOME}/${f}" ]]; then
    rm -f "${TARGET_HOME:?}/${f}"
  fi
done
ok "已写入 ${TARGET_HOME}/.xsession 并清理旧 Xauthority"

# ------------------------------------------------- Xfce 首次登录免打扰（实测踩到）
# 坑 1：用户首次登录会弹「欢迎第一次启动面板」，不点掉就没有任务栏。
#       预置一份 panel 配置即可跳过该对话框。
PANEL_DIR="${TARGET_HOME}/.config/xfce4/xfconf/xfce-perchannel-xml"
mkdir -p "${PANEL_DIR}" "${TARGET_HOME}/.config/autostart"
cat > "${PANEL_DIR}/xfce4-panel.xml" <<PEOF
<?xml version="1.0" encoding="UTF-8"?>

<channel name="xfce4-panel" version="1.0">
  <property name="configdir" type="string" value="${TARGET_HOME}/.config/xfce4/panel"/>
  <property name="panels" type="array">
    <value type="int" value="1"/>
    <property name="panel-1" type="empty">
      <property name="position" type="string" value="p=6;x=0;y=0"/>
      <property name="length" type="uint" value="100"/>
      <property name="position-locked" type="bool" value="true"/>
      <property name="size" type="uint" value="30"/>
      <property name="plugin-ids" type="array">
        <value type="int" value="1"/>
        <value type="int" value="2"/>
        <value type="int" value="3"/>
        <value type="int" value="4"/>
        <value type="int" value="5"/>
      </property>
    </property>
  </property>
  <property name="plugins" type="empty">
    <property name="plugin-1" type="string" value="whiskermenu"/>
    <property name="plugin-2" type="string" value="tasklist">
      <property name="grouping" type="bool" value="true"/>
      <property name="show-handle" type="bool" value="false"/>
    </property>
    <property name="plugin-3" type="string" value="separator">
      <property name="expand" type="bool" value="true"/>
    </property>
    <property name="plugin-4" type="string" value="systray"/>
    <property name="plugin-5" type="string" value="clock">
      <property name="digital-layout" type="uint" value="3"/>
      <property name="digital-time-format" type="string" value="%H:%M"/>
    </property>
  </property>
</channel>
PEOF

# 坑 2：nm-applet 自启会触发「解锁密钥环」密码框，服务器上不需要它。
cat > "${TARGET_HOME}/.config/autostart/nm-applet.desktop" <<'PEOF'
[Desktop Entry]
Type=Application
Name=Network
Exec=nm-applet
Hidden=true
X-GNOME-Autostart-enabled=false
PEOF

chown -R "${TARGET_USER}:$(id -gn "${TARGET_USER}")" "${TARGET_HOME}/.config/xfce4" "${TARGET_HOME}/.config/autostart"
ok "已预置 Xfce 面板配置、禁用 nm-applet 自启（避免首次登录弹窗）"

# ---------------------------------------------------------------- xrdp.ini 调优
if [[ -f /etc/xrdp/xrdp.ini && ! -f /etc/xrdp/xrdp.ini.orig ]]; then
  cp -a /etc/xrdp/xrdp.ini /etc/xrdp/xrdp.ini.orig
fi

python3 - <<'PYEOF'
import os, re, sys, tempfile

path = "/etc/xrdp/xrdp.ini"
try:
    with open(path, encoding="utf-8", errors="replace") as f:
        text = f.read()
except FileNotFoundError:
    print("[xrdp] xrdp.ini 不存在，跳过调优")
    sys.exit(0)

if not text.strip():
    print("[xrdp] xrdp.ini 为空，跳过调优（先修复 conffile）")
    sys.exit(0)

wanted = {
    "max_bpp": "32",
    "bitmap_cache": "true",
    "bitmap_compression": "true",
    "tcp_nodelay": "true",
    "tcp_keepalive": "true",
    "use_compression": "true",
}

# 注意：xrdp.ini 里是 [Globals]（首字母大写），必须带 re.I，否则静默跳过
m = re.search(r"^\[globals\]\s*$", text, re.M | re.I)
if not m:
    print("[xrdp] xrdp.ini 里找不到 [Globals] 段，跳过调优")
    sys.exit(0)

start = m.end()
nxt = re.search(r"^\[", text[start:], re.M)
end = start + (nxt.start() if nxt else len(text) - start)
block = text[start:end]

changed = []
for k, v in wanted.items():
    pat = re.compile(rf"^[#;]?\s*{re.escape(k)}\s*=.*$", re.M)
    line = f"{k}={v}"
    hit = pat.search(block)
    if hit:
        if hit.group(0).strip() != line:
            block = pat.sub(line, block, count=1)
            changed.append(f"{k}: {hit.group(0).strip()} -> {line}")
    else:
        block = block.rstrip("\n") + "\n" + line + "\n"
        changed.append(f"{k}: 新增 {line}")

new_text = text[:start] + block + text[end:]
assert "[Globals]" in new_text, "改写结果丢失 [Globals] 段"

# 原子替换，避免中途失败留下 0 字节文件
fd, tmp = tempfile.mkstemp(dir=os.path.dirname(path), prefix=".xrdp.ini.")
with os.fdopen(fd, "w", encoding="utf-8") as f:
    f.write(new_text)
os.chmod(tmp, 0o644)
os.replace(tmp, path)
print(f"[xrdp] xrdp.ini 已调优 {len(changed)} 处: " + ", ".join(changed))
PYEOF

# ---------------------------------------------------------------- polkit 放行 colord
# 否则 Xfce 每次登录弹 "认证以创建色彩管理设备"
mkdir -p /etc/polkit-1/localauthority/50-local.d
cat > /etc/polkit-1/localauthority/50-local.d/45-allow-colord.pkla <<'PEOF'
[Allow Colord all Users]
Identity=unix-user:*
Action=org.freedesktop.color-manager.create-device;org.freedesktop.color-manager.create-profile;org.freedesktop.color-manager.delete-device;org.freedesktop.color-manager.delete-profile;org.freedesktop.color-manager.modify-device;org.freedesktop.color-manager.modify-profile
ResultAny=no
ResultInactive=no
ResultActive=yes
PEOF
ok "已写入 colord polkit 放行规则"

# ---------------------------------------------------------------- 证书权限
getent group ssl-cert >/dev/null 2>&1 && usermod -aG ssl-cert xrdp 2>/dev/null || true
if [[ ! -s /etc/xrdp/key.pem ]]; then
  warn "/etc/xrdp/key.pem 缺失，生成自签证书"
  if command -v xrdp-keygen >/dev/null 2>&1; then
    xrdp-keygen xrdp auto >/dev/null 2>&1 || true
  fi
fi
[[ -s /etc/xrdp/key.pem ]] && ok "xrdp TLS 证书就绪" || warn "证书仍缺失，xrdp 可能以低安全模式启动（局域网可用）"

# ---------------------------------------------------------------- 防火墙
if command -v ufw >/dev/null 2>&1 && ufw status 2>/dev/null | grep -q "Status: active"; then
  ufw allow 3389/tcp >/dev/null 2>&1 || true
  ok "ufw 已放行 3389/tcp"
else
  log "ufw 未启用，跳过"
fi
# iptables 兜底（仅当前运行期生效，避免动持久化规则）
if command -v iptables >/dev/null 2>&1; then
  if ! iptables -C INPUT -p tcp --dport 3389 -j ACCEPT 2>/dev/null; then
    iptables -I INPUT -p tcp --dport 3389 -j ACCEPT 2>/dev/null || true
  fi
fi

# ---------------------------------------------------------------- 启动
log "启动 xrdp"
systemctl enable xrdp >/dev/null 2>&1 || true
systemctl restart xrdp
sleep 2

# ---------------------------------------------------------------- 自检
echo
echo "=============================== 自检 ==============================="
printf 'xrdp 服务 : '; systemctl is-active xrdp || true
printf '会话管理 : '; systemctl is-active xrdp-sesman || true
printf 'xrdp 版本 : '; dpkg-query -W -f='${Version}\n' xrdp 2>/dev/null || echo "未知"
printf '开机自启 : '; systemctl is-enabled xrdp 2>/dev/null || echo "未启用"

# xorgxrdp 后端模块：multiarch 目录下找，找不到就一定会连上黑屏
if ls /usr/lib/*/xrdp/libxup.so >/dev/null 2>&1; then
  ok "Xorg 后端 libxup.so 就位（$(ls /usr/lib/*/xrdp/libxup.so | head -1)）"
else
  warn "libxup.so 缺失，[Xorg] 会话会加载失败，请重装 xorgxrdp"
fi

# conffile 空文件检查（本次实测踩到的坑）
for f in xrdp.ini sesman.ini startwm.sh; do
  [[ -s "/etc/xrdp/$f" ]] && ok "/etc/xrdp/$f 非空（$(stat -c%s "/etc/xrdp/$f") 字节）" \
                          || warn "/etc/xrdp/$f 为空，xrdp 必然起不来"
done

if ss -ltn 2>/dev/null | grep -q ':3389'; then
  ok "3389 已监听"
else
  warn "3389 未监听，检查 journalctl -u xrdp -n 50"
fi
printf '监听明细 :\n'; ss -ltnp 2>/dev/null | grep -E ':3389' || true
echo "--------------------------------------------------------------------"
printf '会话用户 : %s\n' "${TARGET_USER}"
printf '连接方式 : macOS → Microsoft Remote Desktop → 添加 PC → %s\n' "$(hostname -I 2>/dev/null | awk '{print $1}')"
printf '          或终端:  open "rdp://full%%20address=s:%s"\n' "$(hostname -I 2>/dev/null | awk '{print $1}')"
echo "===================================================================="
echo
ok "安装完成。首次连接若黑屏，看：tail -50 ${TARGET_HOME}/.xsession-errors 与 journalctl -u xrdp -n 80"

log "为了让桌面立即创建，建议现在用 RDP 客户端连一次（不要同时留图形控制台登录）"
