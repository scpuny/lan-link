# LanLink 原型设计评审与开发参考文档（D2 交付物）

> 来源：calicat 设计文件 `2121877126031921152`（画布 1）
> 拉取时间：2026-11-15
> 配套 PRD：`docs/lanlink-invite-serial-PRD.md`（D1）
> 协议层实现：`crates/lanlink-protocol`（D3，已通过 26 项测试）
> 全部截图见 `docs/prototypes/`（29 张 PNG，命名见各节）

---

## 0. 文档目的与范围

本文档将 calicat 上的 **30 个设计页面** 逐一查看、归类、提取成可落地的开发规格，作为后续 Vue 前端 + Tauri 桌面端实现的依据。重点回答两个问题：

1. **主视图选哪个版本** —— 见 §1。
2. **每个对话框/页面长什么样、有哪些字段、状态怎么切换** —— 见 §2～§9。

> ⚠️ 拉取时发现 calicat 截图服务对前两页（连接-就绪、连接-校验）初次生成返回了陈旧渲染（误为「关于」页），已重新生成并覆盖 `connect-ready.png` / `connect-validating.png`，本文以覆盖后的为准。

---

## 1. 主视图（Main Window）选型建议

### 1.1 四个「主视图」版本实为同一布局的状态/填充变体

| 版本（layer_id） | 名称 | 本质 | 截图 |
|---|---|---|---|
| `ebf48ff5` | 主窗口·就绪态 | 空闲态，无 LAN 设备时引导生成凭证 | `prototypes/main-ready.png` |
| `13cf858d` | 主窗口·传输中 | 已建通道、有传输任务的运行态 | `prototypes/main-transferring.png` |
| `49962cad` | 主窗口·局域网设备已发现 | 空闲态 + 已发现 LAN 设备 | `prototypes/main-lan-mdns.png` |
| `a1562ae5` | 主窗口·就绪态（锁定版） | 空闲态 + 已选文件 + 已发现设备（填充态） | `prototypes/main-ready-locked.png` |

四者**左侧导航轨道**与**右侧会话面板**结构完全一致，仅**中央工作区内容**不同：

- **就绪态**：中央 = 拖放区 + 待发送文件表 + **两张连接方法卡片**（外网序列号 / 局域网码）。
- **局域网设备已发现 / 锁定版**：中央 = 拖放区 + 待发送文件表 + **附近设备表（mDNS）** + 压缩成一条的序列号入口；不再有独立「连接方法卡片」。
- **传输中**：中央 = 通道摘要卡 + 传输任务卡（进度条/速率）+ 速率曲线卡 + 排队任务卡；右侧会话面板追加「分片明细卡」「安全说明卡」。

### 1.2 推荐方案：单一主窗口组件 + 状态机驱动中央区

> **结论：不要按 4 个版本做 4 套页面，而是做 1 个三栏主窗口（`App.vue` / `MainWindow`），中央「发送区」按状态切换内容。**

- **布局基底**：三栏（左 220px 导航轨道 / 中自适应工作区 / 右 296px 会话面板）+ 顶部标题栏（菜单 + 窗口控制）+ 底部状态栏。四版本共用，仅中央区与右侧部分卡片随状态显隐。
- **中央区状态**：
  - `idle_no_lan`（无设备）：展示「连接方法双卡片」（外网序列号 / 局域网码）——对应就绪态，引导最清晰。
  - `idle_with_lan`（已发现设备）：展示「附近设备表 + 压缩序列号入口」——对应局域网设备已发现 / 锁定版。
  - `transferring`（通道已建立）：展示传输任务卡 + 速率曲线 + 排队——对应传输中。
- **右侧面板**：`idle` 时显示「当前会话（未连接）+ 最近连接 + 凭卡安全」；`transferring` 时追加「分片明细卡 + 安全说明卡」。
- **「锁定版」不单独实现**：它只是「就绪态 + 已选文件 + 已发现设备」的填充态，由 `idle_with_lan` 状态自然覆盖。

### 1.3 三栏通用布局规格（所有状态共用）

**顶部标题栏**
- 左：应用图标 + 名称「局域网 / 跨网 P2P 文件传输」；菜单栏「文件 / 设备 / 视图 / 帮助」（见 §8）。
- 中（传输中）：通道已建立标记（绿点 + 文案）。
- 右：最小化 / 最大化 / 关闭按钮。

**左侧导航轨道**（220px）
- 本机信息卡片：设备名、`OS·用户`、本机地址行（Lan4 / Pub6 / STUN）。
- 导航列表（带角标）：发送文件、**接收文件**、传输记录（角标）、设置。
- 网络能力检测卡（就绪态为「网络能力检测」）：mDNS 广播 / STUN 端点 / IPv6 全局单播 / IPv4 直连性（绿=可用、橙=警示）。传输中态改为「通道能力卡」：P2P 直连 / 通道加密 / 分片 seq 校验 / 会话黑名单。

**底部状态栏**
- 左：状态点 + 主状态文案（就绪·无活动会话 / 传输中·N 个任务·速率）。
- 中：TCP 端口、会话黑名单条数、口令加密开关。
- 右：版本号 `v0.9.4 (20261114) · 协议 v1`。

---

## 2. 连接流程对话框（外网序列号）规格

文件：`web/src/components/ConnectDialog.vue`（D3 已建骨架，需按本规格细化）
截图：`prototypes/connect-ready.png`、`connect-validating.png`、`connect-handshake.png`、`connect-success.png`、`handshake-A-side.png`

### 2.1 双 Tab 隔离（PRD 7.1 强约束）
- Tab1「外网序列号连接」（默认选中）；Tab2「局域网短码连接」（见 §6）。
- **两种凭证不共用输入缓冲**，切换 Tab 不保留上一页内容（原型底部明确提示）。

### 2.2 就绪态（`connect-ready.png`）
- 标题「连接到远程设备」，右上 ×。
- **等宽序列号输入框**：占位提示「粘贴后自动清洗大小写 / 空格 / 换行」；右侧标注「6 字符 / 组」。
- **口令入口**：复选框「口令保护（该序列号已加密）」+ 说明「勾选后连接时会要求输入对方单独告知的 4-6 位数字口令」，标签 `AES-GCM-256`。
- **底部六态校验胶囊**（两行三列）：解码 / 完整性 / 解析 / 时效 / 会话 / 质询。就绪时均灰，文案「● 就绪，粘贴序列号后点击连接」，右侧「⏱ 质询超时 10s」。
- 按钮：「+ 生成序列号」（次）、「🔗 连接」（蓝主）、「取消」。

### 2.3 校验中（`connect-validating.png`）
- 右上「进行中」；序列号展示「84 字符 · 14 组」；解析出候选地址（Pub6/Pub4）。
- 底部状态「正在校验序列号…（CRC32 校验中）」；「连接中…」按钮禁用（旋转图标）。

### 2.4 握手质询中（`connect-handshake.png`）
- 右上蓝色「质询中」徽标；新增 **动态质询卡片（Blake3 Challenge-Response）**：session 前 8 位、16 字节 challenge（一次性）、目标地址（IPv6 优先）、橙色「剩余 8s」倒计时、说明「成功后写黑名单」。
- 底部「正在握手质询… 已向对端发起连接并下发 challenge，等待 Blake3 摘要应答」「超时 10s」。
- 按钮：「等待应答…」（禁用·旋转）、「取消」（蓝可点）。

### 2.5 成功（`connect-success.png`）
- 绿色成功横幅「握手成功，通道已建立」+「本次握手耗时 1.2s」。
- 六枚绿勾标签：解码/完整性/解析/时效/会话/质询。
- 通道信息：对端设备、连接地址（IPv6 直连标签）、往返延迟、通道加密（关闭=明文+CRC32）、`协议 v1 · flags=0x00`。
- 灰色条「该序列号已入会话黑名单，即刻作废，不可再次使用」「黑名单 1 条」。
- 按钮：「打开传输面板」（蓝主）、「关闭」。

### 2.6 状态机映射（对接 D3 `handshake.rs`）
connect 对话框的五级校验顺序与 D3 `ReceiverAction` 状态机一致：清洗 → Base32 解码（E001）→ CRC/GCM 完整性（E002/E003）→ 解析（E007）→ TTL 时效（E004）→ 会话黑名单（E005）→ 质询（E006/E008）→ 应答成功。所有错误态均作为「连接对话框内嵌错误卡」呈现（见 §9），非独立弹窗。

---

## 3. 生成流程对话框规格

文件：`web/src/components/GenerateDialog.vue`（D3 已建骨架，需细化）
截图：`prototypes/gen-no-pwd.png`、`gen-pwd.png`、`gen-complete.png`、`gen-full.png`

### 3.1 生成前（未启用口令 / 完整版）
- 标题「生成外网连接序列号」，右上设备标识 + ×。
- 说明：「将生成一条自包含候选地址的序列号，通过 IM 发给对方即可直连，无需任何服务器。有效期 15 分钟，仅可成功握手一次。」
- **候选地址选择区（2/4 条，可勾选）**：
  - Pub6：全球单播 IPv6 · 跨网首选（默认勾选·已就绪）
  - Pub4：STUN 观测端点（默认勾选·已就绪）
  - 包含内网地址（192.168.1.24 · ULA）——默认不勾选，标注「不推荐」
- **口令保护复选框**：「启用后需 AES-GCM-256 解密，无口令者无法还原地址」，默认关闭。
- 蓝色主按钮「生成序列号」；外网序列号结果区「尚未生成」。

### 3.2 生成前（已启用口令，`gen-pwd.png`）
- 标题栏「口令保护已开启」。
- 新增「口令保护」区：规则「设置 4–6 位数字口令，纯数字、不可含字母」；两个输入框（口令 + 再次确认），密码掩码 + 显示/隐藏（眼睛）按钮；一致性提示「两次一致·强度可用」（绿勾）。
- 按钮变「🔒 加密并生成序列号」；底部安全说明：口令请单独发一条消息、输错仅提示「口令错误或序列号被篡改」、派生密钥不使用即清除、加密仅保护地址与时间戳（防重放仍靠 TTL+黑名单+质询）。

### 3.3 生成完成（`gen-complete.png`）
- 绿色钥匙图标 + 标题 + 绿色「✔ 已生成」标签。
- 绿色横幅「序列号已复制到剪贴板，可直接粘贴到微信发送」+ 倒计时「⏱ 14:52 后失效」。
- **只读等宽序列号框**（灰底）：内容按 6 字符分组并以连字符分隔，如 `K7G2S9-5MNP8R-…-HN8P3J`；注「12 组 / 每 6 字符一组，超长横向滚动不折行」「生成于 10:33:12」；标题行标注「未加密 / 未压缩 / 84 字符 · 14 组」。
- 按钮：「📋 复制序列号」（蓝主）、「↻ 重新生成」、「关闭」。
- **凭证参数面板**：session、候选地址（Pub6×1/Pub4×1）、Payload 52 字节 → Base32 84 字符（JSON 约 200 字节）、有效期 900s、握手成功即作废。
- 黄色警示框：仅用最新一次生成的序列号；关闭弹窗即内存销毁，请先复制再关闭。
- 底部：「台账不落盘：本次生成的凭证不会写入任何本地文件」。

---

## 4. 口令输入对话框规格（接收端）

截图：`prototypes/pwd-normal.png`、`pwd-E003.png`
> 注意：口令输入**不是独立窗口**，而是连接对话框在检测到 `flags.bit0=1`（已加密）时展开的同一弹窗内态（见 §2.2 复选框逻辑 + 本节错误态）。

### 4.1 正常态（`pwd-normal.png`）
- 标题「该序列号已启用口令保护」，标签 `AES-GCM-256`。
- 口令输入框：掩码（● ×6），上方标注「口令 / 纯数字·4-6 位」。
- 蓝色「🔒 确认并解密」+「取消」。
- 校验通过时显示「✓ 序列号已通过 Base32 解码与长度校验：… flags:bit0=1」。
- 底部说明：「派生密钥在本次会话结束后立即清零」「👁 默认隐藏，仅可显示一次」「请向对方单场索取口令」；校验规则：PBKDF2-HMAC-SHA256 派生 32 字节（10000 次）；GCM Tag 校验通过才继续，失败即终止；不自动重试。

### 4.2 错误态 E003（`pwd-E003.png`）
- 顶部红色「× 解密失败」徽标；标题前红色锁图标。
- 红字「第 1 次尝试失败」+「口令错误或序列号被篡改（不区分具体原因，防侧信道泄露）」。
- 输入框红框 + 右侧红感叹号。
- 红色错误卡（带 E003 标签）：原因=GCM Tag 不通过；建议=与对方核对口令手动重输；说明=不自动重试、不区分口令错/密文改。
- 按钮：「⟳ 重新输入口令」（蓝主）、「关闭」（灰）。
- 底部红条 + 页脚灰字「口令与派生密钥失败后立即清零」。

---

## 5. 局域网短码连接对话框规格

截图：`prototypes/dialog-lan-code.png`
> 即连接对话框的 **Tab2「局域网短码连接」**（`connect-ready` 内），非独立窗口。

- 6 位短码输入框：标签「邀请短码（6 位）」，右侧灰字「留空则直接从下方设备表选择」；大号等宽输入（如 `K7G2S9`）；右侧「● mDNS 广播中 · 端口 42210」。
- **设备发现列表（mDNS）**：表头「附近设备（已发现 N 台）｜地址｜延迟｜操作」；每行带电脑图标 + 平台标识；「选择」链接。
- 蓝色信息横幅：「6 位短码不携带地址，通过 mDNS 广播 + 子网探测发现对端，跨网段/跨 VLAN 请切换外网序列号连接」。
- 底部说明条（ⓘ）：「两种凭证不共用输入缓冲，切换页签不会保留上一页内容」。
- 按钮：「🔄 重新搜索」（描边）、「🔗 连接」（蓝主）、「取消」。
- 状态栏：「● 就绪，输入 6 位短码或直接选择附近设备」「程序运行期有效」。

---

## 6. A 端（发起方）握手对话框规格

截图：`prototypes/handshake-A-side.png`
> 即「生成序列号并等待对方连接」的发起方视角，与 §2 接收方视角互补。

- 标题「等待对方连接」，右上蓝色「握手中」标签 + ×。
- 顶部提示框：「对方已收到序列号并发起连接」+「来自 [2408:8207:…:9c1d]:51820 · 已接收 16 字节质询」。
- **五步进度条**：✅等待连接 → ✅收到请求 → ✅接收质询 → 🔄回传应答（进行中）→ ○校验通过。
- **Blake3 应答计算卡**（耗时 3ms）：输入 `session_id ‖ challenge`；challenge `a7f2c94de1b8035`（16B·一次性）；应答摘要 `3e9b71c4d8f2a605`（32B·↗回传中）；数据链路「TCP 已连接 · 往返 38ms · 无中继」。
- 序列号展示 + 「剩余 14:29」倒计时。
- 按钮：「中断握手」（红描边）、「转到后台」（灰描边）。
- 底部蓝字「正在计算并回传质询应答… 对端需在 10s 内完成摘要比对，超时本轮作废，计时 9.4s」；页脚「应答摘要与 challenge 绑定，旧报文无法重放」。

---

## 7. 设置页规格（4 个分页）

文件：新增 `web/src/components/settings/` 下 4 个组件（通用/传输/安全/关于）
截图：`settings-general.png`、`settings-transfer.png`、`settings-security.png`、`settings-about.png`

### 7.1 通用（`settings-general.png`）
| 分组 | 项 | 默认 |
|---|---|---|
| 启动 | 开机自动启动 | 开 |
| 启动 | 启动时最小化到托盘 | 开 |
| 外观与语言 | 主题 | 跟随系统（浅色/深色） |
| 外观与语言 | 界面语言 | 简体中文 |
| 文件与通知 | 接收文件保存目录 | `D:\LanLink\Downloads`（可更改） |
| 文件与通知 | 传输完成后打开所在文件夹 | 关 |
| 文件与通知 | 收到连接请求时弹桌面通知 | 开 |
| 文件与通知 | 传输完成声音提示 | 关 |

### 7.2 传输（`settings-transfer.png`）
| 项 | 默认 | 备注 |
|---|---|---|
| 分片大小 | 4 MB | 协议固定，仅调度可调 |
| 并发传输任务数 | 2 | |
| 启用断点续传 | 开 | |
| 限速模式 | 自定义限速 | 可选「不限速」 |
| 限速值 | 20 MB/s | |
| 传输完成后 Blake3 校验文件 | 开 | |
| 监听端口 | 42210 | |
| 自动添加防火墙入站规则 | 开 | |
> 说明：「分片与 seq 规则由协议固定，此处仅调整调度参数」；无压缩开关（压缩由协议层 LZ4 自适应决定）。

### 7.3 安全与协议（`settings-security.png`）
| 项 | 默认 | 备注 |
|---|---|---|
| 外网序列号有效期 TTL | 900s（15分钟） | 另选项 300s（5分钟·高安全） |
| 生成时默认包含内网地址 | 关 | 默认仅 Pub6+Pub4 |
| 生成时默认勾选口令保护 | 关 | 算法固定 PBKDF2-HMAC-SHA256(10000)+AES-GCM-256，不可改 |
| 会话黑名单 | 当前 1 条 | 仅内存，退出清空 |
| 日志与隐私（info） | 不可关闭 | 仅记 session 前 8 位+ts+error_code；严禁打印完整序列号与 IP；口令/流密钥用完即 zeroize |
| 协议 | v1 | flags bit2–bit7 未启用（中继/传输加密/权限/版本号） |
> 底部：「配置只作用于本进程，不写入注册表以外的持久存储」。

### 7.4 关于（`settings-about.png`）
- 版本：`LanLink v0.9.4 · build 20261114 · 协议 v1`，「✓ 已是最新」。
- 运行环境卡（含「检查更新」）：OS / 网络能力（IPv6 可用·STUN 已获取·IPv4 疑似 CGNAT）/ 监听端口 TCP 42210。
- 协议依赖（MIT/Apache-2.0）：crc32fast（完整性·明文）、lz4_flex（自适应压缩）、aes-gcm（可选口令加密+Tag）、pbkdf2（口令派生，**⚠️此处写 100000 次**，见 §10 不一致）、blake3（握手摘要+整文件校验）。
- 诊断与日志：「复制诊断信息」「打开日志目录」；日志仅记 session 前 8 位+ts+error_code。
- 隐私声明：「无服务器架构：不收集设备指纹、不上传任何地址或文件」。

---

## 8. 菜单结构总览

截图：`prototypes/menu-structure.png`

| 顶级菜单 | 子项（快捷键） |
|---|---|
| 文件 | 选择文件… (Ctrl+O) / 选择文件夹… / 从剪贴板发送 (Ctrl+V) / 传输记录 (18) / 打开下载目录 / 退出 (Alt+F4) |
| 设备（推荐入口） | 扫描局域网设备 (F5) / 附近设备 (3 台) / 生成外网序列号… (Ctrl+G) / 生成局域网短码… (Ctrl+L) / 连接到远程设备… (Ctrl+K) / 本机网络信息 / 端口与防火墙检查 |
| 视图 | 发送文件 ✓ / 接收文件 / 传输记录 / 显示侧栏 ✓ / 显示速率曲线 ✓ / 紧凑模式 / 外观（跟随系统） |
| 帮助 | 使用指南 (F1) / 快捷键一览 / 错误码说明 E001–E009 / 协议与安全说明 / 检查更新 (v0.9.4) / 关于 LanLink |

> 建议：菜单命令直接映射到 Tauri command 与前端路由/弹窗；`Ctrl+G/L/K` 为生成/连接快捷键，应在桌面端注册全局快捷键。

---

## 9. 错误码对话框规格汇总（E001–E009）

> 全部错误态均为「连接到远程设备」对话框的内嵌错误卡（非独立窗口），由 §2.6 状态机驱动。颜色规则：**红色=凭证问题（需重新生成）**，**橙色=网络问题（凭证仍有效，可重试）**。

| 码 | 名称 | 触发（D3 映射） | 主按钮 | 颜色 | 截图 |
|---|---|---|---|---|---|
| E001 | 序列号损坏/复制不全 | Base32 解码失败（末组不足 6 位） | 重新粘贴 | 红 | `err-E001.png` |
| E002 | CRC32 校验不匹配 | CRC 计算值与凭证不符（被篡改/1bit） | 重新复制 | 红 | `err-E002.png` |
| E003 | 口令错误/序列号被篡改 | GCM Tag 不通过（不区分原因） | 重新输入口令 | 红 | `pwd-E003.png` |
| E004 | 序列号已过期 | TTL 超 900s | 请对方重新生成 | 红 | `err-E004.png` |
| E005 | 已完成握手（黑名单） | 会话 HashSet 命中 | 生成新序列号 | 红 | `err-E005.png` |
| E006 | 握手质询超时 | S_WAIT_RESPONSE 超 10s 未收应答 | 立即重试 | 红 | `err-E006.png` |
| E007 | Payload 字段非法 | candidate_count > 4（长度约束拒绝） | 重新粘贴 | 红 | `err-E007.png` |
| E008 | 应答摘要不匹配 | Blake3(session‖challenge) 比对失败（重放/篡改） | 生成新序列号 | 红 | `err-E008.png` |
| E009 | 凭证合法但地址不通 | 候选地址全探测失败（CGNAT 等） | 重试 / 改用短码 | **橙** | `err-E009.png` |

要点补充：
- E004/E005/E006 **不写入黑名单**（E006 明确「超时不写黑名单，TTL 内可再握手」）；E005 命中即因已成功握手才写入。
- E009 明确区分「橙色网络问题 vs 红色凭证问题」，凭证未作废、可重试，无需重新生成。
- E002 底部注明「CRC 只保证完整性不保密；开启口令后由 GCM Tag 承担」。

---

## 10. 评审中发现的设计不一致（待确认 / 待修正）

1. **PBKDF2 迭代次数矛盾**：
   - 关于页（`settings-about.png`）写 **100000 次**；
   - 安全设置页、生成口令页、PRD（D1）均写 **10000 次**。
   - → 以 PRD/D3 实现（`crypto.rs` 当前为 10000）为准，**关于页多写一个 0，需修正**。
2. **「就绪态（锁定版）」命名歧义**：经核对它并非独立布局，而是「就绪态 + 已选文件 + 已发现设备」的填充态（缺独立连接方法卡片）。建议设计侧明确其只是状态填充，避免被误读为第 5 套布局。
3. **关于页协议依赖文案**：lz4_flex 标注「仅在少量小时采用」、blake3「摘要预摘聚」等应为「仅在变小时采用」「握手摘要」的笔误，属文案级问题。

---

## 11. 与现有代码的落地映射（D3 之后）

| 原型页面 | 现有代码 | 下一步动作 |
|---|---|---|
| 主窗口三栏（§1） | `web/src/App.vue`（极简骨架） | 重写为三栏布局 + 中央区状态机（idle_no_lan / idle_with_lan / transferring） |
| 连接对话框（§2,§5,§9） | `web/src/components/ConnectDialog.vue`（D3 骨架） | 加双 Tab、六态校验胶囊、口令门、错误卡 E001–E009、局域网短码子视图 |
| 生成对话框（§3） | `web/src/components/GenerateDialog.vue`（D3 骨架） | 加候选地址勾选、口令开关、复制+倒计时、凭证参数面板、黄色警示 |
| 口令输入（§4） | 并入 ConnectDialog 状态 | 作为 `flags.bit0=1` 时的内嵌态 |
| A 端握手（§6） | 暂无 | 新增 `WaitConnectDialog.vue`（发起方五步进度 + Blake3 卡） |
| 设置 4 页（§7） | 暂无 | 新增 `web/src/components/settings/` 四组件 + 路由/弹窗容器 |
| 菜单（§8） | 标题栏骨架 | 实现菜单命令 → Tauri command / 前端路由 |
| 协议层（D3） | `crates/lanlink-protocol` | 已覆盖 E001–E009 + 状态机；需补充候选地址采集（STUN+网卡枚举）以驱动实际握手 |

**开发顺序建议**：① 主窗口三栏骨架（§1）→ ② ConnectDialog 状态机 + 错误卡（§2/§5/§9）→ ③ GenerateDialog 细化（§3）→ ④ 设置页（§7）→ ⑤ 菜单与快捷键（§8）→ ⑥ A 端握手弹窗（§6）。

---

## 12. 截图索引（全部 29 张）

主窗口：`main-ready.png` `main-transferring.png` `main-lan-mdns.png` `main-ready-locked.png`
连接：`connect-ready.png` `connect-validating.png` `connect-handshake.png` `connect-success.png`
生成：`gen-no-pwd.png` `gen-pwd.png` `gen-complete.png` `gen-full.png`
口令：`pwd-normal.png` `pwd-E003.png`
局域网/握手：`dialog-lan-code.png` `handshake-A-side.png`
设置：`settings-general.png` `settings-transfer.png` `settings-security.png` `settings-about.png`
菜单：`menu-structure.png`
错误：`err-E001.png` `err-E002.png` `err-E004.png` `err-E005.png` `err-E006.png` `err-E007.png` `err-E008.png` `err-E009.png`

> 注：`err-E003.png` 已归入口令错误态（`pwd-E003.png` 同源），故错误态共 8 张独立 + 1 张口令错误，合计 9 个错误场景。

---

## 13. 前端对齐原型的实现记录（2026-11-15）

> 背景：用户要求「页面与原型一致、色彩必须与原型一致、前后端用桥接而非 HTTP、主题抽象为 design token」。
> 实现位于 `web/`；对比截图见 `docs/screenshots-ui-v2/`（13 张）。

### 13.1 原型色板（对 `docs/prototypes/*.png` 逐像素采样）

用纯标准库 PNG 解码器统计主色频率，并对 9 张图做饱和色筛选，锁定如下色值：

| 用途 | 色值 | 采样占比 / 说明 |
|---|---|---|
| 桌面画布（窗口外层） | `#EEF0F3` | 主窗口四角 |
| 窗口主体 / 卡片表面 | `#FFFFFF` | 40.50% |
| 次级表面（栏底、表头、拖放区、输入框） | `#FAFBFC` | 29.78% |
| 区块底 / 灰色标签 / 只读框 | `#F7F8FA` | 8.01% |
| 品牌蓝（按钮、选中、强调） | `#165DFF` | 1.86%，Arco 原色 |
| 选中蓝（导航、蓝色标签） | `#E8F3FF` | 2.83%（传输中） |
| 信息蓝（对话框说明横幅） | `#F2F6FF` | 7.00%（连接就绪态） |
| 成功 / 浅绿底 | `#00B42A` / `#E8FFEA` | 饱和色采样 |
| 警示 / 浅黄底 | `#FF7D00` / `#FFF7E8` | 同上 |
| 危险 / 浅红底 | `#F53F3F` / `#FFECE8` | 同上 |
| 主文本 | `#1D2129` | Arco 四级文本：`#4E5969 / #86909C / #C9CDD4` |
| 边框 | `#E9EBF0` | 原型为极细浅边框、卡片几乎无投影 |

> 结论：色彩体系就是 Arco Design 标准色。**此前实现的偏差不在语义色，而在中性色层次**——把整体背景铺成了 `#F2F3F5` 灰底，缺少「画布 → 圆角白窗口 → 白色卡片」三层结构。

### 13.2 Design Token 落地

`web/src/styles/tokens.css` 分两层：

- **原语层**（与主题无关）：字体、字号、字重、间距、圆角、动效、布局尺寸、阴影、z-index。新增 `--ll-window-inset: 22px` / `--ll-window-radius: 12px` / `--ll-commandbar-h: 52px` / `--ll-panel-w: 296px`。
- **语义层**（随 `[data-theme]` 切换）：画布 / 窗口 / 表面 1-3 级、边框、文本四级、品牌与选中蓝、信息蓝、成功/警示/危险四色组、遮罩、序列号专用色。
- **主题扩展方式**：新增主题只需再写一组 `:root[data-theme="xxx"]` 覆盖语义变量，组件层零改动。当前内置 `light`（原型基准）与 `dark`，`useTheme.ts` 支持 `system / light / dark` 并持久化。
- 组件层**禁止硬编码色值**，全部 `var(--ll-*)`；`ui.css` 提供按钮/输入/卡片/标签/胶囊/横幅/键值行/表格/开关等基础类。
- 新增 `web/src/components/common/Icon.vue`：统一内联 SVG 线性图标（60+ 图标），替换原型中不一致的 emoji，颜色走 `currentColor` 自动适配主题。

### 13.3 布局与内容对齐清单

| 项 | 之前 | 现在（对齐原型） |
|---|---|---|
| 窗口外壳 | 铺满、无圆角 | `#app` 画布 + `.ll-window` 圆角白窗口 + 投影；Tauri 真机自动加 `is-native` 铺满 |
| 命令栏 | 中央区底部 | **标题栏下方全宽**：`+ 选择文件 / 接收文件·重新扫描·全部暂停 / 搜索框或绿色已连接横幅 / 本机在线 / 设置齿轮` |
| 中央默认态 | 附近设备表 | `main-ready.png`：拖放区 + 5 列文件表（含汇总行）+ 选择连接方式双卡片 + 无任务条 |
| LAN 态 | 简化设备表 | `main-lan-mdns.png`：蓝色 mDNS 横幅 + 4 列设备表（发送按钮）+ 待发送文件表 + 提示条 |
| 传输中 | 两列任务卡 | `main-transferring.png`：通道信息卡（4 项）+ 两张任务卡（速率/进度/seq）+ 近 60s 速率曲线 + 排队条 |
| 左轨 | 头像 emoji、地址无标签 | 本机信息卡（DESKTOP-LL-01 / Lan4·Pub6·STUN + 默认标签）、导航角标 1/18、网络能力检测（运行中/已获取/可用/疑似 CGNAT） |
| 右栏 | 顺序与内容不同 | 就绪态：当前会话 → 最近连接 → 凭证安全；LAN 态增加快捷入口；传输中：当前会话(绿框+断开连接) → 传输层校验 → 凭证状态，**不显示最近连接** |
| 连接对话框 | 600px、副标题、3×2 方块胶囊 | 520px、链条图标+徽标、蓝色说明横幅、预填示例序列号、方形口令复选框、三按钮、**一行 6 枚胶囊**、页脚提示 |
| 错误态 | 两行错误卡、顺序不同 | 三行（原因/建议/说明）+ 同色横幅 + 诊断分区（E002/E004–E009）+ 顺序：输入框 → 胶囊 → 诊断 → 错误卡 → 按钮；E009 橙色且无胶囊、增加「改用局域网短码」 |
| 生成对话框 | 无说明横幅 | 蓝色说明横幅、候选地址 2/4 条卡、口令分区+加密链路、完成态双行序列号+凭证参数+黄色警示 |
| 设置 | 顶部 Tab | **左侧导航 + 右侧分组卡片**，四页分组/默认值逐项对齐，底部提示 + 取消/保存设置 |
| A 端握手 | 五步简版 | 蓝色对端提示 + 五步进度 + Blake3 应答计算卡 + 序列号条 + 红描边中断 + 蓝色进度横幅 |

### 13.4 桥接通信（非 HTTP）

`web/src/bridge/index.ts` 检测 Tauri 环境后用 `@tauri-apps/api` 的 `invoke` 调用 Rust command；浏览器预览态走 `bridge/mock.ts` 内存实现。全链路无任何 `fetch` / HTTP。Mock 文案已换成原型值（本机 `DESKTOP-LL-01`、附近设备三台、地址与文件名），避免数据覆盖设计文案。

### 13.5 对比截图索引（`docs/screenshots-ui-v2/`）

`01-main-ready-light` `02-main-ready-dark` `03-connect-ready` `04-connect-success` `05-error-E001` `06-error-E009` `07-generate-form` `08-generate-done` `09-wait-handshake` `10-settings-general` `11-settings-about` `12-main-lan` `13-main-transferring`

### 13.6 已知取舍

1. **PBKDF2 迭代次数**：关于页原型写 100000，PRD / D3 实现 / 安全页 / 生成页均为 10000 → 前端统一按 **10000** 展示，保持与协议层一致。
2. **预览辅助元素**：连接对话框页脚「错误码 E001–E009」在浏览器态是可下拉的演示选择器（用于注入 9 种错误态），生成完成态页脚增加「等待对方连接 ↗」以打开 A 端握手弹窗；二者均为极小小字，不影响原型视觉。
3. **默认主视图**：`init()` 不再因扫描到设备自动跳到 LAN 态，默认保持 `main-ready.png` 就绪态；LAN 态由菜单「设备 → 扫描局域网设备」触发，传输态由握手成功触发。
4. **窗口外壳**：原型是设计稿中的「桌面画布 + 圆角窗口」，Tauri 真机由系统绘制窗口圆角，故 `is-native` 下自动去掉画布与投影。

---

## 14. 新增「接收文件 / 传输记录」主窗口的布局重构（2026-11-15）

> 背景：Calicat 新增两张主窗口原型（页面层 id `760937d4…` / `2d647fe7…`），本地存为
> `docs/prototypes/main-receiving.png`、`docs/prototypes/main-history.png`。
> 用户要求「按照原型设计重新布局」。对比截图见 `docs/screenshots-ui-v3/`（8 张）。

### 14.1 结构性结论：从「中央区三态」升级为「左轨导航驱动的多视图」

新增原型的左轨导航是固定的四项：**发送文件 / 接收文件 / 传输记录 / 设置**，且「接收文件」「传输记录」各自有完整独立的命令栏、中央区、右栏与状态栏。因此它们**不是弹窗、也不是发送视图的第四第五态**，而是与发送视图平级的主视图。

实现上把原来的「单一 `CenterStage.vue` 内三态 + 两段占位」改为：

```
CenterStage.vue（薄分发器，按 state.mainView 选组件）
├── views/SendView.vue     就绪 / LAN 已发现 / 传输中（原三态，完整保留）
├── views/ReceiveView.vue  接收文件（接收中）
└── views/HistoryView.vue  传输记录
```

`useUi.ts` 中早已预留的 `mainView: 'send' | 'receive' | 'history'` 正式接线；`centerState` 降级为**仅 send 视图内部**的三态，避免视图状态互相污染。新增 `navigate()`、接收偏好开关、记录筛选/分页等交互状态。

### 14.2 三视图逐区规格（逐字对齐原型）

| 区域 | 发送文件 `send` | 接收文件 `receive` | 传输记录 `history` |
|---|---|---|---|
| 命令栏 | `+ 选择文件` / `接收文件·重新扫描·全部暂停` / 搜索或绿色已连接横幅 / 在线 / 齿轮 | `+ 选择文件` / `打开保存目录` / 分隔条 / 开关「允许被同网段设备发现」/ `mDNS 广播中 · 端口 42210` / 齿轮 | `+ 选择文件` / `导出 CSV` / `清空` / 搜索「搜索文件名或设备」/ `本机在线 · 监听 42210` / 齿轮 |
| 左轨导航角标 | 接收 1（灰）、记录 18（灰） | **接收 1（蓝）**、记录 18（灰） | 接收无角标、**记录 18（蓝）** |
| 左轨底部卡 | 网络能力检测 / 发现能力 / 当前通道能力（随三态） | **可接收状态**：mDNS 可发现 开启、防火墙放行 已添加、剩余空间 218 GB | **记录策略**：本地保存 保留 30 天、当前条数 18 条、不上传云端 |
| 中央标题行 | 发送文件 / 传输任务 + 副标题 + 标签组或清空按钮 | 接收文件 + 副标题「同网段设备可直连；跨网设备需提供序列号或口令」+ `D:\LanLink\Downloads` + 更改 | 传输记录 + 副标题「本机流水，含失败原因与错误码，便于排查」+ `今天 / 近 7 天 / 全部` |
| 中央区块 | 三态各自内容（见 §13.3） | 绿色通道条 → 正在接收卡（62%、34.2 MB/s、暂停/取消）→ 队列中 1 个文件 → 本次会话已完成 2 个 → 提示条 | 四统计卡（18 / 16 / 2 / 4.8 GB）→ 筛选行（全部方向·发送·接收 / 全部状态·仅失败）→ 8 列记录表 5 行 → 表底分页 → 提示条 |
| 右栏 | 当前会话 → 最近连接 → 凭证安全（三态变体见 §13.3） | **本机可被发现**（绿描边 + 复制本机邀请信息）/ **接收偏好**（两个开关）/ **接收安全** | **近 7 天**（发送 12 / 接收 6 / 失败 2 进度条）/ **失败原因**（E009、E006）/ **记录隐私** |
| 状态栏 | 就绪 / 传输中 + TCP 42210 + 黑名单 + 加密 | `接收中 · 1 个进行 · 1 个排队 · 34.2 MB/s` + TCP 42210 + 剩余空间 218 GB | `传输记录 · 共 18 条 · 失败 2` + TCP 42210 + 本机存储 · 保留 30 天 |

### 14.3 数据与分层

- 新增 `web/src/data/views.ts`：集中存放两张新原型的全部文案（接收通道、队列、已完成、四统计、5 行记录、概览条、失败分布、记录策略）。与 `data/errors.ts` 同样策略，避免运行态数据覆盖设计稿。
- `bridge/mock.ts` 仍是唯一的本机信息/附近设备来源；新视图不新增 HTTP 调用，全链路继续走 Tauri `invoke` 桥接。
- 新增图标：`chevronLeft` / `hardDrive` / `inbox`（`Icon.vue`），全部 `currentColor`。
- 组件层继续只消费 `var(--ll-*)`，未新增任何硬编码色值；深浅主题共用同一套语义变量，两张新视图均通过深色截图核验。

### 14.4 交互（预览态可用）

- 左轨 / 「视图」菜单可切换三视图，菜单勾选项随 `mainView` 动态更新；标题栏「传输中」标记仅在 `send + transferring` 出现。
- 接收视图：发现开关、两个偏好开关、暂停/继续、断开/重新允许接收。
- 记录视图：时间范围、方向、状态筛选、搜索（与命令栏共享）、分页高亮；「仅失败」筛选后表格收敛为 E009 / E006 两行并更新计数。
- **预览直达参数**（仅 `bridgeMode === 'mock'` 生效，Tauri 真机忽略）：`?view=receive|history|send&send=idle_no_lan|idle_with_lan|transferring`，用于视觉回归截图。

### 14.5 对比截图索引（`docs/screenshots-ui-v3/`）

`01-receive-light` `02-history-light` `03-send-ready` `04-send-transferring` `05-send-lan` `06-history-only-failed` `07-receive-dark` `08-history-dark`

其中 `03/04/05` 是回归验证：拆分 `CenterStage.vue` 后发送视图三态保持不变，右栏顺序与底部状态栏无裁切。

### 14.6 已知取舍

1. **分页数据**：原型表格展示 5 行但分页标「共 18 条」，预览态只内置这 5 行；切换页码仅切换高亮与计数文案，不伪造额外记录。
2. **状态筛选组配色**：原型「全部状态」为浅灰底、方向组「全部方向」为实心蓝，故状态组 active 使用 `brand-light` 浅蓝底而非实心蓝。
3. **左轨本机信息卡**：新原型地址行省略了「默认 / 未打包」小标签，但既有 `main-ready.png` 明确带这些标签且地址本身就会截断；保留既有实现，避免破坏上一轮对齐。
4. **窗口高度**：原型画布为 642/671px，预览视口统一 780px，故新视图底部留白略多于原型；所有区块仍完整可见、无裁切。

---

## 15. 发送页按「就绪态（锁定版）」定版重画（2026-11-15）

> 背景：用户反馈「发送页面和定版的原型界面不一致」。核对 Calicat 最新渲染后，
> 4 张发送相关原型（就绪态 / 锁定版 / mDNS / 传输中）与本地缓存 **MD5 完全一致**，
> 页面列表也没有新增发送页；经确认 **定版 = `prototypes/main-ready-locked.png`**
> （Calicat 页面「LanLink 主窗口 · 就绪态（锁定版）」）。
> 对比截图见 `docs/screenshots-ui-v4/`（6 张）。

### 15.1 结构性结论

旧实现把 `main-ready.png`（无设备、「选择连接方式」双卡片）当作默认态；定版则是「已发现 3 台设备」的填充态，中央区改为「选择目标设备」卡。因此：

- `idle_with_lan` 重画为定版布局，并升为**应用默认态**（`useUi.ts` 的 `centerState` 初值改为 `idle_with_lan`）。
- `idle_no_lan`（双连接方式卡片）保留为无设备变体，仅预览参数 `?send=idle_no_lan` 可直达。
- `transferring` 保持不变。三态结构未变，只是默认态与 LAN 态的「内容」被替换。

### 15.2 逐区规格（逐字对齐定版）

| 区域 | 定版内容 |
|---|---|
| 命令栏 | `+ 选择文件` / `打开保存目录`（原为 `接收文件`，见 §16 修订）/ 分隔 / `重新扫描设备`；右：绿点 `mDNS 广播中 · 已发现 3 台` + 齿轮。**无搜索框** |
| 左轨本机卡 | `DESKTOP-LL-01` / `Windows 11 · chen` / `Lan4 192.168.1.24` / `Pub6 2408:8207:…9c1d` / `STUN 121.36.80.24`，**无「默认 / 未打包」小标签** |
| 左轨导航 | 发送文件 `2`（蓝）、接收文件（无角标）、传输记录 `18`（灰）、设置 |
| 左轨底部卡 | `发现能力`：mDNS 广播 运行中 / 子网探测 42210 / 跨网序列号 可用，**紧跟导航、下方留白** |
| 中央标题行 | 发送文件 / `先选文件，再选目标：同网段直连，跨网用序列号` / `清空列表` |
| 中央区块 | 拖放区 → 2 行文件表（4.2 MB / 862 MB，表底 `已选 2 个文件，共 866.2 MB · 剩余空间 218 GB`）→ **选择目标设备卡** → 底部提示条 |
| 选择目标设备卡 | 浅蓝标题栏 `选择目标设备` + `最近扫描 2s 前`；绿行 `同网段 · mDNS 自动发现 3 台`；设备表 3 行（李工的 MacBook Pro `2 ms` 蓝色「连接」/ OFFICE-PC-07 `11 ms`「发送」/ chen 的 Android `24 ms`「发送」橙色）；分隔行 `跨网 / 异地设备 · 需用凭证` + `序列号自包含地址，经 IM 搬运；一个凭证仅可握手一次，有效期 15 分钟` + `生成序列号` / `粘贴序列号连接` |
| 底部提示条 | `当前无进行中的传输` / `选择目标设备后开始传输；进度、速率与剩余时间会在此实时显示` / `查看传输记录` |
| 右栏 | **当前会话**（连接状态 未连接 / 数据通道 未建立 / 待发送 2 个文件 · 866.2 MB / 通道加密 关闭 + `连接到远程设备`，普通白卡无绿描边）→ **最近连接**（同网段直连 / 序列号 E009）→ **凭证安全**（3 条定版文案）。**无「快捷入口」卡** |
| 状态栏 | `就绪 · 待发送 2 个文件 · 已发现 3 台设备` / `TCP 42210` / `会话黑名单 0 条` / `口令加密：关` |

### 15.3 数据与分层

- 新增 `web/src/data/send.ts`（`SEND_LOCKED`）：集中存放定版全部文案与设备/文件/右栏数据，与 `data/errors.ts`、`data/views.ts` 同一策略。
- `bridge/mock.ts` 附近设备 `platform` 文案同步定版（`macOS · 可接收` / `Windows 11 · 可接收` / `Android · 需对方确认`）。
- `Icon.vue` 新增 `compass`（选择目标设备标题图标），全部 `currentColor`，无硬编码色值。
- 桥接方式未变：Tauri `invoke` + 浏览器 mock，全链路无 HTTP。

### 15.4 布局修正

- 定版画布 1180×820，预览视口统一 1180×780：压缩 `SendView.vue` 的标题行 / 拖放区 / 表格行 / 卡片内边距与 `.center-scroll` gap，使「跨网凭证条 + 底部提示条」在 780px 内完整可见；窗口更矮时中央区仍可滚动。
- 左轨「发现能力」卡原本被 `margin-top: auto` 推到窗口底部，与定版不符；新增 `.cap-card--top` **仅在定版态生效**，接收 / 记录视图位置不变。
- 右栏去掉「快捷入口」卡与「当前会话」绿描边，补齐定版「凭证安全」卡。

### 15.5 对比截图索引（`docs/screenshots-ui-v4/`）

`01-send-locked-light` `02-send-locked-dark` `03-send-idle-no-lan` `04-send-transferring` `05-receive-light` `06-history-light`

其中 `03`–`06` 是回归验证：旧就绪态、传输中、接收文件、传输记录均未因行高压缩或默认态切换而裁切、错位。

### 15.6 已知取舍

1. 跨网凭证条说明文字在定版为单行，预览态字号压到 10px 保持单行；窗口更窄时按 `nowrap` 省略，不换行挤压按钮。
2. 定版原型无「默认 / 未打包」标签，仅定版态隐藏；旧就绪态与新增两视图维持既有实现，避免破坏 §13 / §14 的对齐成果。
3. 默认态从「无设备」改为「已发现 3 台设备」，命令栏不再显示搜索框；搜索能力保留在传输记录视图与旧就绪态变体。

---

## 16. 就绪页左上角按钮改为「打开保存目录」（2026-11-15）

### 16.1 需求与结论

用户要求把定版就绪页左上角命令栏的第二个按钮由 `接收文件` 改为 `打开保存目录`。该按钮原行为是 `navigate('receive')` 跳转到接收视图；改为打开本机接收目录后，**不再切换主视图**，接收视图仍由左轨导航与标题栏「视图」菜单进入。

适用范围：send 视图的两个就绪态（`idle_with_lan` 定版 / `idle_no_lan` 变体）；`transferring` 仍为 `全部暂停`。接收视图命令栏原有的 `打开保存目录` 按钮与标题栏「文件 → 打开下载目录」菜单项统一接上同一动作，避免三处入口行为不一致。

### 16.2 桥接实现（仍为 Tauri invoke，无 HTTP）

| 层 | 改动 |
|---|---|
| 前端状态 | `useUi.ts` 新增 `openSaveDir()`：调用 `invoke<{path:string}>('open_save_dir')`，成功后 `showToast('已打开保存目录 · <path>')`；新增 `toast` ref + 2.6s 自动消失 |
| 数据 | `data/send.ts` 新增 `SEND_LOCKED.openSaveDirLabel`，文案集中管理 |
| 桥接路由 | `bridge/index.ts` 新增 `open_save_dir` 分支；`bridge/mock.ts` 的 `mockOpenSaveDir()` 返回 `RECEIVE_CHANNEL.saveDir`（`D:\LanLink\Downloads`），预览态不触碰文件系统 |
| 桌面端 | `crates/lanlink-desktop/src/lib.rs` 新增私有 `#[tauri::command] fn open_save_dir()`：按平台解析 `<HOME>/Downloads/LanLink` → `create_dir_all` → `open` / `explorer` / `xdg-open` 打开系统文件管理器，返回真实路径；注册进 `generate_handler!`。目录由后端决定，避免原型里的 Windows 示例路径在 macOS/Linux 上被误创建 |
| 组件 | `CommandBar.vue` 第二按钮图标 `download → folder`、行为 `navigate → openSaveDir`；`App.vue` 新增 `ll-toast` 轻量提示 |

### 16.3 视觉位置取舍

`ll-toast` 初版固定在窗口底部状态栏上方，会压住中央区底部提示条「当前无进行中的传输 … 查看传输记录」。改为**标题栏中央空白区**（`tb-center` 在空闲态为空），`is-native` 下把 top 从 `var(--ll-window-inset) + 5px` 收敛到 `5px`，不遮挡菜单、命令栏与三栏内容；过渡动画由下浮改为上浮。

### 16.4 验证与截图索引（`docs/screenshots-ui-v5/`）

`01-send-open-save-dir`（按钮文案与命令栏无挤压）、`02-send-toast`（点击反馈，底部提示条完整可见）、`03-receive-open-save-dir`（接收视图入口回归）。

- `npm run build` 通过：79 模块，CSS 83.55 kB / JS 186.38 kB。
- `cargo check -p lanlink-desktop` 通过（新增 Tauri command 编译无误）。

---

## 17. 统一定版表格样式：去分割线 / 表头浅灰底 / 抬高行高（2026-11-15）

### 17.1 需求澄清

用户明确：**表格行之间不要分割线，表头与数据行之间也不要分割线**；表头颜色按原型取「比白稍深」的浅灰；行高再高一档。同时补上上一轮提到的「选择目标设备下方的『同网段』行有背景色」。

对 `main-ready-locked.png` 逐像素采样确认原型值：文件表表头 / 表底汇总行 / 「同网段」行均为 **`#FAFBFC`**（即 `--ll-color-surface-2`）；表格数据行为 `#FFFFFF`；行与行之间、表头与数据行之间**没有任何横线**（扫描 620–660 行线像素数为 0）。所以层次完全靠浅灰带 + 留白表达，而非边框。

### 17.2 全局规则（`web/src/styles/ui.css`）

| 项 | 改动 |
|---|---|
| `.ll-table th` | 新增 `background: var(--ll-color-surface-2)`（#FAFBFC）、文字 `--ll-color-text-secondary`、`--ll-fw-medium`、`padding: 9px var(--ll-space-3)`、`border: none` |
| `.ll-table td` | 去掉 `border-top`，改 `border: none`，`padding: 9px var(--ll-space-3)` |
| 删除 | `.ll-table tr:last-child td { border-bottom: none; }`（不再存在任何行线） |

### 17.3 各视图局部覆盖

- `views/SendView.vue`：`.file-table td` / `.device-table td` 由 `3px` 抬到 **`6px`**；`.tc-hint` 补 `background: var(--ll-color-surface-2)`；`.file-foot` 去掉 `border-top` 并补同色浅灰底；`.tc-cross` 去掉 `border-top`（设备表与跨网凭证条之间也不画线）。
- `views/HistoryView.vue`：`.history-table th/td` 由 `7px` 抬到 **`10px`**；`.table-foot` 去掉 `border-top`，保留浅灰底。
- `views/ReceiveView.vue`：队列 / 已完成列表（div 模拟表格）`.lc-head` / `.lc-row` 去掉 `border-bottom` / `border-top`，表头文字加深、加 `--ll-fw-medium`，padding 抬到 `9px` / `10px`；删掉 `.lc-row:first-of-type` 规则。

**保留的结构线**（不属于表格行）：标题栏 / 命令栏 / 状态栏边界、「选择目标设备」浅蓝标题栏下边框、右栏键值列表、设置页表单行、连接对话框诊断行。这些是卡片区块或键值列表，不是数据表格。

### 17.4 行高抬高后的布局补偿

定版页原本为塞进 780px 视口压得较紧，5 行表格抬高约 +30px。同步压缩：`.ch-title` 内边距 `8/4px → 6/2px`、`.dropzone` `padding 12px → 10px` 且 `gap 8px → 6px`、`.center-scroll` `gap 4px → 3px`、底部 `padding 20px → 12px`。结果：`01-send-locked` 中拖放区、文件表、选择目标设备卡（含跨网凭证条）、底部提示条**全部完整可见**，未被状态栏裁切。

### 17.5 验证与截图索引（`docs/screenshots-ui-v6/`）

`01-send-locked` `02-send-idle-no-lan` `03-history` `04-receive` `05-send-dark` `06-history-dark`

- `npm run build` 通过：79 模块，CSS 83.26 kB / JS 186.38 kB。
- 逐张核验：亮/暗主题下表格均无横线、表头浅灰底（深色主题自动映射为深灰）对比度正常、行高宽松、8 列记录表无挤压、三视图无裁切。
- 表头色若需再深一档，把 `.ll-table th` 的背景从 `--ll-color-surface-2`（#FAFBFC，原型值）切到 `--ll-color-surface-3`（#F7F8FA）即可，组件层无需改动。

## 18. 目标设备卡层次修复 + 表格行 hover（2026-11-15）

### 18.1 两个问题的结论

1. **「融为一块」**：§17 给「同网段 · mDNS 自动发现 3 台」行和设备表表头都上了 `--ll-color-surface-2`（#FAFBFC），同色相邻且无边界，视觉上连成一整片。
2. **表格是不是 Arco**：**没有用任何 UI 组件库**。表格是 `web/src/styles/ui.css` 里手写的 `.ll-table`（Vue3 + 纯 CSS + design token）；Arco 依赖在早期为保证构建可靠、主题可控已移除。行 hover 是漏写的规则，不是组件库没配置。

### 18.2 层次修复（`views/SendView.vue`）

| 区块 | 之前 | 现在 |
|---|---|---|
| `.tc-hint` 同网段状态条 | `#FAFBFC` 浅灰，与表头同色 | `--ll-color-success-bg` 极浅绿 + `border-bottom: 1px solid var(--ll-color-divider)`，padding 抬到 `7px` |
| 设备表表头 | `#FAFBFC` | 保持原型值不变 |
| `.tc-cross` 跨网凭证条 | 白底、仅 padding | 补 `--ll-color-surface-2` 浅灰底，与上方白色数据行分层 |

卡片自上而下形成 **浅蓝标题栏 → 浅绿状态条 → 灰表头 → 白数据行 → 灰凭证条** 五段节奏。`.tc-hint` 的下边界属于「状态条与表头的区块边界」，**不是表格行分隔线**，与 §17「表格行无线」不冲突。

### 18.3 行 hover

新增语义 token，与通用 `--ll-color-surface-hover` 分开，避免影响导航 / 菜单 / 窗口按钮等其它 hover：

- `tokens.css`：`--ll-color-row-hover` = light `#f2f3f5` / dark `#2b3240`（深色比通用 hover `#242a34` 更亮，保证整行反馈可见）。
- `ui.css`：`.ll-table tbody tr { transition: background-color .15s ease }`、`:hover { background: var(--ll-color-row-hover) }`、`:hover td { color: var(--ll-color-text) }`。
- `views/ReceiveView.vue`：队列 / 已完成列表是 **div 模拟行**，全局 `tbody tr` 规则覆盖不到，单独给 `.lc-row` 补同样的 transition 与 hover。

作用范围：4 处 `.ll-table`（Send 文件表 ×2、Send 设备表、History 记录表）+ 接收视图 2 个 div 列表；`thead` 不高亮。

### 18.4 验证与截图索引（`docs/screenshots-ui-v7/`）

`01-send-locked-light` `02-send-hover-device` `03-send-hover-file` `04-history-hover-row` `05-receive-hover-row` `06-send-locked-dark` `07-send-dark-hover` `08-target-card-crop`

- `npm run build` 通过：79 模块，CSS 83.65 kB / JS 186.38 kB。
- `08-target-card-crop` 局部放大确认浅绿状态条与灰表头不再融合；设备表 / 文件表 / 记录表 / 接收列表的 hover 均整行生效且覆盖所有列；深色下 hover 与浅绿条文字可读；定版页所有区块仍完整可见、无裁切。
- 若要把 hover 换成品牌蓝调，只需把 `--ll-color-row-hover` 的 light 值改为 `--ll-color-info-surface` 对应的 `#f2f6ff`，组件层不动。


## 19. 视觉风格（Visual Styles / Base Styles）主题体系（2026-11-15 定版）

### 19.1 需求与方向修正

用户要求「前端主题参考 DeepSeek-Reasonix-studio，实现同样的主题色彩」。
第一版实现参考的是该项目的**官方主题包 JSON**（`official-clear-blue.json` / `official-tea-green.json`）
与默认 OKLCH 浅色色板，做出 `prototype / studio / clear-blue / tea-green` 四套浅色色板。
用户用设置页截图纠正：**要的是「视觉风格 / Base Styles」那 6 套深色为主的风格 + 卡片式选择器**
（Graphite / Aurora / Slate / Carbon / Nocturne / Amber），不是浅色主题包。
因此 `studio / clear-blue / tea-green` 三套已废弃，本节记录最终实现。

### 19.2 参考来源与取色依据

| 来源 | 用途 |
|---|---|
| `/Users/yangbo/Downloads/DeepSeek-Reasonix-studio/docs/THEME_PACK.md` | 状态模型：`themeMode`(auto/light/dark) 与 `baseStyle` **相互独立**；6 套 base style 名称 |
| 用户提供的「视觉风格」设置页截图 | 6 张卡片的标题 / 气质标签 / 描述逐字对齐；强调色逐像素采样 |
| `/Users/yangbo/WorkBuddy/scpuny/frontend/packages/theme-ironlink/src/palettes.ts` | 6 套 base style 的 **light/dark 精确 hex**（`bg / bgSoft / bgElev / border / fg`） |
| `.../theme-ironlink/src/index.ts` | `RESERVED_COLORS`：success / warning / error / info 为保留色，**禁止被主题 accent 覆盖** |
| `.../apps/platform-shell/src/theme/ThemeSettings.vue` | 卡片式配色选择器形态 |

截图逐像素采样的强调色（页面底色 `#17162A`，选中描边 `#8B7CFF`）：

| 风格 | 强调色 | 深色中性（bg / bgSoft / bgElev / border / fg） |
|---|---|---|
| Graphite 石墨 | `#FF7B45` | `#14161a` / `#1a1d23` / `#22262e` / `#2a2f38` / `#e5e7eb` |
| Aurora 柔雾极光 | `#9E7BFF`（渐变第二色 `#8A99F7`） | `#0f1418` / `#15201f` / `#1c2a28` / `#243634` / `#dcebe8` |
| Slate 精炼 | `#478AF6` | `#1a1d21` / `#212529` / `#2a2f34` / `#333a40` / `#dfe3e8` |
| Carbon 深邃 | `#28D4D1` | `#0d0e10` / `#141518` / `#1b1c20` / `#26282d` / `#e6e7ea` |
| Nocturne 柔和 | `#8A8BF9` | `#0f1020` / `#171833` / `#1f2240` / `#2b2e52` / `#e3e2f5` |
| Amber 琥珀 | `#D76B38` | `#1c1714` / `#231b16` / `#2c211b` / `#3a2c22` / `#ece3da` |

### 19.3 状态模型与 CSS 落地

```
<html data-theme="light|dark" data-palette="graphite|aurora|slate|carbon|nocturne|amber">
                 ↑ 明暗 pref                      ↑ 视觉风格（两个维度互相独立）
```

- `prototype` 为原型基准，**不写 `data-palette`、不写任何覆盖**，沿用 `tokens.css`，前 18 节逐像素对齐成果完全不变，仍是应用默认。
- `web/src/styles/palettes.css`：6 套 × light/dark = 12 个覆盖块，选择器为
  `:root[data-palette="x"]`（浅色）与 `:root[data-palette="x"][data-theme="dark"]`（特异性更高）。
- **只覆盖中性色 + 品牌色变量**：画布 / 窗口 / 各级表面 / hover / 边框 / 分割线 / 文字 / titlebar /
  `brand*` / `info-surface` / `surface-active` / 序列号条。
  `success / warning / danger / info` 及其浅染、边框**一律不覆盖**，随 `tokens.css` 明暗切换 ——
  这就是 scpuny `RESERVED_COLORS` 规则，保证「同网段」绿条、E009 橙、E006 红在任何风格下语义稳定。
- 加载顺序 `tokens.css → palettes.css → base.css → ui.css`（`main.ts`）。组件层只消费 `var(--ll-*)`，
  新增一套风格只需补一组覆盖，无需改任何 `.vue`。
- **文字对比度是全局不变量**：生成器按中性色比例推导的 `text-secondary/tertiary/quaternary` 偏淡
  （深色 tertiary 曾低到 `#5D6667`，左轨小字几乎不可读）。最终方案是**色板层完全不覆盖四级文字色**，
  统一交给 `tokens.css` 的明暗基线（light `#1d2129/#4e5969/#86909c/#c9cdd4`，
  dark `#e8eaed/#a9b0bb/#767d8a/#4a505c`）。文字该多深只由明暗决定，不随风格漂移 —— 这是协调性的第一条规则。
- **主导航单独提升一级**：新增 `--ll-color-nav-text` / `--ll-fw-nav`，基准值就是 secondary/regular（原型不变），
  12 个视觉风格块覆盖为 `var(--ll-color-text)` / `medium`。左轨与设置页左轨消费这两个 token，
  解决「亮色风格下主要字发虚」，同时不动原型基准。

### 19.4 交互入口

| 文件 | 改动 |
|---|---|
| `web/src/data/themes.ts` | `PaletteId` 改为 `prototype + 6 风格`；`VISUAL_STYLES` 过滤掉基准项；每套含英文/中文名、气质标签、描述、三色预览 `swatch`、Aurora 渐变第二色、来源 |
| `web/src/styles/palettes.css` | 12 个覆盖块（约 405 行），生成自 scpuny `BASE_PALETTES` + 截图 accent |
| `web/src/composables/useTheme.ts` | `palette` / `setPalette()` / `resetPalette()`；`data-palette` 与 `ll-palette` 持久化；`?palette=` 预览直达优先级最高；**旧值 `studio/clear-blue/tea-green` 自动清除并回退 `prototype`** |
| `web/src/components/dialogs/SettingsDialog.vue` | **新增独立的「外观」页**（左轨第 2 项）：上为「主题」明暗分段控件，下为「视觉风格」3×2 共 6 张卡片（标题 / 标签 / 3 色块 / 描述 / 选中品牌描边 + 勾选圆点），右上「原型基准」一键恢复，底部保留色说明。通用页恢复为「启动（含界面语言）+ 文件与通知」两组完整可见。**窗体固定 720×590**（见 §19.6） |
| `web/src/components/layout/TitleBar.vue` | 「视图 → 视觉风格」子菜单只列 6 套 + 「原型基准」，真机同样可切换 |

设计取舍：把视觉风格做成**独立设置页**而不是塞进「通用 → 外观与语言」。
3×2 卡片放进通用页会把第三组「文件与通知」挤出可视区（第一版的教训），独立页同时更贴近参考截图的分区方式。

### 19.5 验证与截图索引（`docs/screenshots-ui-v9/`，19 张）

`01-prototype-light`（基准回归） `02-settings-general` `03-settings-appearance`
`04-settings-appearance-aurora-dark`（选中态） `05/06-graphite-light/dark` `07/08-aurora-light/dark`
`09/10-slate-light/dark` `11/12-carbon-light/dark` `13/14-nocturne-light/dark` `15/16-amber-light/dark`
`17-history-aurora-dark` `18-receive-carbon-dark` `19-send-slate-light`

- `npm run build` 通过：**81 模块**，CSS 97.70 kB / JS 189.98 kB。
- 逐张核验结论：默认原型色板未变；6 套风格在浅色下是「白/浅中性 + 各自强调色」，深色下是
  scpuny 中性色板 + 同款强调色；橙 / 紫 / 蓝 / 青 / 柔紫 / 暖橙六套区分明显，无蓝色串色；
  成功绿、E009 橙、E006 红在各风格下保持一致（RESERVED 生效）；
  发送 / 接收 / 记录三视图、表格 hover、目标设备卡层次在深色下均清晰；
  设置页通用三组与外观页 3×2 卡片均无裁切、无溢出。
- `docs/screenshots-ui-v8/` 为已废弃的第一版浅色主题包截图，仅作历史记录，不再作为验收基准。
- 想把某套设为应用默认：把 `data/themes.ts` 的 `DEFAULT_PALETTE` 改成对应 id 即可。


### 19.6 主题协调性修订 + 设置窗体固定尺寸（2026-11-15 第二轮）

用户反馈：切到其它主题后「亮色下主要字感觉不实」、深色下「配色不协调」，且设置窗体换选项卡会变大变小太突兀。
根因不是某一处色值，而是**色板层把可读性变量也一起主题化了**，以及饱和信息面与语义色相撞。

**四条协调性规则（已写入 `styles/palettes.css` 头注释）**

1. **四级文字色不覆盖**：从 12 个覆盖块中删除 `--ll-color-text*`（`text-inverse` 除外），
   全部回落到 `tokens.css` 明暗基线。文字对比度是全局可读性常量，不是风格资产。
2. **主导航 token 化**：`--ll-color-nav-text` / `--ll-fw-nav` 默认等于原型 secondary/regular，
   仅视觉风格块提升为主文字色 + medium。原型基准像素级不变（v10 `01-prototype-light` 采样确认主按钮仍是
   `#165DFF` + `#FFFFFF`，左轨仍是次级灰字）。
3. **品牌按钮文字用主色同色相深色调（on-primary）**：6 套 accent 都是中高亮度，白字对比度不足
   （Carbon 青绿 `#28D4D1` 上白字仅约 1.8:1），但上一轮一刀切的中性墨字 `#10121A` 又显得突兀。
   最终改为按每套 accent 派生同色相深色写入 `--ll-color-text-inverse`（完整色值表见 §19.8）；
   组件里散落的硬编码 `#fff`（标题栏 logo、文件类型 video 徽标、历史筛选/分页激活态、
   设置页勾选圆点与版本 logo、握手步骤圆点）全部换成该 token。
   语义色徽标（PDF 红 / ZIP 橙 / 成功绿 / 危险红）仍用白字，保持 RESERVED 语义。
4. **深色信息面中性化**：原先深色块的 `--ll-color-info-surface` 是高饱和紫 / 橙 / 青
   （aurora `#1D1E2F`、amber `#2F1F18`），导致「选择目标设备」紫色标题条 + 亮绿同网段状态条 + 灰表头三色相撞。
   改为各风格 `surface-2`/`surface-3` 的中性抬升面（aurora `#1F2B2A`、carbon `#1E1F22` 等），
   `info-border` 同步改中性边框。标题条现在只靠明度分层，绿色状态条成为卡片内唯一彩色带。
   另外浅色风格遮罩从 0.42 降到 0.30，避免打开对话框时整个背景被压灰、主文字显虚。

**设置窗体固定尺寸**

- `Modal` 宽度固定 `720`（原来按 tab 在 620/720 间切）；`.set-layout` 与 `.set-content` 都锁 `height: 440px`，
  内容超出由 `.set-content` 自身滚动，窗体外壳不再随内容伸缩。
- 「关于」页 `header-extra` 的 `ll-btn--sm` 是 26px，比 24px 关闭按钮高 2px，会把窗体撑到 592；
  给按钮加 `.set-head-btn { height: 24px }` 抹平。
- Playwright 逐 tab 实测：通用 / 外观 / 传输 / 安全与协议 / 关于 **全部 720×590**。

### 19.7 验证与截图索引（`docs/screenshots-ui-v10/`，12 张）

`01-prototype-light`（基准回归） `02-prototype-dark` `03-aurora-light`（主要字）
`04-settings-general-aurora-light` `05-settings-appearance-aurora-light`
`06-settings-appearance-aurora-dark` `07-history-aurora-light`
`08-aurora-dark`（协调性） `09-carbon-dark` `10-graphite-dark` `11-amber-light`
`12-receive-carbon-dark`

- `npm run build` 通过：**81 模块**，CSS 97.40 kB / JS 189.92 kB。
- 逐张核验：原型基准未被改动；浅色风格主导航 / 设置导航文字转为深色实字，遮罩不再压灰；
  深色下目标卡标题条中性、绿色状态条独立成层、紫/青品牌按钮墨字清晰；
  6 套风格亮暗均无撞色；外观页 6 卡与说明完整无裁切；5 个选项卡窗体尺寸完全一致。
- `docs/screenshots-ui-v9/` 保留为上一版视觉风格截图；`v8/` 为已废弃的浅色主题包方案。


### 19.8 主按钮文字「补色」：on-primary 同色相深色派生（2026-11-15 第三轮）

用户反馈：切主题后主按钮仍是纯黑字，太突兀，应该「补色」。
根因：上一轮为修白字对比不足，把 12 个色板块的 `--ll-color-text-inverse` **统一写死** `#10121A`——
那是中性黑，和彩色按钮底没有任何色彩关联。

**派生算法**：保留每套 accent 的 H/S（`S = min(s, 0.90)`），把 L 从 0.26 逐步压到 0.10，
取第一个同时满足「常态底 ≥ 5:1、hover 底 ≥ 4:1」的值，即**在该对比度下尽可能亮、色相最明显**的深色调。
这比中性黑柔和，同时避免回到对比不足的白字。

| 风格 | accent | on-primary 文字 | 常态对比 | hover 对比 |
|---|---|---|---|---|
| Graphite 石墨 | `#FF7B45` | `#571D05` 深橙棕 | 5.17:1 | 4.04:1 |
| Aurora 柔雾极光 | `#9E7BFF` | `#22066F` 深紫 | 5.08:1 | 6.03:1 |
| Slate 精炼 | `#478AF6` | `#031A3F` 深海军蓝 | 5.09:1 | 4.02:1 |
| Carbon 深邃 | `#28D4D1` | `#0F4D4C` 深青 | 5.21:1 | 4.05:1 |
| Nocturne 柔和 | `#8A8BF9` | `#07087E` 深靛 | 5.32:1 | 4.18:1 |
| Amber 琥珀 | `#D76B38` | `#2A1309` 深褐 | 5.04:1 | 4.01:1 |

**约束与影响面**

- 明/暗 12 个块同值（两套的 `--ll-color-brand` 相同）；`tokens.css` 不动，
  原型基准仍是 `#165DFF` + `#FFFFFF`（v11 `01-btn-prototype.png` 采样确认）。
- 组件层只消费 `var(--ll-color-text-inverse)`，没有新增硬编码色值；图标用 `currentColor` 自动跟随。
  覆盖点：右栏「连接到远程设备」、命令栏「选择文件」、目标设备卡「连接」/「生成序列号」、
  生成弹窗主按钮、连接弹窗「连接」、历史筛选/分页激活态、设置页勾选圆点与关于页 logo、
  握手步骤圆点、video 文件类型徽标。
- 语义色徽标（PDF 红 / ZIP 橙 / 成功绿 / 危险红）仍用白字，RESERVED 规则不变。
- 若日后要换成「深底白字」或「浅色调按钮 + 同色深字」，只需改 12 个块的
  `--ll-color-text-inverse` 与 `--ll-color-brand`，组件层零改动。

### 19.9 验证与截图索引（`docs/screenshots-ui-v11/`，16 张）

`01-btn-prototype`（基准蓝底白字回归） `02-full-prototype-light`
`03-btn-graphite-light` `04-btn-aurora-light` `04b-full-aurora-light`
`05-btn-slate-light` `06-btn-carbon-light` `06b-full-carbon-light`
`07-btn-nocturne-light` `08-btn-amber-light`
`10-btn-aurora-dark` `10b-full-aurora-dark` `11-btn-carbon-dark` `11b-full-carbon-dark`
`12-connect-aurora-light`（连接弹窗） `13-settings-appearance-aurora-light`

- `npm run build` 通过：**81 模块**，CSS 97.40 kB / JS 189.92 kB；`cargo check -p lanlink-desktop` 通过。
- 逐张核验：六套亮/暗按钮文字均带各自色相、不再读作中性黑，对比度清晰；
  整窗、连接弹窗、外观设置页均无撞色；设置窗体仍为 **720×590**（Playwright 实测），切换选项卡不跳大小。
- `v10/` 是上一轮「中性墨字」方案截图，`v9/` 为视觉风格定版，`v8/` 为已废弃的浅色主题包方案。


## 20. 两处定版原型细节修正（2026-11-15）

用户指出两处与原型不一致：①「传输记录」时间范围选中项「近 7 天」缺右边框；
②「选择目标设备」卡底部跨网凭证条**第二行不该有背景色**。两处均已按 `main-history.png` /
`main-ready-locked.png` 逐像素复核后修正。

### 20.1 时间范围分段控件：四边完整边框

**根因**（`web/src/components/layout/views/HistoryView.vue`）：
```css
/* 旧：所有按钮去掉右边框，只有最后一项补回 */
.range-btn { border: 1px solid var(--ll-color-border); border-right: none; }
.range-btn:last-child { border-right: 1px solid var(--ll-color-border); }
.range-btn.active + .range-btn { border-left: none; }
```
选中项虽然把 `border-color` 改成品牌色，但 `border-right` 宽度仍是 **0**；
紧随其后的 `.active + .range-btn` 又把「全部」的左边框也去掉，于是选中项右侧完全没有线。

**修正**：四边都保留 1px，用 `margin-left:-1px` 让相邻边叠成一条，选中项 `z-index:1` 让蓝框压在灰框之上：
```css
.range-btn { border: 1px solid var(--ll-color-border); margin-left: -1px; }
.range-btn:first-child { margin-left: 0; border-radius: var(--ll-radius-sm) 0 0 var(--ll-radius-sm); }
.range-btn:last-child { border-radius: 0 var(--ll-radius-sm) var(--ll-radius-sm) 0; }
.range-btn.active {
  position: relative; z-index: 1;
  color: var(--ll-color-brand);
  background: var(--ll-color-brand-light);
  border-color: var(--ll-color-brand);
  font-weight: var(--ll-fw-medium);
}
```
像素验证（v12 `01-range-prototype.png`，2× 截图 324×52）：蓝色竖线出现在
`x=98–99`（左）与 `x=224–225`（右），蓝色横线出现在 `y=0–1` 与 `y=50–51` —— 四边完整。

### 20.2 跨网凭证条：第一行浅灰、第二行纯白

原型 `main-ready-locked.png` 逐像素采样（区域 `x=250..800`）：

| 区域 | 主导底色 | 占比 |
|---|---|---|
| 第一行「跨网 / 异地设备 · 需用凭证」`y=610..640` | `#FAFBFC` | 73.0% |
| 第二行「序列号自包含地址…」+ 两个按钮 `y=645..685` | `#FFFFFF` | 77.2% |

我们此前把整个 `.tc-cross` 铺了 `var(--ll-color-surface-2)`，用户反馈截图里
上下两行都是 `#FAFBFC`（69% / 73%），第二行确实多了灰底。

**修正**（`web/src/components/layout/views/SendView.vue`）：把背景从容器下移到两行自身，
第一行用 `--ll-color-surface-2`（与表头同色），第二行用 `--ll-color-surface`（白），
padding 同步下移，仍不画任何分割线；两行文字由 tertiary 提为
`--ll-color-text-secondary`（原型 `#4E5969`），图标 `currentColor` 自动跟随。

```css
.tc-cross { background: var(--ll-color-surface); }
.tc-cross-title { padding: 6px var(--ll-space-3);
  background: var(--ll-color-surface-2); color: var(--ll-color-text-secondary); }
.tc-cross-body  { padding: var(--ll-space-2) var(--ll-space-3);
  background: var(--ll-color-surface); }
```

修正后采样（v12 `04-cross-prototype.png`）：上半 `#FAFBFC` 77.0%、下半 `#FFFFFF` 78.3%，
与原型一致。视觉风格下自动跟随：Aurora 亮色 `#E8F1F2` / `#F2F7F8`，
Aurora 深色 `#162021` / `#1C2A28`，层次仍在但不会融成一块。

> 这条同时更新了 §17 那条「`.tc-cross` 跨网凭证条补浅灰底」的旧描述：
> 浅灰只属于第一行标题，不是整块。

### 20.3 验证与截图索引（`docs/screenshots-ui-v12/`，14 张）

`01-range-prototype` `01-full-prototype`（时间范围） `02-range-aurora-light`
`02-full-aurora-light` `03-range-aurora-dark` `03-full-aurora-dark`
`04-cross-prototype` `04-full-prototype`（跨网条） `05-cross-aurora-light`
`05-full-aurora-light` `06-cross-aurora-dark` `06-full-aurora-dark`
`07-cross-carbon-dark` `07-full-carbon-dark`

- `npm run build` 通过：**81 模块**，CSS 97.41 kB / JS 189.92 kB；`cargo check -p lanlink-desktop` 通过。
- 逐张核验：原型基准 / 浅色 Aurora / 深色 Aurora / 深色 Carbon 下，选中项四边描边完整、
  相邻按钮无双线断线；跨网条两行底色分层清楚、文字与按钮清晰；设置窗体 720×590 未受影响。


## 21. Tauri 开发模式 + bridge 真机通信接线（2026-11-15）

用户定调：「布局差不多了，现在程序通过 Rust 启动，开发模式下不编译前端、直接加载前端，
但前后端通信用 bridge」。本节记录把 bridge 从「假接通」修到「真调用 Rust」的全过程。

### 21.1 接线前的四个硬伤

| # | 问题 | 后果 |
|---|---|---|
| 1 | `web` 未安装 `@tauri-apps/api`，bridge 用 `import('@tauri-apps/api/core')` + `@vite-ignore` 动态导入 | Tauri 态导入必失败 → 静默回退 mock，**从没真正调到 Rust** |
| 2 | Rust 只注册 3 个 command（generate/decode/open_save_dir），UI 实际调 6 个 | `get_self_info` / `scan_lan` / `connect` 在真机态报 command not found |
| 3 | Rust 返回结构与 `types/invite.ts` 不匹配（generate 返回裸 String、decode 字段名不同、open_save_dir 返回 String 而非 `{path}`） | 即使接通，前端拿到的数据也渲染不出来 |
| 4 | `devUrl=localhost:5173` 而 vite `strictPort:false`，dev server 漂到 5174 | Tauri 窗口加载不到前端 / 加载到旧页 |

### 21.2 修复后的数据流

```
开发模式：cargo tauri dev
  └─ beforeDevCommand: npm run dev --prefix ../web      → Vite 监听 127.0.0.1:5173 (strictPort)
  └─ devUrl: http://127.0.0.1:5173                       → Tauri 窗口直接加载 dev server（前端热更新，不编译）

生产模式：cargo tauri build
  └─ beforeBuildCommand: npm run build → web/dist
  └─ frontendDist: ../../web/dist

通信（两种模式一致，全程 IPC，无 HTTP）：
  Vue 组件 → bridge/index.ts invoke(cmd,args)
      ├─ Tauri 运行时 → @tauri-apps/api/core 的 invoke → Rust #[tauri::command]
      └─ 浏览器预览   → bridge/mock.ts 内存实现
  真机里加 ?bridge=mock 可强制走 mock，用于 A/B 对比真/Rust 输出
```

### 21.3 关键改动

**`web/src/bridge/index.ts`**
- 由动态 `@vite-ignore` 导入改为**静态** `import { invoke as tauriInvoke } from '@tauri-apps/api/core'`。
  静态导入才会被 Vite 打进 bundle（构建后确认含 `__TAURI_INTERNALS__`），是真接通的前提。
- Tauri 态调用失败**不再静默降级**到 mock——command 缺失直接抛错，便于发现问题。
- 新增 `forceMock()`：`?bridge=mock` 在真机里强制走内存 mock。
- `bridgeMode()` 同步考虑 `forceMock()`，UI 角标准确。

**`crates/lanlink-desktop/src/lib.rs`（整体重写 6 个 command）**

| command | 返回结构 | 实现程度 |
|---|---|---|
| `generate_serial(candidates, password?)` | `GenerateResult` | **真协议编码**：本机真实出口 IP 构造候选 → UUID v4 → `encode_block` + `to_serial`。前端传的 `value` 是原型描述文案（非 IP），Rust 忽略并按选中的 `kind` 采集真实地址 |
| `decode_serial(serial, password?)` | `DecodedInvite` | **真解码校验**：`codec::decode_serial`，失败回传 `E001: …` 错误码字符串 |
| `connect(serial, password?)` | `SessionInfo` | 骨架：先 decode 拿候选地址，真 P2P 通道 / Blake3 握手未落地，`rtt/took_ms` 暂为 0 |
| `scan_lan()` | `NearbyDevice[]` | 骨架：真 mDNS 未落地，返回空列表（左轨设备卡依赖后续接真发现） |
| `get_self_info()` | `SelfInfo` | **真实采集**：设备名取环境变量，IPv4/IPv6 用 UDP connect 查路由表得到出口 IP（不发包、零依赖） |
| `open_save_dir()` | `SaveDirResult {path}` | 真机建目录并 open/explorer/xdg-open 打开 |

- 所有结构加 `#[serde(rename_all = "camelCase")]`，字段与 `types/invite.ts` 一一对应。
- 候选种类映射：前端 `Pub6/Pub4/Lan4/ULA` → 协议层 `Pub6/Pub4/Lan4/Lan6`。

**端口固定**
- `vite.config.ts`：`server.port=5173`、`strictPort=true`、`host=127.0.0.1`。
- `tauri.conf.json`：`devUrl=http://127.0.0.1:5173`（不用 localhost，避免 IPv6 解析差异）。

### 21.4 验证

- `cargo check -p lanlink-desktop` 通过。
- `npm run build` 通过：**83 模块**（含 `@tauri-apps/api`），CSS 97.41 kB / JS 189.96 kB；bundle 内确认含 `__TAURI_INTERNALS__`。
- 浏览器态 mock 回归：`docs/screenshots-ui-v13/` 3 张（发送 / 记录 / 接收），`?bridge=mock` 直达，三视图无 console error。
- `tauri dev` 真机验证（2026-11-15）：Vite 5173 就绪 → 编译完成 → `Running target/debug/lanlink-desktop`；Rust 进程日志出现 `[lanlink-bridge] get_self_info` / `[lanlink-bridge] scan_lan`（`lib.rs` 的 `bridge_log` 探针，仅 debug 构建输出），确认前端经 Tauri IPC 真实调用 Rust command，而非静默回退 mock。

### 21.5 后续（尚未落地，非本次范围）

- `connect` 接真 P2P 通道 + Blake3 动态质询握手状态机（`handshake.rs` 已就绪，待接入 command）。
- `scan_lan` 接真 mDNS / 局域网广播，替换空列表。
- `get_self_info.stun` 接真 STUN 观测端点（当前为空串）。
- 设置页配置项（保存目录、端口）持久化后覆盖 Rust 默认值。

---

## 22. 自定义窗体（去除系统原生标题栏，区分 macOS / 非 macOS）（2026-11-15）

### 22.1 问题

`tauri.conf.json` 原先未配置 `decorations`，窗口使用系统原生标题栏；而前端 `TitleBar.vue` 是原型里的自绘标题栏，二者同时存在 → **双标题栏**。且原 `.tb-win` 三键用的是 Electron 的 `-webkit-app-region`（Tauri v2 不识别），按钮也没有点击行为。

### 22.2 平台分支策略

两个平台的「无边框」代价不同，因此不采用统一的 `decorations:false`：

| 平台 | 配置 | 交通灯 / 三键 | 圆角与阴影 | 拖拽 |
|---|---|---|---|---|
| macOS | `decorations:true` + `titleBarStyle:"Overlay"` + `hiddenTitle:true` + `trafficLightPosition` | **系统交通灯**（红黄绿），浮在内容之上 | 系统绘制（保留） | `data-tauri-drag-region` |
| Windows / Linux | `decorations:false` | **自绘三键**（最小化 / 最大化 / 关闭）置于右上角 | 无（矩形窗口） | `data-tauri-drag-region` |

macOS 选择「保留交通灯」而非全部自绘，原因：系统交通灯自带 hover 图标、全屏/缩放语义、辅助功能与窗口管理行为，自绘无法等价复刻；`Overlay` 已能实现「隐藏标题、内容延伸到顶部」的自定义观感。

### 22.3 配置分层

Tauri v2 支持平台专属配置，但**数组字段是整体替换而非深合并**，因此 macOS 文件必须写完整 `windows` 数组：

- `crates/lanlink-desktop/tauri.conf.json` —— 基础配置（Windows / Linux 行为）
- `crates/lanlink-desktop/tauri.macos.conf.json` —— macOS 覆盖

```json
// tauri.conf.json
"windows": [{
  "label": "main", "title": "LanLink",
  "width": 1180, "height": 820,
  "minWidth": 900, "minHeight": 620,
  "resizable": true, "center": true,
  "decorations": false
}]
```

```json
// tauri.macos.conf.json
"windows": [{
  "label": "main", "title": "LanLink",
  "width": 1180, "height": 820,
  "minWidth": 900, "minHeight": 620,
  "resizable": true, "center": true,
  "decorations": true,
  "hiddenTitle": true,
  "titleBarStyle": "Overlay",
  "trafficLightPosition": { "x": 16, "y": 15 }
}]
```

`trafficLightPosition` 取 `y=15`：标题栏高 42px、交通灯直径约 12px，`(42-12)/2 = 15` 垂直居中；`x=16` 贴近系统默认左边距。

### 22.4 前端适配

**`web/src/composables/usePlatform.ts`（新增）**
- `detectPlatform()` 依次尝试 `userAgentData.platform` → `navigator.platform` → UA 正则。
- 导出 `PLATFORM` / `IS_MAC`。用 UA 而不用 IPC 的原因：`useUi.init()` 是异步的，标题栏首帧就需要平台分支，UA 判断无时序依赖，浏览器 mock 态也一致。

**`web/src/components/layout/TitleBar.vue`**
- `header` 增加 `:class="{ 'is-macos': IS_MAC }"` 与 `data-tauri-drag-region`。
- macOS：`.titlebar.is-macos { padding-left: 78px }` 给交通灯让位（交通灯占 x=16→68，留 10px 余量）。
- 非 macOS：`v-if="!IS_MAC"` 渲染 `.tb-win` 三键，绑定 `getCurrentWindow().minimize() / toggleMaximize() / close()`，仅在 `bridgeMode === 'tauri'` 时执行（浏览器预览态静默忽略）。
- 删除 `-webkit-app-region`：Tauri 的拖拽由 `data-tauri-drag-region` 驱动，Electron 属性在 Tauri 无效。
- 拖拽属性同时加在 `header` / `.tb-left` / `.tb-center` / 文本节点上：Tauri 只对**带该属性的元素本身**响应 mousedown，子元素会吞掉事件，故需逐个标注；`AppMenu` 与窗口按钮不加，保持可点击。

### 22.5 权限（capabilities）

项目此前**没有** `capabilities/` 目录。Tauri v2 中 `core:window` 的 `default` 集合只含只读类权限（`is-*` / `get-*`），`minimize` / `toggle-maximize` / `close` / `start-dragging` 必须显式授权，否则 IPC 会被拒绝、窗口操作静默失败。

新增 `crates/lanlink-desktop/capabilities/default.json`：

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-minimize",
    "core:window:allow-unminimize",
    "core:window:allow-toggle-maximize",
    "core:window:allow-is-maximized",
    "core:window:allow-close",
    "core:window:allow-start-dragging"
  ]
}
```

`"windows": ["main"]` 必须与 `tauri.conf.json` 的 `label: "main"` 对应；匹配不到窗口时权限不生效。

### 22.6 窗口尺寸

对齐定版原型画布：`1180 × 820`（原 960 × 640 与定版不符），`minWidth 900` / `minHeight 620` 保证三栏不被挤压。

### 22.7 注意事项

- macOS `Overlay` 下 webview 内容从窗口 `y=0` 开始，标题栏顶部两角由系统圆角裁切，`.ll-window.is-native` 必须保持 `border-radius:0 / box-shadow:none`（已具备）。
- 非 macOS 无边框窗口没有系统圆角与阴影，属既定取舍；若要圆角，需 `transparent:true` + 自绘圆角 + 处理 resize 边缘，成本较高。
- 双击标题栏最大化由 Tauri 的 drag region 内建处理（Windows / Linux），无需额外代码。
- **不要用 `tauri dev -c tauri.conf.json` 启动**：`--config` 是「再合并一次」语义，会把主配置的 `windows` 数组（`decorations:false`）整体盖回到平台合并结果之上，导致 macOS 退回无装饰窗口——交通灯消失、圆角丢失。直接 `tauri dev` 即可，CLI 会自动发现同目录 `tauri.conf.json` 并先与 `tauri.macos.conf.json` 合并。

### 22.8 验证

- `cargo check -p lanlink-desktop` 通过（4.76s）：`tauri.conf.json` 与 `capabilities/default.json` 均通过 `tauri-build` 校验。
- `npm run build` 通过：**88 模块**（新增 `@tauri-apps/api/window`、`usePlatform.ts`），CSS 97.47 kB / JS 206.30 kB。
- 窗口探针（`lib.rs` 的 `setup`，仅 debug 输出）：
  `[lanlink-window] decorated=Ok(true) size=Ok(PhysicalSize { width: 2360, height: 1642 })`
  → decorations 生效、窗口 1180×821 逻辑像素。
- 桥接探针（`bridge_log`，仅 debug 输出）：Rust 进程日志出现 `[lanlink-bridge] get_self_info` / `[lanlink-bridge] scan_lan`，证明前端确实经 Tauri IPC 调用 Rust（mock 分支不会产生该日志）。
- 截图核验：顶部为自绘标题栏（logo + LanLink + 菜单项），**红黄绿交通灯完整可见并与 logo 垂直居中**，窗口左上角为系统圆角。
- **Windows / Linux 分支未在本机验证**（当前仅有 macOS 环境）：`decorations:false` + 自绘三键的代码路径已通过类型检查与构建，需在 Windows 实机复核按钮点击、双击最大化与无边框边缘拖拽。

### 22.9 主窗体 1px 黑色边框（系统深色 + 应用浅色）

**现象**：系统外观为深色、应用手动切到浅色时，窗口四周出现 1px 纯黑描边。

**定位**：前端 `.ll-window.is-native` 已是 `border:none`，没有边框来源。逐像素测量窗口左边缘（整屏转 BMP，扫一条横向像素线读 RGB）：

```
x=348  rgb(33, 6, 84)     ← 桌面壁纸（深紫）
x=349  rgb(1, 0, 3)       ← 1px 纯黑
x=350  rgb(243,240,249)   ← 窗口内容（浅色）
```

`defaults read -g AppleInterfaceStyle` 返回 **Dark**，而应用主题是 light —— 两者不一致。

**根因**：`NSWindow` 的边框与阴影取 `effectiveAppearance`，默认**跟随系统外观**而非应用主题。系统深色时边框取深色，压在浅色内容外围形成黑边。这不是 CSS 能覆盖的层级（边框在 webview 之外、窗口之下）。

**修复**：`useTheme.applyToDom()` 在写入 `data-theme` 后调用 `getCurrentWindow().setTheme(resolved)`，把原生窗口外观同步成与应用一致的 light / dark；新增权限 `core:window:allow-set-theme`。浏览器预览态没有原生窗口，`isTauri()` 判断后直接跳过。

**验证**：用 `?theme=light`（新增的调试参数，与 `?palette=` 对称，只影响首屏、不写 localStorage）在系统 Dark 下强制浅色，逐像素复测窗口四边：

| 位置 | 修复前 | 修复后 |
|---|---|---|
| 左边 x=500 | `rgb(1,0,3)` 1px 纯黑 | `rgb(245,244,252)` 直接衔接内容 |
| 右边 x=2860 | — | `rgb(241,238,248)` → 壁纸 `rgb(53,109,102)`，无黑线 |
| 顶边 / 底边 | — | 无黑线 |

**注意**：`system` 模式下 `resolved` 就是系统主题，窗口外观与系统一致，边框本就协调；只有「系统深色 + 应用浅色（或反之）」的错配才会暴露问题。



---

## 23. 菜单栏平台差异：macOS 系统菜单栏 / Windows 窗口内菜单（2026-11-15）

原型里的「文件 / 设备 / 视图 / 帮助」画在窗口标题栏内，这只是 **Windows / Linux 的做法**。macOS 的菜单属于「应用」，固定渲染在屏幕顶部，窗口内不放菜单。两者不能共用一套实现。

### 23.1 平台分工

| 平台 | 菜单承载 | 实现位置 | 窗口标题栏里的菜单 |
|---|---|---|---|
| macOS | 屏幕顶部系统菜单栏（NSMenu） | Rust `src/menu.rs`，`app.set_menu()` | **不渲染** |
| Windows / Linux | 窗口标题栏内自绘 | 前端 `AppMenu.vue` | 渲染 |

判定在前端用 `IS_MAC`（`composables/usePlatform.ts`），在 Rust 用 `#[cfg(target_os = "macos")]`，两边一致。

### 23.2 Rust 侧（`crates/lanlink-desktop/src/menu.rs`）

- 用 `SubmenuBuilder` / `MenuItemBuilder` / `PredefinedMenuItem` 构建，菜单顺序遵循 macOS HIG：
  `LanLink`（应用菜单）→ 文件 → 设备 → 视图 → 窗口 → 帮助。
- 应用菜单放系统语义项，全部用 `*_with_text()` 显式给中文：关于（原生 About 面板，带 `AboutMetadata`）/ 设置… `Cmd+,` / 服务 / 隐藏 / 隐藏其他 / 全部显示 / 退出。
- **省略「编辑」菜单**：当前没有富文本编辑场景，`input` / `textarea` 的复制粘贴由 macOS 默认实现兜底；将来加「编辑」时补 `PredefinedMenuItem::{undo, redo, cut, copy, paste, select_all}`。
- 菜单 id 约定 `key` 或 `key|value`（用于「外观」「视觉风格」二级菜单），`split_id()` 拆成 `{ key, value }`。
- 菜单项**不放业务逻辑**，只 `emit("ll-menu", MenuAction)` 转发给前端。

### 23.3 前端接线

`TitleBar.vue` 挂载时 `listen('ll-menu')`，把事件并入与窗口内菜单**同一个** `onAction(key, value)`：

```ts
unlistenMenu = await listen<{ key: string; value?: string | null }>('ll-menu', (e) => {
  onAction({ key: e.payload.key, value: e.payload.value ?? undefined })
})
```

因此两个平台共享同一套动作分发，`theme` / `palette` 同时接受窗口内菜单的中文短名和原生菜单的 machine id（`light` / `graphite` …）。

### 23.4 加速键（仅 macOS）

`Cmd+O` 选择文件、`Cmd+Shift+O` 选择文件夹、`Cmd+Shift+V` 从剪贴板发送、`Cmd+Shift+D` 打开下载目录、`Cmd+R` 扫描局域网、`Cmd+G` / `Cmd+L` / `Cmd+K` 生成序列号 / 短码 / 连接、`Cmd+1/2/3` 切换发送 / 接收 / 记录。`Cmd+W`、`Cmd+M`、`Cmd+,`、`Cmd+Q`、`Cmd+H` 用系统预定义项，自带本地化文本与行为。

> 注意：`Cmd+H`（隐藏）、`Cmd+M`（最小化）、`Cmd+Q`（退出）由系统占用，自定义项不要冲突。

### 23.5 验证

1. **菜单栏渲染**：截图确认屏幕顶部为 `LanLink 文件 设备 视图 窗口 帮助`，且窗口标题栏内只剩 logo + 名称 + 副标题（无重复菜单）。
2. **事件链路**：无 UI 点击权限时用探针验证 ——

   ```bash
   LL_MENU_PROBE=1 ../../web/node_modules/.bin/tauri dev
   ```

   debug 构建下每 3 秒 `emit("ll-menu", { key: "view-recv" })`，界面若从中切到「接收文件」视图，即证明 `Rust emit → 前端 listen → onAction` 全链路通畅。探针仅在 `debug_assertions` + 环境变量同时满足时生效，正常启动无影响。

3. 未实机验证：Windows / Linux 窗口内菜单为既有实现，本轮未改动。

### 23.6 标题栏品牌块布局（2026-11-15 修订）

原实现是「图标 + LanLink + 副标题」一条横排，副标题挤占横向空间，标题栏看起来偏长。改为：

```
[交通灯 78px]  [图标]  LanLink            ← 14px / semibold
                      局域网 / 跨网 P2P…  ← 11px / tertiary
                             LanLink      ← 仅 macOS，居中作窗口标题替身
```

- `.tb-brand` 用 `flex-direction: column` + `gap:1px` + `line-height:1.15`，两行总高约 30px，在 42px 标题栏内上下居中留 6px。
- 名称 `--ll-fs-lg`（14px，比原 `--ll-fs-md` 大一号），副标题 `--ll-fs-xs`（11px）。
- 图标容器 26×26（原 24），图形 16px，视觉上与两行文本块等高。
- **居中标题仅在 macOS 出现**：该平台 `hiddenTitle: true` 不显示系统标题，用 `.tb-title`（13px / medium / text-secondary）补一个窗口标题。居中区 `.tb-center` 用 `position:absolute; inset:0` 绝对定位，保证相对**整个窗口**居中，不受左右内容宽度影响；容器 `pointer-events:none`，不挡拖拽。
- **Windows / Linux 布局**：`[图标] LanLink/副标题 [文件 设备 视图 帮助] … [最小化 最大化 关闭]`，菜单经 `.tb-menu`（`align-self:stretch` + `margin-left:12px`）紧接品牌块，不再居中放标题。

### 23.7 品牌块移到命令栏左侧（2026-11-15 修订）

标题栏里同时出现「左侧品牌块」和「居中窗口标题」，两行内说了两遍 LanLink，重复且挤压标题栏的留白。改为职责分离：

| 区域 | 职责 | 内容 |
|---|---|---|
| 标题栏 | 只表达「窗口」 | macOS：交通灯 + 居中标题「LanLink」；Windows/Linux：交通灯替身（自绘三键在右）+ 左侧菜单 |
| 命令栏 | 表达「产品 + 操作」 | **品牌块**（图标 + LanLink/副标题两行）│ 分隔线 │ 选择文件 / 视图相关动作 … 右侧在线状态 + 齿轮 |

实现：

- 新增 `components/layout/BrandBlock.vue`，把原来散在 `TitleBar.vue` 的 `.tb-logo/.tb-brand/.tb-name/.tb-sub` 收拢成一个组件（26px 图标 + 14px semibold 名称 + 11px tertiary 副标题，`gap:1px`、`line-height:1.15`）。
- `CommandBar.vue` 在 `.cb-left` 首位渲染 `<BrandBlock class="cb-brand" />`，其后加 `.cb-sep.cb-sep--tall`（1×24px 竖线，比默认 18px 高，用来分隔两行文字高度的品牌块与操作区）。
- `TitleBar.vue` 删除品牌块与其样式，`Icon` 仅剩窗口三键使用。
- 品牌块放在 `.cb-left` 内、三个视图的 `v-if` 之外，因此发送 / 接收 / 记录 view 下位置恒定。

> 注意：命令栏最小宽度限制。窗口 `minWidth: 900` 时「记录」视图内容最宽（3 按钮 + 232px 搜索框 + 右侧状态），品牌块约占 200px，靠 `.cb-search` 的默认可收缩兜住；若后续再加按钮，优先缩搜索框宽度而不是折行。

---

## 24. 底部能力卡固定 + 主窗体最小尺寸 + 窄窗表格列宽（2026-11-15）

### 24.1 左轨底部卡三视图统一贴底

原实现里发送页定版态用 `.cap-card--top { margin-top: 0 }` 让「发现能力」卡紧跟导航，接收/记录视图却是 `margin-top: auto` 贴底。切换视图时卡片在「紧跟导航」和「贴底」之间跳，视觉抖动。

改为**三视图一律 `margin-top: auto` 贴左轨底部**，删掉 `.cap-card--top` 修饰类。`.rail` 是 flex column，卡片自带 `margin-top:auto` 自动吃掉导航下方的剩余空间；窗口高度变化时它只会上下移动，不会跳档。

> 这是对 §15「定版发送页发现能力卡紧跟导航」的**覆盖**。原型里之所以紧跟导航，是因为定版画布 820px 高、卡片跟着导航更紧凑；但跨视图一致性更重要。

### 24.2 主窗体最小尺寸 1040×760

`minWidth / minHeight` 从 `900×620` 提到 **`1040×760`**（两个平台配置文件都要改）。

依据是横向固定开销：左轨 206px + 右栏 296px = 502px，中央区还需要约 500px 才能让「设备表 4 列 + 跨网凭证条单行说明」不挤；1040 − 502 = 538，刚好有余量。纵向 760 = 标题栏 42 + 命令栏 52 + 状态栏 26 + 中央区 640，比定版画布的 700 少 60，是压缩各区块后仍能完整显示「拖放区 + 文件表 + 设备表 + 凭证条 + 提示条」的下限。

**验证**：`tauri-runtime-wry` 在创建窗口时把 config 的 `min_width/min_height` 转成 `inner_size_constraints`（`lib.rs:877`），macOS 落到 `NSWindow.setMinSize`，用户拖拽边框时由窗口管理器钳制。

> **坑**：macOS 的 `NSWindow.setContentSize`（即 Tauri 的 `set_size`）**不应用 minSize**，所以「程序里 set_size 到 600×400 成功」不能用来判断约束是否失效。要验证最小尺寸，看合并后的运行时配置：
>
> ```bash
> LL_SIZE_PROBE=1 tauri dev     # 打印 config width/height/min
> LL_SIZE_PROBE=1 LL_MIN_SIZE=1 tauri dev   # 同时把窗口摆到最小尺寸便于截图验收
> ```

### 24.3 窄窗下的表格列宽（传输记录）

最小宽度下记录表的「文件 / 大小 / 耗时」会被压成两行（`2` / `个`、`866` / `MB`、`26` / `s`），行高参差。

根因是 `table-layout: auto`：各列 `th` 虽然写了 px 宽度，但总宽超出可用宽度时浏览器会按比例压缩每一列，文字自然折行；而且原来 8 列固定宽度合计 566px + 单元格内边距 128px ≈ 694px，**比 1180 默认宽度下的中央区（约 638px）还宽**，属于本来就溢出的设定。

修法：

```css
.history-table { table-layout: fixed; }          /* 列宽由表头一次定死 */
.history-table th:nth-child(1) { width: 94px; }  /* 时间 */
.history-table th:nth-child(2) { width: 50px; }  /* 方向 */
.history-table th:nth-child(3) { width: auto; }  /* 对端设备：唯一弹性列 */
.history-table th:nth-child(4) { width: 42px; }  /* 文件 */
.history-table th:nth-child(5) { width: 58px; }  /* 大小 */
.history-table th:nth-child(6) { width: 44px; }  /* 耗时 */
.history-table th:nth-child(7) { width: 54px; }  /* 状态 */
.history-table th:nth-child(8) { width: 40px; }  /* 操作 */
.history-table th { white-space: nowrap; }
.history-table td:nth-child(4),
.history-table td:nth-child(5),
.history-table td:nth-child(6),
.history-table td:nth-child(7),
.history-table td:nth-child(8) { white-space: nowrap; }
```

- 列宽按「最小窗口下内容单行」反推（`866 MB` 约 40px 文字 + 16px 内边距 → 58px），不是按比例拍的。
- 「对端设备」列设为 `auto` 吸收全部剩余空间，超长靠内部 `.ll-truncate` 省略号，所以窄窗下它变窄而不是把邻居挤到折行。
- `fixed` 布局是前提：`auto` 布局下 `th` 的 px 宽度只是「建议值」，压缩时依旧会折行。

### 24.4 验收

- **1040×760 三视图截图**（`LL_MIN_SIZE=1` 起，`LL_MENU_PROBE=view-send|view-recv|view-history` 切视图）：
  - 记录表 `2 个 / 866 MB / 26 s / E009` 全部单行，列边界无重叠、无裁切；
  - 发送页设备表 4 列与文件表 4 列单行，跨网凭证条说明与两个按钮同排不溢出；
  - 接收页队列行单行；
  - 左轨底部卡（发现能力 / 可接收状态 / 记录策略）均贴底，切换视图不跳。
- 窗口实际宽度实测 2108 物理像素 ≈ 1054 逻辑像素，确认跑的就是最小尺寸。

### 24.5 发送页表格也吃同一类 bug（2026-11-15 补充）

记录表修好后，发送页的文件表和设备表仍在最小宽度下折行（`4.2` / `MB`、`11-14` / `10:22`）。原因是同一类问题：`table-layout: auto` + 只有 `th:nth-child(1) { width: 46% }`，其余列没定宽，宽度不够时浏览器按比例压缩每一列，文字折行。

两张表都改成 `fixed` 布局、弹性只给第一列、窄列 `nowrap`：

```css
.file-table { table-layout: fixed; }
.file-table th:nth-child(1) { width: auto; }  /* 文件名：弹性，超长 .ll-truncate 省略 */
.file-table th:nth-child(2) { width: 64px; }  /* 大小 */
.file-table th:nth-child(3) { width: 94px; }  /* 修改时间 */
.file-table th:nth-child(4) { width: 72px; }  /* 状态 */
.file-table th:nth-child(5) { width: 44px; }  /* 移除 */
.file-table th { white-space: nowrap; }
.file-table td:nth-child(n+2) { white-space: nowrap; }

.device-table { table-layout: fixed; }
.device-table th:nth-child(1) { width: auto; }
.device-table th:nth-child(2) { width: 116px; }  /* 地址 */
.device-table th:nth-child(3) { width: 60px; }   /* 延迟 */
.device-table th:nth-child(4) { width: 84px; }   /* 操作 */
.device-table td:nth-child(2) { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
```

- 地址列额外给 `ellipsis`：出现长 IPv6 时它自己省略，而不是把「延迟 / 操作」挤到折行。
- 宽度按「最小窗口下内容单行」反推，注意发送页 `td` 内边距是 `6px 12px`（左右共 24px），比记录表的 16px 大，所以同名列需要给更宽。

**通用结论**：本项目所有 `.ll-table` 一律 `table-layout: fixed` + 明确列宽 + 窄列 `nowrap`，弹性只留一列。`auto` 布局下 `th` 的 px/百分比宽度只是建议值，压缩时必然折行。
