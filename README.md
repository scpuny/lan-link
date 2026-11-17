# LanLink

局域网 / 跨网 P2P 文件传输桌面应用。无服务器、无中转：把自包含的「外网序列号」或「局域网短码」发给对方即可直连传文件。

- 技术栈：Tauri v2 + Rust（workspace）+ Vue 3 / Vite / TypeScript
- 平台：macOS / Windows / Linux（自定义窗口，macOS 保留原生交通灯与菜单栏）
- 协议版本：v1（`com.lanlink.app`）
- 许可证：MIT OR Apache-2.0

## 功能特性

**文件传输**
- 选择文件 / 文件夹、拖放落区、剪贴板粘贴；保存目录可配置并持久化
- 4 MB 分片、seq 单调递增校验、重复分片丢弃、乱序回退重传
- 接收端真实落盘、同名自动重命名、完成后 Blake3 整文件校验
- 暂停 / 继续 / 取消 / 全部暂停；`ll-transfer-progress` 事件驱动实时进度与速率曲线

**连接相关**
- **外网序列号**：自包含候选地址（IPv6 / STUN 观测端点 / 内网地址）+ CRC32 + 可选 AES-GCM 口令加密 + Base32 编码，TTL 900s / 300s 可调
- **局域网短码**：6 位短码，UDP 42211 广播换取完整序列号，仅同网段有效
- 三层防重放：TTL 时效 + session 内存黑名单 + Blake3 动态质询应答
- 五级校验：解码 → CRC/GCM 完整性 → Payload 解析 → TTL → 黑名单 → 质询
- TCP 42210 拨号，候选按 Pub6 → Pub4 → Lan4 → Lan6 优先级回落

**界面**
- 三主视图：发送文件 / 接收文件 / 传输记录；左轨本机信息卡三视图统一、取自真实采集
- 明暗主题（跟随系统 / 浅色 / 深色）× 6 套视觉风格色板
- 传输记录：本地持久化 30 天、方向/状态/时间筛选、搜索、CSV 导出、脱敏诊断面板
- 自定义无边框窗口（非 macOS），最小尺寸 1040×760；macOS 原生菜单

完整功能清单与当前完成状态见 [`docs/feature-inventory.md`](docs/feature-inventory.md)（**§0.5 任务状态总览** + §0.5.2 TODO 追踪表）。

## 项目结构

```
lanLink/
├── Cargo.toml                       # Rust workspace 根
├── crates/
│   ├── lanlink-protocol/            # 纯 Rust 协议层（Base32/CRC32/LZ4/AES-GCM/Blake3/双 FSM，可独立测试）
│   └── lanlink-desktop/             # Tauri v2 桌面壳：命令、网络采集、设备发现、握手、传输引擎、原生菜单
├── web/
│   ├── src/
│   │   ├── bridge/                  # Tauri invoke 桥接 + 浏览器 mock 回退
│   │   ├── components/              # 布局（三视图/弹窗）与通用组件
│   │   ├── composables/             # useUi / useTheme / useRecords / usePlatform
│   │   ├── data/                    # 原型文案、色板、错误码
│   │   ├── styles/                  # design token / 色板覆盖 / UI 组件样式
│   │   └── types/                   # 前后端共享类型（camelCase 对齐 Rust）
│   └── vite.config.ts
└── docs/
    ├── lanlink-invite-serial-PRD.md # 外网序列号协议 PRD（D2/D3/D4 里程碑）
    ├── lanlink-lan-code-PRD.md      # D4 局域网短码规格
    ├── prototype-review.md          # 原型逐项评审与 UI 规格（§13–§24 为权威）
    └── feature-inventory.md         # 功能全景清单 + 状态总览 + TODO 追踪
```

前后端只走 Tauri `invoke`（进程内 IPC），**不使用 HTTP**；浏览器预览态回退到 `web/src/bridge/mock.ts`。

## 环境要求

- Rust（edition 2021，stable）
- Node.js + npm
- Tauri v2 系统依赖（[官方文档](https://v2.tauri.app/start/prerequisites/)）
- macOS 需 Xcode Command Line Tools

### Linux 运行要求（终端用户）

Linux 产物由 CI 在 `ubuntu-22.04` 上构建。**glibc ≥ 2.34** 与**系统提供 webkit2gtk 4.1** 两个条件必须同时满足：

| 发行版 | glibc | webkit 4.1 | 可运行 |
| --- | --- | --- | --- |
| Ubuntu 22.04 / 24.04 | 2.35 / 2.39 | ✅ | ✅ |
| Debian 12 / 13 | 2.36 | ✅ | ✅ |
| RHEL 9 / Rocky 9 / Alma 9 | 2.34 | ✅ | ✅（正好卡线） |
| Fedora 36+ | 2.35+ | ✅ | ✅ |
| deepin 23+ / UOS V25 / 麒麟 V11 | — | ✅ | ✅ |
| Ubuntu 20.04、Debian 11 | 2.31 | ❌ 仅 4.0 | ❌ |
| Ubuntu 18.04 | 2.27 | ❌ 仅 4.0 | ❌ |
| UOS V20 全系、麒麟桌面 V10 SP1 | 2.28 / 2.31 | ❌ | ❌ |

两点保障：

- `.deb` 已在 `Depends` 声明 `libwebkit2gtk-4.1-0`，不达标的系统会被包管理器**直接拒绝安装**，不会出现"装上了打不开"。
- CI 有 **glibc 地板守卫**（`build-linux.yml`），一旦构建基线上移导致符号版本超过 2.34 会直接失败，防止无意收窄可运行范围。

> 老机器（18.04 / UOS V20 / 麒麟 V10）跑不了 Tauri v2，这是 webkit2gtk 4.1 与 glibc 2.34 的双重硬约束，不能靠自带库绕过。若确有需求，方向是把 Rust 核心拆成无 GUI 本地服务、UI 交给系统浏览器，而非继续在 Tauri 上打补丁。

### 在虚拟机 / 无 GPU 环境运行

VirtualBox、VMware、QEMU 这类环境没有可用的硬件 OpenGL，WebKitGTK 默认走的 DMABUF + 合成器会一直等 GPU 信号。典型症状是**窗口能画出来，但整机界面逐渐冻结，CPU 占用接近 0，最后连 SSH 都不响应**（不是算力不够，是图形栈阻塞）。

程序启动时会读 `/sys/class/dmi/id/product_name` 与 `sys_vendor` 自动识别虚拟机，识别到就设置 `WEBKIT_DISABLE_COMPOSITING_MODE=1`、`WEBKIT_DISABLE_DMABUF_RENDERER=1`、`LIBGL_ALWAYS_SOFTWARE=1`，无需手动干预。

宿主机侧还有三项需要人工确认，缺一项都可能让虚拟机变卡：

| 项 | 正确值 | 说明 |
| --- | --- | --- |
| 3D 加速 | **关闭** | macOS 主机的 VirtualBox 不提供可用 3D 后端，开启后 vmwgfx 会卡在等 GPU 完成信号 |
| 客户机分辨率 | **≤ 1920×1080** | 关闭 3D 后 VBox 可能把模式重置到 3840×2160，llvmpipe 要填 830 万像素/帧，等于 8 倍负载 |
| 窗口合成器 | **关闭** | Xfce 的 `xfwm4` 合成（`/general/use_compositing`）会让每次重绘多一次全屏合成 |

对应的命令行操作：

```bash
VBoxManage modifyvm <vm> --accelerate3d off
VBoxManage setextradata <vm> GUI/MaxGuestResolution "1440,900"
# 客户机内：
xfconf-query -c xfwm4 -p /general/use_compositing -s false
```

裸机但无 GPU 的场景（xrdp 远程桌面、Xvfb、容器）用脚本兜底：

```bash
./scripts/run-linux.sh                 # 自动查找已安装的 lanlink-desktop
./scripts/run-linux.sh /path/to/bin    # 或指定路径
FORCE_SOFTWARE_GL=0 ./scripts/run-linux.sh   # 有真实 GPU 时关掉软件渲染
```

## 开发

```bash
# 前端依赖（首次）
cd web && npm install && cd ..

# 启动 GUI 开发模式（自动拉起 Vite dev server 127.0.0.1:5173 并编译 Rust）
cd crates/lanlink-desktop
../../web/node_modules/.bin/tauri dev
```

> 本机若未安装 `cargo-tauri`，用仓库内的本地 CLI `web/node_modules/.bin/tauri dev`；命令在 `crates/lanlink-desktop` 目录下执行，`beforeDevCommand` 会以 `--prefix ../web` 拉起前端。

仅预览前端（无真实后端，走 mock）：

```bash
cd web && npm run dev
```

## 测试与构建

```bash
# 协议层单元测试（26 项）
cargo test -p lanlink-protocol

# Rust 检查
cargo check -p lanlink-desktop

# 前端生产构建（产物 web/dist，Tauri 打包时读取）
cd web && npm run build
```

## 双机联调（无需发包）

前端已内嵌进二进制，编译产物只依赖 macOS 系统框架，**可直接拷到另一台同架构机器运行**，第二台不需要 Rust / Node 环境：

```bash
./scripts/build-portable.sh      # 刷新前端 + debug 二进制（联调够用）
# 产物 target/debug/lanlink-desktop，拷到另一台机器后 chmod +x 并运行
```

> ⚠️ 必须走 `tauri build`（脚本已封装）。直接 `cargo build` 编出的 debug 二进制仍指向 devUrl `127.0.0.1:5173`，脱离 Vite 会**白屏**。
>
> 资源占用实测：主进程 ~25 MB，含 WebKit 子进程整组 ~58 MB，空闲 CPU 接近 0——低配机即可联调。

完整步骤（短码 / 序列号两条路径、防火墙、连通性自检、架构限制）见 [`docs/joint-debug-guide.md`](docs/joint-debug-guide.md)。

Linux 备用测试机（192.168.1.11，Ubuntu 18.04）已装 xrdp + Xfce 远程桌面，Mac 可用 Microsoft Remote Desktop 直连操作；重装/迁移用：

```bash
scp scripts/install-xrdp-xfce.sh scpun@192.168.1.11:/tmp/
ssh scpun@192.168.1.11 'sudo bash /tmp/install-xrdp-xfce.sh scpun'
```

详见 [`docs/joint-debug-guide.md` §8](docs/joint-debug-guide.md)。

## 调试开关

- `?bridge=mock`：真机强制走内存 mock，用于真/Rust 输出 A/B 对比
- `?view=send|receive|history`、`?palette=`、`?theme=`：预览直达指定视图 / 色板 / 主题
- debug 日志 `[lanlink-bridge] <cmd>` 表示命令走了真实 IPC 而非 mock；`[lanlink-window]` / `[lanlink-menu]` / `[lanlink-size-probe]` 分别用于窗口、菜单、最小尺寸验收

## Git 分支约定

| 分支 | 用途 |
| --- | --- |
| `main` | 稳定版，验收后由 `dev` 合并 |
| `dev` | 日常开发集成分支（默认 working 分支） |
| `feature-xxx` | 功能分支，从 `dev` 切出，完成后合回 `dev` |

每个阶段同步三处：Rust command → `web/src/bridge/mock.ts` → `web/src/bridge/index.ts` 路由；新增命令还需补 capability 权限。

## 已知限制 / TODO

详见 [`docs/feature-inventory.md` §0.5.2](docs/feature-inventory.md)。摘要：

- ⬜ 跨网 NAT 打洞（对称 NAT 需中继 / UPnP，当前仅 STUN 直连）
- ⚠️ 托盘 / 开机自启 / 防火墙放行 / 检查更新需打包签名与系统权限
- ⚠️ 传输记录存 localStorage（清缓存会丢），待改为 Rust 侧文件
- ⚠️ 握手 / 传输需两台实例端到端联调；局域网短码跨 VLAN 失效（E010）
