# LanLink 双机联调指南（无需发包）

> 目标：在没有安装包的情况下，用两台机器跑通端到端（握手 + 传输）。
> 结论：**用 `tauri build --no-bundle` 编出内嵌前端的二进制，拷到第二台机器即可**，第二台不需要装 Rust / Node / npm 任何开发环境。
> 验证日期：2026-11-17（本机 x86_64 Intel Mac，macOS）。

---

## 0. ⚠️ 最容易踩的坑：必须用 `tauri build`，不能用 `cargo build`

| 命令 | 结果 |
| --- | --- |
| `cargo build -p lanlink-desktop` | ❌ **界面白屏**。debug profile 下编出的二进制仍指向 `devUrl`（`127.0.0.1:5173`），脱离 Vite 就没有页面 |
| `tauri build --debug --no-bundle` | ✅ **正常渲染**。`tauri build` 才会把 `frontendDist`（`web/dist`）真正嵌入二进制 |

判断方法：`strings target/debug/lanlink-desktop | grep -E "assets/index-.*\.(js|css)"`，能搜到 hashed 资源名才算嵌入成功。脚本 `scripts/build-portable.sh` 已封装正确命令并自动做这个自检。

---

## 1. 为什么不用发包也能联调（已验证事实）

| 验证项 | 结果 | 证据 |
| --- | --- | --- |
| 前端是否内嵌 | ✅ 是（须用 `tauri build`） | 5173 端口全程空闲仍正常渲染界面，`strings` 可查到 `assets/index-*.js/css` |
| 二进制是否自包含 | ✅ 是 | `otool -L` 只依赖 macOS 系统框架（WebKit / AppKit / Foundation / libSystem），无第三方 `.dylib` |
| 网络是否允许跨机 | ✅ 是 | TCP 42210 与 UDP 42211 都绑 `0.0.0.0`（非 127.0.0.1），见 `session.rs:90`、`discovery.rs:218` |
| 短码是否跨机 | ✅ 是 | 广播用「每个网卡的子网广播地址 + 255.255.255.255」，LA 应答回填来源地址拨号，见 `discovery.rs:132` / `lib.rs:602` |
| 资源占用 | 很低 | 主进程 ~25 MB，含 WebKit 渲染进程整组 ~58 MB；空闲 CPU 接近 0（见 §6） |
| 二进制体积 | 35 MB | `target/debug/lanlink-desktop` |

**唯一硬性前提**：两台机器 CPU 架构要一致（见 §4.1）。

---

## 2. 快速开始（推荐：局域网短码，同网段即可）

假设 A = 生成方（发文件 / 生成凭证），B = 连接方（收文件）。

### 2.1 准备二进制

本机（A）已有：
```bash
cd /Users/yangbo/WorkBuddy/lanLink
./scripts/build-portable.sh          # 生成 or 刷新 target/debug/lanlink-desktop
```

传到 B 机器（二选一）：
```bash
# U 盘 / AirDrop / scp 均可
scp target/debug/lanlink-desktop user@B机器IP:~/Desktop/
# B 机器上：
chmod +x ~/Desktop/lanlink-desktop
```

> macOS 若提示「无法打开，因为来自身份不明的开发者」：右键 → 打开，或 `xattr -cr ~/Desktop/lanlink-desktop` 去掉隔离属性。

### 2.2 两台机器都启动

```bash
# A 和 B 各自执行（终端保持开着，日志会打印 [lanlink-bridge] 等）
./lanlink-desktop
```

确认两端窗口都出界面、日志无 `bind ... failed`。

### 2.3 走通短码流程

| 步骤 | A 机器（生成方 / 发送） | B 机器（连接方 / 接收） |
| --- | --- | --- |
| 1 | 命令栏打开「可发现」开关 | 命令栏打开「可发现」开关 |
| 2 | 发送页 → 选择要发的文件 | — |
| 3 | 点击「生成局域网短码」→ 得到 6 位码（如 `K7G2S9`）→ 口头/IM 告知 B | — |
| 4 | 等待对方连接 | 「连接远程设备」→ Tab「局域网短码」→ 输入 6 位码 → 连接 |
| 5 | 握手完成，开始分片传输 | 接收队列出现文件，落盘到保存目录 |
| 6 | 进度条 / 速率曲线更新 | 完成后「传输记录」可查 |

排错看日志关键字：`[lanlink-discovery]`（发现/短码查询）、`[lanlink-session]`（握手）、`[lanlink-bridge]`（前端调用）、`ll-transfer-progress`（传输进度）。

---

## 3. 外网序列号联调（跨网，非同网段）

仅在两台机器不在同一局域网、且都处于**全锥型 / 端口受限型 NAT**（对称 NAT 不支持，见 TODO-01）时走这条：

1. A：发送页选文件 → 生成外网序列号（14 段）→ 复制，IM 发给 B
2. B：连接远程设备 → Tab「外网序列号」→ 粘贴 → 连接
3. 双方依赖 STUN 观测端点直连（TCP 42210）

> 注意：序列号采集依赖 STUN（外网可访问 `stun.l.google.com:19302` 之类）与出口 IP 探测（连 `8.8.8.8:80` 查路由表，不实际发包）。内网完全离线时 Pub4/Pub6 候选为空，只剩 Lan4——此时等价局域网直连，应改用短码。

---

## 4. 前置条件与坑（联调前必看）

### 4.1 CPU 架构必须一致
本机是 **x86_64（Intel i7-8569U）**。裸二进制不能跨架构运行：
- B 也是 Intel Mac → 直接拷
- B 是 Apple Silicon（arm64）→ 在 B 上本地编译：
  ```bash
  git clone git@github.com:scpuny/lan-link.git && cd lan-link
  cd web && npm install && npm run build && cd ..
  cargo build -p lanlink-desktop
  ./target/debug/lanlink-desktop
  ```
  （B 需要装 Rust + Node 环境；或开启 Rosetta 2 后直接跑 x86_64 二进制）

### 4.2 防火墙 / 系统授权
- **必须允许传入连接**：首次弹「是否允许 LanLink 接受传入网络连接」要点「允许」；否则 TCP 42210 拨号会被系统丢弃，表现是「一直连不上 / 超时」
- 若手动开了 macOS 应用防火墙，需在「系统设置 → 网络 → 防火墙」放行，或临时关闭做联调
- 端口：TCP **42210**（握手+传输）、UDP **42211**（发现+短码），两端都要通

### 4.3 端口占用
同一台机器**只能跑一个实例**（42210/42211 独占）。启动时若见：
```
[lanlink-discovery] bind 42211 failed: Address already in use
[lanlink-session] host bind 42210 failed
```
说明已有实例在跑，先 `pkill -f lanlink-desktop` 再启动。

### 4.4 局域网短码的边界
- 仅**同一网段 / 同一 WiFi**有效；跨 VLAN / 跨路由广播不转发 → E010（见 TODO-08）
- 两端「可发现」开关都要开
- 查询窗口 2s，代码在 `connect_lan_code` 里；超时会明确提示 E010

### 4.5 仅已实现 STUN 直连
跨网 NAT 打洞未做（TODO-01）。对称 NAT、运营商大内网场景跨网不通，属已知限制，不是联调配置错误。

---

## 5. 连通性自检（连不上时先跑这个）

在 B 机器上测 A 的端口是否可达（把 `A_IP` 换成 A 的局域网 IP，可在 A 的「本机信息卡 / Lan4」看到）：

```bash
# TCP 握手端口
nc -vz -G 3 A_IP 42210 && echo "TCP 42210 通" || echo "TCP 42210 不通（查防火墙/同网段）"

# UDP 发现端口（在 A、B 都启动的前提下，B 侧应能在 UI 设备表看到 A）
```

若 TCP 通则网络层没问题，再去查序列号/短码/口令逻辑；TCP 不通则先解决防火墙和同网段。

---

## 6. 构建脚本用法与资源占用

`scripts/build-portable.sh`：
```bash
./scripts/build-portable.sh          # 刷新前端 + debug 二进制（联调够用，快）
./scripts/build-portable.sh --release # release 二进制（更小更快，编译久）
```
脚本会打印产物路径、体积、架构、动态库依赖，并自动校验「前端是否真的嵌入」（打印 `前端内嵌 : ✓`）。

### 资源占用实测（2026-11-17，x86_64 Intel Mac / macOS 15.7）

| 进程 | 物理内存 |
| --- | --- |
| lanlink-desktop 主进程 | ~25 MB |
| WebKit 渲染/GPU/网络子进程（3 个） | ~33 MB |
| **整组合计** | **~58 MB** |
| 空闲 CPU | 接近 0（事件循环等待，`sample` 采样无忙等） |

结论：**任何能跑 Chrome 的机器都绰绰有余**，老笔记本、低配办公机均可。内存/CPU 不是联调门槛，真正的门槛只有「同网段 + CPU 架构一致 + 允许传入连接」。

---

## 7. 与 TODO 的对应

本指南服务于 `docs/feature-inventory.md` 的 **TODO-07（双实例端到端联调）**。跑通后请在 TODO-07 的「完成日期 / 验证结果」栏填入实测结果，并把状态改为 ✅。

---

## 8. 备用测试机 192.168.1.11（Ubuntu 18.04）远程桌面

这台机器是 Ubuntu 18.04.6 LTS x86_64，原本只有 SSH，没有远程桌面。已装 **xrdp + Xfce**，可从 Mac 直接图形化操作它跑 LanLink。

### 8.1 连接方式

1. Mac 打开 **Microsoft Remote Desktop**（App Store 免费）→ `Add PC`
2. PC name 填 `192.168.1.11`，User account 填 `scpun` / 密码
3. 显示尺寸建议 `1280×800` 或 `1440×900`

终端一键打开也行：
```bash
open "rdp://full%20address=s:192.168.1.11"
```

### 8.2 机器信息

| 项 | 值 |
| --- | --- |
| 系统 | Ubuntu 18.04.6 LTS x86_64（架构与本机一致，二进制可直接拷） |
| 内存 / 磁盘 | 9.9 GB 可用 8 GB；根分区剩 13 GB |
| 桌面 | GNOME 已在物理控制台 `:0` 登录（未改动）；xrdp 会话独立跑 Xfce |
| 服务 | `xrdp` + `xrdp-sesman` 均 active，已 `enabled` 开机自启 |
| 端口 | 3389（RDP）；原有 22（SSH）、80（Python mDNSStudio）未受影响 |
| 免密登录 | 已把本机 `id_ed25519` 写入 `scpun@192.168.1.11:~/.ssh/authorized_keys` |

Sanity check：
```bash
ssh scpun@192.168.1.11 'systemctl is-active xrdp xrdp-sesman; ss -ltn | grep 3389'
```

### 8.3 重装 / 迁移用脚本

`scripts/install-xrdp-xfce.sh`，幂等，可重复执行：
```bash
scp scripts/install-xrdp-xfce.sh scpun@192.168.1.11:/tmp/
ssh scpun@192.168.1.11 'sudo bash /tmp/install-xrdp-xfce.sh scpun'
```

脚本做完的事：源可用性判定（不看 `apt-get update` 退出码，看 `xrdp` 有无候选版本）→ 装包 → 修 conffile → 写 `startwm.sh`/`.xsession` → 预置 Xfce 面板配置 → 禁 `nm-applet` → colord polkit 放行 → 放行 3389 → 自检。

### 8.4 本次实际踩到的 5 个坑（都已在脚本里修掉）

| 现象 | 根因 | 处置 |
| --- | --- | --- |
| 脚本误判「源挂了」，把 sources.list 改成 old-releases | `download.docker.com` 走 IPv6 握手失败，导致 `apt-get update` 整体非零退出 | 判定改为 `apt-cache policy xrdp` 是否有候选版本；docker 源已注释（备份在 `/var/backups/xrdp-install/docker.list.bak`） |
| `xrdp` 一启动就 `exit 1`，journal 里**一行日志都没有** | `/etc/xrdp/xrdp.ini` 与 `sesman.ini` 是 **0 字节**（历史 remove 残留 → dpkg 保留空 conffile） | `dpkg-deb -x` 从 deb 提取默认配置覆盖，md5 与 dpkg 记录一致 |
| `dpkg` 卡在半配置状态 | postinst 里 `systemctl start xrdp` 失败 | `dpkg --configure -a` 修好，状态回到 `ii` |
| 调优静默失效 | 正则写的是 `[globals]`，实际是 **`[Globals]`**（首字母大写） | 加 `re.I`；并改成先写临时文件再原子替换，避免 0 字节 |
| 连上后弹「欢迎第一次启动面板」，不点掉没有任务栏 | 用户首次运行 `xfce4-panel` 无配置 | 预置 `~/.config/xfce4/xfconf/xfce-perchannel-xml/xfce4-panel.xml` |

另：`nm-applet` 自启会弹「解锁密钥环」，服务器上用不到，已用 `~/.config/autostart/nm-applet.desktop`（`Hidden=true`）屏蔽。

### 8.5 验证方式（已实测通过）

不依赖 Mac 端 GUI，直接在目标机回环自测：`Xvfb` 起一个虚拟屏跑 `xfreerdp` 连 `127.0.0.1:3389`，然后 `import -window root` 截图。实测 sesman 正常拉起：

```
scpun  Xorg :10 -auth .Xauthority -config xrdp/xorg.conf -noreset -nolisten tcp
scpun  xrdp-chansrv
scpun  xfce4-session / xfwm4 / xfdesktop
```

截图确认：Xfce 壁纸、桌面图标（回收站/文件系统/主文件夹）、顶部面板（whiskermenu + 时钟）、中文字体均正常，无黑屏、无弹窗。测试用的 `freerdp2-x11` / `xvfb` 已 `purge` 清理。

### 8.6 排错入口

```bash
# xrdp 主服务日志
ssh scpun@192.168.1.11 'sudo journalctl -u xrdp -n 80 --no-pager'
# 会话内部（Xorg / xfce 的具体报错）
ssh scpun@192.168.1.11 'tail -60 ~/.xsession-errors; ls ~/.xorgxrdp.*.log && tail -40 ~/.xorgxrdp.*.log'
```

黑屏时的典型原因排序：`startwm.sh` 没 exec 到桌面 → `XDG_RUNTIME_DIR` 缺失 → dbus 没起 → 同一用户已有图形控制台会话冲突（必要时先在 `:0` 注销）。
