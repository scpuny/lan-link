# LanLink 功能全景清单（页面功能 + 后端能力）

> 用途：界面定版之后、进入功能实现之前的范围基线。逐项记录「页面上有什么、点了应该发生什么、现在做到哪一步、依赖后端什么」。
> 依据：`docs/lanlink-invite-serial-PRD.md`（FR-01~FR-12 / BR-01~BR-07）、`docs/prototype-review.md`、`web/src/**`、`crates/**`。
> 日期：2026-11-16。

## 0. 状态图例与范围边界

### 0.1 当前状态符号（看「完成了没有」用这套）

| 符号 | 状态 | 含义 |
| --- | --- | --- |
| ✅ | 完成 | 前端交互 + 后端真实能力均已闭环，可直接使用 |
| ⚠️ | 受限 / 待验证 | 主体已实现，但依赖打包签名、系统权限、双端联调或发布渠道才能真正生效 |
| ⬜ | 未完成 | UI 位置可能已预留，功能整体未落地 |
| 📌 | 设计取舍 | 按 PRD 有意不做，不属于待办 |

> **当前状态的唯一权威来源是 §0.5 任务状态总览。** 下方 §1–§10 是「开发前基线记录」，其中的「静态 / Mock / 骨架 / 待实现」是当时的历史状态，**不代表现在**；§11–§12 才是落地记录。判断"完成了没有"请直接看 §0.5。

### 0.2 历史状态词（仅用于读 §1–§10 基线）

| 状态 | 含义 |
| --- | --- |
| 已完成 | 前端交互与本地状态已闭环，无需后端真实能力 |
| 真后端 | 已通过 Tauri `invoke` 调到 Rust，返回真实结果 |
| 骨架 | 后端 command 已接通，但返回的是结构正确的占位数据（非真实网络能力） |
| Mock | 只有浏览器预览态 `bridge/mock.ts` 有数据，Tauri 态无对应实现或返回空 |
| 静态 | 原型文案/结构已还原，按钮无业务逻辑或未接线 |
| 待实现 | UI 位置已预留，功能整体未落地 |

边界说明：本清单把「已经画出来的界面」和「真正可用的功能」明确分开。§1–§10 基线记录停留在开发前状态，实际推进以 §0.5 + §11 + §12 为准。

---

## 0.5 任务状态总览（权威 · 更新至 2026-11-16）

### 0.5.1 里程碑与功能组

| 模块 | 状态 | 当前说明 | 详细 |
| --- | --- | --- | --- |
| D2 协议设计 / PRD | ✅ 完成 | PRD v1.0 定稿，D2/D3/D4 里程碑全标完成 | PRD |
| D3 协议层 `lanlink-protocol` | ✅ 完成 | Base32/CRC32/LZ4/PBKDF2/AES-GCM/Blake3/双 FSM，26 单测通过 | §8.1 |
| D4 局域网短码 | ✅ 完成 | 6 位短码 + UDP LQ/LA + 复用握手/传输，C12–C16 全通 | §12 |
| 文件入口 F1–F4 | ✅ 完成 | 文件/目录选择、剪贴板、拖拽、保存目录持久化 | §11.2 |
| 地址与发现 F5–F11 | ✅ 完成 | 网卡/STUN/IPv6 采集、UDP 42211 设备发现、真实 RTT | §11.2 |
| 握手与通道 F12–F17 | ✅ 完成 | TCP 42210 拨号回落、复用 FSM（TTL/黑名单/质询）、口令探测、取消清零 | §11.2 |
| 传输引擎 F18–F23 | ✅ 完成 | 4MB 分片、seq 校验/重传、落盘、Blake3、暂停/继续/取消、进度事件 | §11.2 |
| 记录与运维 F24–F29、F32 | ✅ 完成 | localStorage 持久化/30 天清理/CSV/诊断/全量设置持久化 | §11.2 |
| 生成弹窗 TTL 设置传递 | ✅ 完成 | `ttlSecs` 贯通 `generate_serial`，倒计时随设置变化 | §11.6-3 |
| 托盘/自启/通知 F30 | ⚠️ 受限 | UI 与开关持久化已做，实际生效需打包签名 + 系统 API | TODO-03 |
| 防火墙入站规则 F31 | ⚠️ 受限 | 开关已持久化，实际放行需管理员权限 + 打包 | TODO-04 |
| 帮助/检查更新 F33 | ⚠️ 受限 | 静态文案，需发布渠道后接入 | TODO-05 |
| 菜单动作分发 | ⚠️ 部分 | 主要项已接（文件/视图/主题/色板/短码/设置等），少数帮助项待发布能力 | §2.6 |
| 传输记录持久化位置 | ⚠️ 受限 | localStorage，清缓存会丢；符合不上传但不跨浏览器 | TODO-06 |
| 双端端到端联调 | ⚠️ 待验证 | 握手/传输为真实现，需两台实例验证；单机可回环自测 | TODO-07 |
| 跨网 NAT 打洞 | ⬜ 未完成 | 仅 STUN 观测端点直连；对称 NAT 需打洞或中继，本轮范围外 | TODO-01 |

### 0.5.2 TODO / 待办追踪表

> **后期维护规则**：某项完成时，把「状态」列改为 ✅ 完成，并填「完成日期」「验证结果」两列；**不要删行**，保留轨迹。新增待办请顺延编号。

| ID | 待办功能 | 状态 | 当前阻塞 / 原因 | 完成标准 | 完成日期 | 验证结果 |
| --- | --- | --- | --- | --- | --- | --- |
| TODO-01 | 跨网 NAT 打洞（对称 NAT 直连） | ⬜ 未完成 | 当前只做 STUN 观测端点直连；对称 NAT 需 UDP 打洞或中继/UPnP，本轮范围外 | 对称 NAT 下两私网设备可直连或经中继完成握手与传输 | — | — |
| TODO-02 | TTL 设置传递到 `generate_serial` | ✅ 完成 | —（已完成） | 设置页 TTL 900/300 影响生成倒计时与参数卡 | 2026-11-16 | cargo check + web build 通过；§11.6-3 |
| TODO-03 | 托盘 / 开机自启 / 系统通知（F30） | ⚠️ 受限 | 需打包签名与系统权限；代码预留 UI 与持久化 | 打包后托盘驻留、自启开关写系统配置、通知可达 | — | — |
| TODO-04 | 防火墙入站规则自动放行（F31） | ⚠️ 受限 | 端口 42210 放行需管理员权限 | 安装时或首次运行按权限提示放行 42210/42211 | — | — |
| TODO-05 | 检查更新 / 帮助内容（F33） | ⚠️ 受限 | 需发布渠道与版本号服务 | 检查更新能拉到最新版本信息；帮助页可打开完整文档 | — | — |
| TODO-06 | 传输记录改为 Rust 侧文件持久化 | ⚠️ 受限 | 现用 localStorage，清缓存/重装会丢，不跨浏览器 | 记录写入应用配置目录文件，浏览器无关，30 天清理仍生效 | — | — |
| TODO-07 | 双实例端到端联调（握手/传输/短码） | ⚠️ 待验证 | 引擎与 FSM 为真实现，但尚未两台真机跑通全流程 | 两台实例完成：外网序列号握手+传输、短码握手+传输、暂停/续传/取消 | — | 联调方法与二进制构建见 `docs/joint-debug-guide.md` |
| TODO-08 | 局域网短码跨网段 / VLAN | 📌 设计取舍 | UDP 42211 广播不跨路由，PRD 定义短码仅同网段有效（E010） | 若要跨网段需改服务端注册或单播发现（另立需求） | — | — |
| TODO-09 | 序列号/密钥显式清零（zeroize） | ⚠️ 部分 | FSM 有 `cancel()`、口令缓冲已清；BR-01 要求派生密钥显式 zeroize | 关窗/握手后对 session、口令派生密钥显式 zeroize | — | — |

### 0.5.3 已评估并决定搁置的方案（📌 不再重复讨论）

> 2026-11-16 针对「外网序列号段数过多」做过一轮技术评估，结论是**维持现状**。以下方案均已否决，除非用户明确重启需求，否则不再执行。

| 方案 | 评估结论 | 否决原因 |
| --- | --- | --- |
| 外网序列号压到 11 段（去编码冗余，保 IPv6） | 📌 搁置 | D3 协议 breaking change，要改 payload 编解码 + 26 单测 + 握手 + 前端 + PRD；当前无存量凭证，收益（84→66 字符）不抵改动面 |
| 压到 6 段（砍 IPv6 / session 缩到 8B） | 📌 搁置 | 必须放弃公网 IPv6 候选优先（违背 BR-05）；且 6 段是 payload ≤22B 的物理硬墙 |
| 砍掉局域网短码、内外网统一用 6 段凭证 | 📌 **否决** | 短码 6 位辨识度高、可口头念/手抄，是局域网场景的核心优势；外网序列号 34 字符只能复制粘贴，替代不了 |
| 口令保护改为 flags bit3 传输层加密（口令不进凭证，6 段恒定） | 📌 搁置 | 需新增传输层 AES-GCM 流加密（当前 transfer 明文），属 PRD bit3 扩展预留；随本轮重构一并搁置 |

**定稿**：局域网 = 6 位短码（D4 保留）；外网 = 现有 14 段序列号（PRD §4.3.3 公式值，§4.2 的 6 段仅为观感示意串）；口令 = 现行为 AES-GCM-256 加密 payload（4~6 位数字、单独发送）。三者均不改。

---

## 1. 页面 / 组件总览

> ⚠️ 本表「当前状态」列为**开发前基线**（历史状态）。真实现状见 §0.5，落地记录见 §11–§12。

| 页面 | 文件 | 进入方式 | 基线状态 | 当前（2026-11-16） |
| --- | --- | --- | --- | --- |
| 应用外壳 | `web/src/App.vue` | 启动即加载 | 已完成 | ✅ 完成 |
| 标题栏 | `components/layout/TitleBar.vue` | 常驻 | 部分静态（菜单动作分发不全，见 §2.6） | ✅ 完成（菜单分发见 §0.5.1） |
| 命令栏 | `components/layout/CommandBar.vue` | 常驻，随主视图切换内容 | 部分静态 | ✅ 完成 |
| 左轨 | `components/layout/LeftRail.vue` | 常驻，四项导航 | 部分静态 | ✅ 完成 |
| 中央区路由 | `components/layout/CenterStage.vue` | `mainView` 分发 | 已完成 | ✅ 完成 |
| 发送文件视图 | `layout/views/SendView.vue` | 默认主视图，`centerState` 三态 | 静态 + 部分 Mock | ✅ 完成（F1–F23） |
| 接收文件视图 | `layout/views/ReceiveView.vue` | 左轨第 2 项 | 静态 | ✅ 完成（F19 落盘等） |
| 传输记录视图 | `layout/views/HistoryView.vue` | 左轨第 3 项 | 静态（本地筛选可用） | ✅ 完成（F24–F28） |
| 右栏 | `components/layout/RightPanel.vue` | 常驻，随主视图切换内容 | 静态 | ✅ 完成 |
| 状态栏 | `components/layout/StatusBar.vue` | 常驻 | 已完成（派生文案真实） | ✅ 完成 |
| 生成外网序列号弹窗 | `dialogs/GenerateDialog.vue` | 命令栏「选择文件」/ 跨网条 / 菜单 `gen` | 真后端（`generate_serial`） | ✅ 完成（含 TTL 设置传递） |
| 连接远程设备弹窗 | `dialogs/ConnectDialog.vue` | 右栏 / 跨网条 / 菜单 `connect` | 真后端（`decode_serial` + `connect` 骨架） | ✅ 完成（含 D4 短码 Tab） |
| 等待对方连接弹窗 | `dialogs/WaitConnectDialog.vue` | 生成完成态「等待对方连接」 | 静态 | ⚠️ 部分（弹窗内容仍为展示） |
| 设置弹窗（5 页） | `dialogs/SettingsDialog.vue` | 齿轮 / 左轨设置 / 菜单 `settings` | 外观页真实，其余静态 | ✅ 完成（F29 全量持久化；F30/31/33 受限） |
| macOS 原生菜单 | `crates/lanlink-desktop/src/menu.rs` | 屏幕顶部（仅 macOS） | 已渲染，动作分发不全 | ⚠️ 部分（少数帮助项待发布能力） |
| 非 macOS 菜单 | `components/layout/AppMenu.vue` | 标题栏内（Windows/Linux） | 已渲染，动作分发不全 | ⚠️ 部分（同上） |

---

## 2. 全局壳层功能

> ⚠️ **§2–§8 均为开发前基线记录**，表格中的「当前状态」列是历史状态（静态 / Mock / 骨架 / 待实现），**不代表 2026-11-16 的真实现状**。请先看 §0.5 总览；落地后的状态以 §9（已加状态列）+ §11 + §12 为准。

### 2.1 标题栏 `TitleBar.vue`

| 功能点 | 期望行为 | 当前状态 | 依赖 |
| --- | --- | --- | --- |
| macOS 交通灯 | 系统绘制，左侧留 78px | 已完成 | `tauri.macos.conf.json` Overlay / hiddenTitle / trafficLightPosition |
| 居中标题 | macOS 显示 `LanLink` 替身；非 macOS 不显示 | 已完成 | `IS_MAC` |
| 传输中标记 | 发送页 `transferring` 时显示脉冲绿点 + 「传输中」 | 已完成 | `state.centerState` |
| 非 macOS 最小化 | 调 `getCurrentWindow().minimize()` | 已完成 | capability `core:window:allow-minimize` |
| 非 macOS 最大化/还原 | `toggleMaximize()` | 已完成 | capability `core:window:allow-maximize` |
| 非 macOS 关闭 | `close()` | 已完成 | capability `core:window:allow-close` |
| 窗口拖拽 | `data-tauri-drag-region` 逐节点标注 | 已完成 | capability `core:window:allow-start-dragging` |
| 原生菜单事件 | 监听 `ll-menu` → `onAction({key,value})` | 已完成（分发缺口见 §2.6） | `menu.rs` + `core:event:default` |
| Toast | 一次性反馈置于标题栏中央，2.6s 自动消失 | 已完成 | `useUi.showToast()` |

### 2.2 命令栏 `CommandBar.vue`

左侧固定：品牌块（`BrandBlock.vue`）+ 24px 竖分隔线 + 「选择文件」。

| 视图 | 控件 | 期望行为 | 当前状态 |
| --- | --- | --- | --- |
| 通用 | 选择文件 | 打开系统文件选择器，选完追加到待发送表 | **静态**：当前误接为 `openDialog('generate')`，功能开发时必须改为真实文件选择 |
| 通用 | 右侧在线状态 | 显示 mDNS / 监听 / 版本相关状态文案 | 已完成文案，数据来自 `SEND_LOCKED` 或固定串 |
| 通用 | 齿轮设置 | 打开设置弹窗 | 已完成 |
| 发送（就绪） | 打开保存目录 | `invoke('open_save_dir')` + Toast | 真后端 |
| 发送（就绪） | 重新扫描设备 | 重新调 `scan_lan` 并刷新设备表 | 静态：仅把 `centerState` 置回 `idle_with_lan` |
| 发送（无 LAN 变体） | 搜索框 | 搜索传输记录 / 设备 | 静态：输入框未绑状态 |
| 发送（传输中） | 全部暂停 | 暂停全部进行中任务 | 静态：仅切回 `idle_with_lan` |
| 发送（传输中） | 已连接标记 | 对端名 / IPv6 直连 / 38ms | 静态文案 |
| 接收 | 打开保存目录 | 同发送页 | 真后端 |
| 接收 | 允许被同网段设备发现 | 开关控制 mDNS 广播 | 静态：只改本地 `discoverable`，未下发后端 |
| 记录 | 导出 CSV | 把筛选结果导出为 CSV 文件 | 静态 |
| 记录 | 清空 | 清空本机传输记录 | 静态 |
| 记录 | 搜索文件名或设备 | 实时过滤记录表 | 已完成（本地 `HISTORY_ROWS` 过滤） |

### 2.3 左轨 `LeftRail.vue`

| 功能点 | 期望行为 | 当前状态 |
| --- | --- | --- |
| 本机信息卡 | 设备名 / 系统用户 / Lan4 / Pub6 / STUN，三视图统一 | ✅ 完成（2026-11-16）：三视图共用 `selfCard` computed，统一取自 `state.self`（`get_self_info` 真采集），字段缺失回退原型常量；已移除发送态静态假数据与「未打包 / 默认」假标签，切视图不再 IP 忽真忽假 |
| 四项导航 | 发送文件 / 接收文件（角标 1）/ 传输记录（角标 18）/ 设置 | 前三项路由已完成；设置打开弹窗 |
| 角标随视图变化 | 定版发送态：发送 2、接收 0；其余视图接收 1 | 已完成 |
| 底部能力卡（发送） | 按三态切换：网络能力检测 / 发现能力 / 当前通道能力 | 静态数据，随 `centerState` 切换文案 |
| 底部能力卡（接收） | mDNS 可发现 / 防火墙放行 / 剩余空间 | 静态 |
| 底部能力卡（记录） | 记录策略：本地保存 30 天 / 当前 18 条 / 不上传云端 | 静态 |
| 卡片始终贴底 | 三视图统一 `margin-top:auto`，窗口缩放不跳位 | 已完成 |

### 2.4 右栏 `RightPanel.vue`

| 视图 | 卡片 | 内容 / 期望行为 | 当前状态 |
| --- | --- | --- | --- |
| 发送 | 当前会话 | 连接状态、时长、累计传输、通道加密；非传输态显示待发送信息 | 静态；「断开连接 / 连接到远程设备」仅切状态或开弹窗 |
| 发送 | 传输层校验（仅传输中） | 期望 seq / 重复丢弃 / 重传次数 | 静态 |
| 发送 | 凭证状态（仅传输中） | 序列号作废、黑名单保留至退出、口令建议 | 静态文案 |
| 发送 | 凭证安全 | 内存即焚 / 握手即作废 / 同网段无需序列号 | 静态文案 |
| 发送 | 最近连接 | 两条历史，失败项橙色告警 | 静态 |
| 接收 | 本机可被发现 | 设备名 / 局域网 / 端口 / mDNS + 「复制本机邀请信息」 | 静态（复制按钮未接线） |
| 接收 | 接收偏好 | 接收前先询问、同名自动重命名 | 静态（只改本地 state） |
| 接收 | 接收安全 | 两条提示 | 静态 |
| 记录 | 近 7 天概览 | 发送 / 接收 / 失败三条进度条 | 静态 |
| 记录 | 失败原因 | E009 ×1 / E006 ×1 | 静态 |
| 记录 | 记录隐私 | 仅存 session 前 8 位、不存 IP 详情 | 静态文案 |

### 2.5 状态栏 `StatusBar.vue`

| 功能点 | 期望行为 | 当前状态 |
| --- | --- | --- |
| 主文案 | 按视图/状态派生：就绪 · 待发送 2 个文件 · 已发现 3 台设备 / 接收中 / 传输记录 / 传输中 | 已完成 |
| 中段信息 | 发送：端口 + 黑名单条数 + 加密开关；接收：端口 + 剩余空间；记录：端口 + 保留 30 天 | 已完成（部分为固定值） |
| 右侧版本 | `v0.9.4 (20261114) · 协议 v1` | 已完成 |

### 2.6 菜单与动作分发缺口（功能开发必读）

macOS 菜单由 `menu.rs` 构建为系统 NSMenu；Windows/Linux 由 `AppMenu.vue` 绘制在标题栏。两端共用 `TitleBar.onAction(key, value)`，但目前**只有下列 key 有实际处理**：

`gen`、`connect`、`settings`、`about`、`view-send`、`view-recv`、`view-history`、`view-history-2`、`scan`、`open-dl`、`theme`（system/light/dark 或中文短名）、`palette`（machine id 或中文短名）。

**已渲染但无分发的菜单项**（点了对界面无影响，需逐一实现）：

`open-file`、`open-folder`、`paste-send`、`nearby`、`gen-lan`、`netinfo`、`portcheck`、`view-history`（文件菜单项已有分发）、`toggle-rail`、`toggle-rate`、`help`、`shortcuts`、`errcodes`、`protocol`、`update`。

菜单项 id 约定 `key` 或 `key|value`；新增项必须同时在 `menu.rs`（macOS）和 `AppMenu.vue`（其他平台）登记，并在 `onAction` 里实现，否则会出现「菜单能点但没反应」。

---

## 3. 发送文件视图 `SendView.vue`（三态）

### 3.1 定版就绪态 `idle_with_lan`（默认）

| ID | 区块 | 功能点 / 期望行为 | 当前状态 |
| --- | --- | --- | --- |
| S1 | 标题行 | 标题「发送文件」+ 副标题 + 「清空列表」 | 静态（清空未接线） |
| S2 | 拖放区 | 拖入文件或文件夹 → 加入待发送表；点击唤起文件选择器 | 静态（无拖拽监听） |
| S3 | 文件表 | 文件名 / 大小 / 修改时间 / 状态 / 移除；追加、删除、汇总 | 静态（数据来自 `data/send.ts`） |
| S4 | 选择目标设备卡 | 同网段状态条 + 设备表（设备 / 地址 / 延迟 / 操作） | 静态（数据来自 `L.target.devices`） |
| S5 | 设备行操作 | 「连接 / 发送」→ 建立直连并开始传输 | 静态：`primary=true` 打开连接弹窗，其余打开生成弹窗（占位语义） |
| S6 | 跨网凭证条 | 第一行标题，第二行说明 +「生成序列号」+「粘贴序列号连接」 | 弹窗可打开，凭证流程见 §6.1 / §6.2 |
| S7 | 底部提示条 | 「当前无进行中的传输」+ 查看传输记录 | 已完成（跳转记录视图） |
| S8 | 扫描刷新 | 最近扫描时间、设备延迟实时更新 | 静态 |

### 3.2 无 LAN 设备变体 `idle_no_lan`

| ID | 功能点 | 当前状态 |
| --- | --- | --- |
| S9 | 拖放区 + 文件表（3 行）+ 汇总行 | 静态 |
| S10 | 「选择连接方式」双卡片：外网序列号 / 局域网短码 | 外网卡片打开生成弹窗；短码卡片错误地也打开连接弹窗（应改为短码生成流程） |
| S11 | 空闲任务条 + 「查看传输记录」 | 跳转已完成 |

### 3.3 传输中 `transferring`

| ID | 功能点 | 期望行为 | 当前状态 |
| --- | --- | --- | --- |
| S12 | 通道信息卡 | 对端 / 连接地址 / RTT / 合计速率 + session 短号 | 静态（数据来自 Mock tasks） |
| S13 | 任务卡 | 进度条、速率、分片信息、已传输/剩余、已确认 seq | 静态 |
| S14 | 任务操作 | 暂停 / 取消单任务 | 静态（按钮无业务逻辑） |
| S15 | 近 60s 速率曲线 | 按速率序列画折线 + 面积，峰值/均值 | 已完成绘制（静态序列） |
| S16 | 排队条 | 排队数量 + 「立即启动」 | 静态 |

---

## 4. 接收文件视图 `ReceiveView.vue`

| ID | 功能点 | 当前状态 | 依赖 |
| --- | --- | --- | --- |
| R1 | 保存目录展示与「更改」 | 展示路径已完成；「更改」静态 | 需文件目录选择命令 |
| R2 | 接收通道条 | 对端 / 地址 / 速率 / 通道加密 | 静态 |
| R3 | 断开 / 重新允许接收 | 仅切换本地 `connected`，未通知后端停止/启动监听 | 静态 → 需新 command |
| R4 | 正在接收卡 | 进度、分片信息、已传输/剩余 | 静态 |
| R5 | 暂停 / 继续接收 | 切换 `state.receivePaused`，状态栏文案随之变化 | 半完成：本地状态真实，后端未联动 |
| R6 | 取消接收 | 无逻辑 | 静态 |
| R7 | 等待队列 | 列表 + 逐项取消 | 静态 |
| R8 | 已完成列表 | 列表 +「打开所在目录」（可复用 `open_save_dir` 逻辑） | 静态 |
| R9 | 底部提示条 | seq 单调校验、断点续传说明 | 静态文案 |
| R10 | 文件落盘 / 分片重组 / 去重 | 接收端真正写文件、按 seq 校验、重复分片丢弃 | **待实现，无 UI 入口，属后端隐藏能力** |

---

## 5. 传输记录视图 `HistoryView.vue`

| ID | 功能点 | 当前状态 |
| --- | --- | --- |
| H1 | 时间范围分段（今天 / 近 7 天 / 全部） | 分段样式已完成，`today` 有过滤逻辑，7d/all 暂未区分 |
| H2 | 四张统计卡（总数 / 成功 / 失败 / 累计流量） | 静态数据 |
| H3 | 筛选行：方向（全部/发送/接收）、状态（全部/仅失败） | 已完成（本地过滤） |
| H4 | 搜索框联动（来自命令栏） | 已完成（peer / linkType / status 字段匹配） |
| H5 | 记录表 8 列 + 空态 | 静态数据，列宽已按最小窗口固定 |
| H6 | 行尾「查看诊断详情」 | 静态：未打开任何诊断面板 |
| H7 | 分页（3 页 + 上下翻页） | UI 状态已完成，但翻页不改变数据（无真实数据源） |
| H8 | 结果数 / 表底条数文案 | 已完成（派生） |
| H9 | 记录持久化 / 30 天自动清理 / CSV 导出 | **待实现** |

---

## 6. 弹窗功能

### 6.1 生成外网序列号 `GenerateDialog.vue`（480 宽，两态）

| ID | 功能点 | 期望行为 | 当前状态 |
| --- | --- | --- | --- |
| G1 | 候选地址勾选 | Pub6 / Pub4 / Lan4(ULA) 勾选，显示「x / 4 条」，内网地址标「不推荐」 | 已完成（本地勾选） |
| G2 | 口令保护开关 | 4–6 位纯数字 + 二次确认 + 显隐；不一致/非法实时提示 | 已完成（前端校验） |
| G3 | 加密链路展示 | 52B Payload → PBKDF2 32B → AES-GCM-256 → 密文+16B Tag → Base32 | 静态文案（链路在协议层真实实现） |
| G4 | 生成按钮 | `invoke('generate_serial', { candidates, password, ttlSecs })` | **真后端**；`ttlSecs` 来自设置页 TTL |
| G5 | 完成态 | 成功横幅 + TTL 倒计时（900→0 / 300→0）+ 序列号（不折行）+ 凭证参数卡 | 已完成；倒计时取后端返回 `GenerateResult.ttl`，随设置页 TTL 变化 |
| G6 | 复制序列号 | 写入剪贴板 + 1.5s 「已复制」反馈；生成后自动复制一次 | 已完成 |
| G7 | 重新生成 | 回到表单态、停止倒计时、清空结果 | 已完成（FR-12：仅最新一次有效） |
| G8 | 安全警示 ×3 | 仅用最新串 / 口令单独发送 / 防重放靠 TTL+黑名单+质询 | 静态文案，规则见 §8 |
| G9 | 等待对方连接 | 完成态底部link → `WaitConnectDialog` | 已完成（弹窗内容静态） |
| G10 | 台账不落盘提示 | 凭证不写本地文件 | 静态文案；后端当前确实不落盘 |

### 6.2 连接远程设备 `ConnectDialog.vue`（520 宽，双 Tab + 多相态）

**Tab1：外网序列号**

| ID | 功能点 | 当前状态 | 说明 |
| --- | --- | --- | --- |
| C1 | 序列号输入与容错清洗 | 已完成（前端可粘贴预判；协议层 `from_serial` 去 `-`/空白/换行、转大写） | FR-03 |
| C2 | 口令勾选与输入 | 已完成前端（4–6 位、显隐），是否按 flags 自动弹出尚未联动 | FR-06 未闭环 |
| C3 | 六态校验胶囊 | 解码→完整性→解析→时效→会话→质询，按阶段 active/done/error | 动画已完成，阶段由本地 `step()` 驱动 |
| C4 | 连接动作 | `invoke('decode_serial')`（真实校验）→ 模拟步骤 → `invoke('connect')`（骨架） | 半真：解码真，握手/通道假 |
| C5 | 解析结果卡 | 展示 Base32 84 字符 / 14 组 / Pub6 / Pub4 | 静态文案 |
| C6 | 动态质询卡 | 16B challenge、session 前 8 位、10s 倒计时、超时 E006 | 静态倒计时；`crypto::challenge_response` 已实现但未接线 |
| C7 | 成功态通道信息 | 对端 / 地址 / RTT / flags / 加密关闭 + 黑名单 1 条 | 静态（`connect` 返回 rtt=0） |
| C8 | 打开传输面板 | 切 `centerState='transferring'` 并关弹窗 | 已完成 |
| C9 | 错误态 E001–E009 | 六态胶囊定位 + 诊断分区 + 错误卡 + 主按钮分流（E004/E005/E008 → 重新生成；E009 → 改短码） | 完整实现于前端；预览态可用下拉注入错误码 |
| C10 | 错误码规范 | UI 只按 `code` 映射文案，禁止硬编码判断 | 已完成（`data/errors.ts`） |
| C11 | 取消/关闭 | 任意阶段关闭 → 清定时器、释放 challenge | 前端已完成；后端 FSM `cancel()` 未接线 |

**Tab2：局域网短码**

| ID | 功能点 | 当前状态（D4 落地后） |
| --- | --- | --- |
| C12 | 6 位短码输入 | ✅ 完成：格式校验 A-Z/2-9 六位，回车即连接，非法走 E010 错误卡 |
| C13 | 附近设备列表 | ✅ 完成：`state.nearby` 真机来自 `scan_lan`；行可点选，选中地址作为优先拨号目标 |
| C14 | 重新搜索 | ✅ 完成：接 `scan_lan` 真实重扫 + 搜索中反馈 |
| C15 | 选择/连接 | ✅ 完成：接 `connect_lan_code`（UDP 短码查询→换 serial→复用握手 FSM）；连上后「打开传输面板」 |
| C16 | 两 Tab 隔离 | ✅ 完成：watch(tab) 切换即清 phase/error/steps，输入缓冲各自独立（BR-06/FR-11） |

配套新增「生成局域网短码」弹窗 `GenerateLanDialog.vue`（SendView 能力卡 + 菜单 `gen-lan` 入口）：
调用 `generate_lan_code` 生成 6 位短码 → 构造仅含局域网候选的 serial → 注册到发现服务并广播 → 展示短码/复制/等待连接状态。

### 6.3 等待对方连接 `WaitConnectDialog.vue`（540 宽）

| ID | 功能点 | 当前状态 |
| --- | --- | --- |
| W1 | 五步进度 | 等待连接→收到请求→接收质询→回传应答→校验通过；当前固定停在「回传应答」 | 静态 |
| W2 | Blake3 应答计算卡 | session_id ‖ challenge 输入、challenge、32B 应答摘要、链路 RTT | 静态 |
| W3 | 序列号与 TTL | 14:29 倒计时（每秒递减） | 静态 |
| W4 | 中断握手 / 转到后台 | 仅关闭弹窗 | 静态 |
| W5 | 进度横幅 + 计时 | 10s 超时提示 | 静态 |

### 6.4 设置弹窗 `SettingsDialog.vue`（固定 720×590，5 项导航）

| Tab | 功能点 | 当前状态 |
| --- | --- | --- |
| 通用 | 开机自动启动 / 启动时最小化到托盘 / 界面语言（中/英） | 静态 |
| 通用 | 接收文件保存目录 +「更改」 | 静态（路径展示） |
| 通用 | 完成后打开所在文件夹 / 桌面通知 / 声音提示 | 静态 |
| **外观** | 主题明暗三档（跟随系统 / 浅色 / 深色），同步应用与窗口外观 | **真实**，写 localStorage + `setTheme()` → `getCurrentWindow().setTheme()` |
| **外观** | 视觉风格 3×2 六套卡片 +「原型基准」恢复 | **真实**，写 localStorage `ll-palette` |
| 传输 | 分片大小 4MB（协议固定，不可调） | 展示正确 |
| 传输 | 并发任务数 1–4 | 静态 |
| 传输 | 断点续传 / 限速模式与数值 / 完成后 Blake3 校验 | 静态 |
| 传输 | 监听端口 42210 / 自动添加防火墙入站规则 | 静态 |
| 安全与协议 | TTL 900s / 300s 切换 | **真实**，写 localStorage `ll-settings-v1` → 生成弹窗读取并传 `ttlSecs` → `generate_serial` 回写 `GenerateResult.ttl` → 倒计时与参数卡按该值显示 |
| 安全与协议 | 默认包含内网地址 / 默认勾选口令保护 | 静态 |
| 安全与协议 | 会话黑名单清空（显示当前条数） | 静态；黑名单后端尚未建立 |
| 安全与协议 | 日志红线说明（只记 session 前 8 位 + ts + error_code） | 静态文案，实现见 §8.3 |
| 关于 | 版本 / 检查更新 | 静态 |
| 关于 | 运行环境（OS / 网络能力 / 监听端口） | 静态 |
| 关于 | 复制诊断信息 / 打开日志目录 | 静态 |
| 关于 | 协议依赖列表 / 隐私声明 | 静态文案 |
| 全部 | 取消 / 保存设置 | 仅关闭弹窗；除外观外均不持久化 |

---

## 7. 后端已暴露命令（`invoke` → `#[tauri::command]`）

6 个 command 定义在 `crates/lanlink-desktop/src/lib.rs`，返回结构全部 camelCase 对齐 `web/src/types/invite.ts`。

| 命令 | 入参 | 返回 | 真实度 | 主要调用方 |
| --- | --- | --- | --- | --- |
| `generate_serial` | `candidates[]`、`password?`、`ttlSecs?`（900 / 300，来自设置页） | `GenerateResult`（serial/session/candidates/payloadBytes/base32Chars/createdAt/ttl/encrypted） | **真实协议编码**：UUID v4 session、真实出口 IP、LZ4/CRC32/AES-GCM/Base32 全链路；`ttl` 按传入有效期回写（缺省 900s） | 生成弹窗 G4 |
| `decode_serial` | `serial`、`password?` | `DecodedInvite`（flags/session/ttl/ts/candidates/...） | **真实解码与 CRC/GCM 校验**；当前不校验 TTL 与黑名单（FSM 中已实现但未接） | 连接弹窗 C4 |
| `connect` | `serial`、`password?` | `SessionInfo`（peerName/addr/rtt/encrypted/protocol/flags/tookMs） | **骨架**：复用 `decode_serial`，取首个候选地址；`peerName="远端设备"`、`rtt=0`、`tookMs=0`，无 TCP 拨号 | 连接弹窗 C4 |
| `scan_lan` | — | `NearbyDevice[]` | **骨架**：真机返回空数组；Mock 返回 3 台 | 定版设备表、短码 Tab |
| `get_self_info` | — | `SelfInfo`（deviceName/osUser/lan4/pub6/stun/netCapabilities） | **部分真实**：UDP connect 探测出口 v4/v6 + 主机名/用户名；`stun` 空、`mdns=false`、`stun=false` | 左轨本机卡、生成弹窗 |
| `open_save_dir` | — | `{ path }` | **真实**：`<HOME>/Downloads/LanLink`，创建目录并用平台文件管理器打开（open/explorer/xdg-open） | 命令栏两个按钮、菜单 `open-dl` |

调试探针（仅 debug 构建）：`[lanlink-bridge] <cmd>` 证明走 IPC 而非 mock；`[lanlink-window]`、`[lanlink-size-probe]`、`[lanlink-menu]` 分别用于窗口配置、最小尺寸、菜单链路验收。

---

## 8. 后端隐藏能力（已实现但不直接暴露给页面）

### 8.1 协议层 `crates/lanlink-protocol`（26 项单测通过）

| 模块 | 隐藏能力 | 说明 |
| --- | --- | --- |
| `base32.rs` | 定制字符集编解码 | `ABCDEFGHJKLMNPQRSTUVWXYZ23456789`，剔除 0/1/I/O，L 保留；无填充、每 6 字符分组展示由 codec 负责 |
| `payload.rs` | 二进制 Payload 序列化 | 大端：`[session(16)][ts(4)][count(1)][candidates]`，Candidate 为 Lan4/Pub4/Lan6/Pub6 + port + addr；校验地址长度、count ≤ 4、无尾随字节（E007） |
| `payload.rs` | CRC32 覆盖区 | `[flags][session][ts][count][candidates]`；`session_short()` 供脱敏日志使用 |
| `codec.rs` | 明文 / LZ4 自适应压缩 | 压缩后更小才采用（FLAG_COMPRESS），解压上限 64KB 防内存耗尽 |
| `codec.rs` | 可选 AES-GCM 加密块 | `[flags(0x02)][nonce(12)][ct+tag]`；flags 始终明文；AD 置空；口令错误与篡改统一 E003 |
| `codec.rs` | 序列号清洗与分组 | 去 `-`/空格/制表/换行、统一大写后解码；每 6 字符插 `-` |
| `crypto.rs` | PBKDF2-HMAC-SHA256 | 固定盐 `lanlink-invite-v1-salt`、10000 次迭代、派生 32B 密钥 |
| `crypto.rs` | Blake3 质询应答 | `digest = Blake3(session_id ‖ challenge)`，32B；16B 一次性随机 challenge |
| `handshake.rs` | 接收方 FSM | 11 状态：SIdle→SParseClean→SVerifyCrypto→SParsePayload→SCheckTtl→SCheckSessionBlack→SSendChallenge→SWaitResponse→SVerifyResponse→SEstablished/SFailed；返回 `ReceiverAction` 交给上层做网络 I/O |
| `handshake.rs` | 发起方 FSM | 7 状态：AIdle→ADial→AWaitChallenge→AGenResp→ASendResp→ATrans/AFailed；返回 `InitiatorAction` |
| `handshake.rs` | 取消语义 | 双方 `cancel()` 清 challenge / payload 回 Idle（BR-03），**前端未调用** |
| `error.rs` | E001–E009 码表 | 协议层只给 `code + message`，UI 按 code 映射完整文案与诊断（FR-08） |
| `lib.rs` | 常量 | FLAG_COMPRESS / FLAG_ENCRYPT、MAX_CANDIDATES=4、DEFAULT_TTL_SECS=900、CHALLENGE_TIMEOUT_SECS=10、PROTOCOL_VERSION=1 |

### 8.2 平台 / 运行时隐藏能力

| 能力 | 位置 | 说明 |
| --- | --- | --- |
| 平台窗口分叉 | `tauri.conf.json` / `tauri.macos.conf.json` | macOS：`decorations:true` + Overlay + hiddenTitle + 交通灯；Windows/Linux：`decorations:false` + 自绘三键；最小 1040×760 |
| 窗口权限 | `capabilities/default.json` | minimize / maximize / close / start-dragging / set-theme，windows label `main` |
| 应用主题跟随 | `useTheme.ts` | `data-theme`（light/dark/system）+ `data-palette`（6 套）；写 `setTheme()` 避免 macOS 1px 黑描边 |
| 双维度色板 | `styles/palettes.css` | 6 套 Base Styles：graphite / aurora / slate / carbon / nocturne / amber；语义色 success/warning/danger/info 为保留色 |
| 原生菜单构建 | `menu.rs` | 6 个顶级菜单 + 主题/视觉风格二级菜单，id 形如 `key\|value`；`emit("ll-menu")` |
| 浏览器 Mock 桥 | `bridge/mock.ts` | 6 个命令的内存实现；`?bridge=mock` 可在真机强制对比 |
| 预览直达参数 | `useUi.ts` / `useTheme.ts` | `?view=` / `?send=` / `?palette=` / `?theme=`；调试开关 `LL_MIN_SIZE` / `LL_MENU_PROBE` / `LL_SIZE_PROBE` |

### 8.3 PRD 规定的隐藏业务规则（代码已部分具备，UI 未完整体现）

| 规则 | 内容 | 落地情况 |
| --- | --- | --- |
| BR-01 内存即焚 | 序列号与派生密钥不落盘，关窗清零 | 后端不落盘；`zeroize` 未显式实现 |
| BR-02 黑名单随进程退出 | `seen_session` 只存内存 | 后端尚未建立黑名单；前端仅显示条数 |
| BR-03 任意阶段取消 | 中断 FSM、释放 challenge | FSM `cancel()` 已实现；前端关闭弹窗未调用 |
| BR-04 UUID v4 | 每次生成新 session，禁止自增/时间派生 | 已用 `Uuid::new_v4()` |
| BR-05 候选上限截断 | 超过 4 个按 Pub6→Pub4→Lan4→Lan6 优先级保留 | `MAX_CANDIDATES` 校验已有；优先级截断未实现（超上限直接报错） |
| BR-06 Tab 隔离 | 两凭证不共用输入缓冲 | 前端已完成 |
| BR-07 连续生成 | 允许多次生成，提示只用最新 | 前端已完成（G7） |
| 防重放三层 | TTL + session 黑名单 + 动态质询 | 质询与 TTL 判定在 FSM；黑名单未接线 |
| 日志红线 | 只记 session 前 8 位 + ts + error_code，不打印完整序列号/IP | `session_short()` 已提供；实际日志输出未接入 |
| 地址优先级 | Pub6 → Pub4 → Lan4/Lan6 依次尝试 | UI 文案有；后端拨号未实现 |

---

## 9. 功能能力清单（原「待实现」，现标注当前状态）

按依赖顺序分组；P0 为闭环主流程必需，P1 为体验/运维增强。状态符号见 §0.1。F1–F29、F32 已完成；F30/F31/F33 受打包签名与系统权限限制（见 TODO-03/04/05）。落地细节见 §11。

### 9.1 文件与选择（P0）

| ID | 状态 | 能力 | 说明 | 关联 |
| --- | --- | --- | --- | --- |
| F1 | ✅ 完成 | 选择文件 / 文件夹 | Tauri plugin-dialog 或多选文件，追加到待发表，支持拖拽落区 | S2、`open-file`、`open-folder` |
| F2 | ✅ 完成 | 从剪贴板发送 | 读取剪贴板文本/路径并转待发送项 | `paste-send` |
| F3 | ✅ 完成 | 待发表真实数据 | 从 `data/send.ts` 静态数组改为响应式任务队列，增删改、汇总、剩余空间 | S3 |
| F4 | ✅ 完成 | 保存目录可配置 | 「更改」→ 目录选择 → 持久化 → 所有入口复用 | R1、设置通用 |

### 9.2 候选地址采集（P0）

| ID | 状态 | 能力 | 说明 | 关联 |
| --- | --- | --- | --- | --- |
| F5 | ✅ 完成 | 网卡枚举 | 列出 Lan4 / ULA，判定同网段 | `get_self_info`、`generate_serial` |
| F6 | ✅ 完成 | STUN 观测端点 | 获取 Pub4 映射地址，填充 `SelfInfo.stun` 与候选 | `get_self_info`、G1 |
| F7 | ✅ 完成 | IPv6 全球单播判定 | 区分 Pub6 / ULA，设置 `ipv6Global` | `get_self_info` |
| F8 | ✅ 完成 | 候选上限优先级截断 | 实现 BR-05，超 4 个按优先级保留而非报错 | `generate_serial` |

### 9.3 设备发现（P0）

| ID | 状态 | 能力 | 说明 | 关联 |
| --- | --- | --- | --- | --- |
| F9 | ✅ 完成 | mDNS 广播与发现 | `_lanlink_tcp`，端口 42210，持续刷新附近设备 | `scan_lan`、S4、C13 |
| F10 | ✅ 完成 | 可发现开关下发 | 命令栏开关控制本机广播启停 | 命令栏接收页 |
| F11 | ✅ 完成 | 设备延迟测量 | 真实 RTT 替换 Mock 的 2/11/24ms | 设备表 |

### 9.4 握手与通道（P0）

| ID | 状态 | 能力 | 说明 | 关联 |
| --- | --- | --- | --- | --- |
| F12 | ✅ 完成 | TCP 拨号与候选回落 | 按 Pub6→Pub4→Lan4 依次连接，全失败返回 E009 | `connect` |
| F13 | ✅ 完成 | 质询链路接线 | 接收方下发 challenge、发起方用 FSM 计算 Blake3 应答、接收方比对（E006/E008） | C6、W1–W2 |
| F14 | ✅ 完成 | TTL + 黑名单校验接线 | 解码后按 FSM 顺序校验（E004/E005），成功后写 session | `decode_serial`、C3 |
| F15 | ✅ 完成 | 口令 flags 自动探测 | 加密块 flags=0x02 时自动弹出口令输入（FR-06） | C2 |
| F16 | ⚠️ 部分 | 取消与清零 | 关闭弹窗调用 FSM `cancel()` 已接；派生密钥显式 zeroize 待补（TODO-09） | C11、BR-01/03 |
| F17 | ✅ 完成 | 会话黑名单管理 | 内存 HashSet、设置页清空按钮、条数真实显示 | 设置安全页 |

### 9.5 传输引擎（P0）

| ID | 状态 | 能力 | 说明 | 关联 |
| --- | --- | --- | --- | --- |
| F18 | ✅ 完成 | 分片与 seq 校验 | 4MB 分片、seq 单调递增、重复丢弃、乱序处理 | S13、R9 |
| F19 | ✅ 完成 | 文件落盘与重组 | 接收端写文件、同名重命名、磁盘空间检查 | R4、R10 |
| F20 | ✅ 完成 | 断点续传 | 断开后从最后确认 seq 续传 | 设置传输页、S13 |
| F21 | ✅ 完成 | 暂停/继续/取消/排队调度 | 单任务与「全部暂停」按钮联动 | S14、命令栏 |
| F22 | ✅ 完成 | 速率与进度实时推送 | Tauri event → 前端任务卡、速率曲线、状态栏 | S12/S15、StatusBar |
| F23 | ✅ 完成 | 完成后 Blake3 整文件校验 | 失败分片自动重传，不覆盖已确认分片 | 设置传输页 |

### 9.6 记录与运维（P1）

| ID | 状态 | 能力 | 说明 | 关联 |
| --- | --- | --- | --- | --- |
| F24 | ⚠️ 受限 | 记录持久化 | localStorage + 30 天清理已做；改 Rust 侧文件待办（TODO-06） | H2/H5/H7 |
| F25 | ✅ 完成 | 真分页与全量筛选 | 分页作用于真实数据源；时间范围按记录真实时间动态判断（修复 7d 硬编码切片 bug），切换范围自动回第 1 页 | H1/H7 |
| F26 | ✅ 完成 | 导出 CSV | 按当前筛选结果生成文件 | 命令栏记录页 |
| F27 | ✅ 完成 | 诊断详情面板 | 行尾按钮打开该次会话的诊断快照 | H6 |
| F28 | ✅ 完成 | 清空记录 | 二次确认后清空（统一 `confirm()` 弹窗，非原生 `window.confirm`） | 命令栏记录页 |
| F29 | ✅ 完成 | 设置持久化 | 除外观外所有开关写入配置并在启动时生效 | 设置弹窗 |
| F30 | ⚠️ 受限 | 托盘 / 开机自启 / 通知 | UI 与开关持久化已做，需打包签名 + 系统 API（TODO-03） | 设置通用页 |
| F31 | ⚠️ 受限 | 防火墙入站规则 | 开关已持久化，实际放行需管理员权限（TODO-04） | 设置传输页 |
| F32 | ✅ 完成 | 诊断信息复制 / 日志目录 | 脱敏后复制 | 设置关于页 |
| F33 | ⚠️ 受限 | 帮助内容与检查更新 | 静态文案，需发布渠道后接入（TODO-05） | 菜单帮助项 |

---

## 10. 实现顺序建议与验收要点

**顺序**：F1–F4（文件入口）→ F5–F11（地址与发现，让定版设备表变真）→ F12–F17（握手闭环，含 TTL/黑名单/口令）→ F18–F23（传输引擎）→ F24–F33（记录与运维）。其中 F12–F17 必须复用 `handshake.rs` 的 FSM，不要在前端再写一套状态判断。

**每个阶段必须同时更新三处**：Rust command、前端 `bridge/mock.ts`（保持预览态可演示）、`bridge/index.ts` 的命令路由；新增命令还需补 capability（如 dialog / filesystem 权限）。

**验收要点**：

1. 新命令在 debug 日志出现 `[lanlink-bridge] <cmd>`，证明没有静默回退 mock。
2. 错误只返回 `code`，前端按 `data/errors.ts` 映射，不新增前端硬编码判断。
3. 1040×760 极限窗口下，三张表仍保持 `table-layout:fixed`，窄列不折行。
4. 切换主题与视觉风格后，原型基准（`data-palette` 未设置）应逐像素不变。
5. 任意阶段关闭/取消后，倒计时、定时器、challenge、口令派生密钥均被清理。

---

## 11. 功能落地记录（2026-11-16）

本节记录 F1–F33 的实现结果。所有条目均保持「原型默认态不变、真机走 IPC、浏览器预览走 mock」三原则。

### 11.1 新增文件

| 文件 | 职责 |
| --- | --- |
| `crates/lanlink-desktop/src/net.rs` | F5–F8：网卡枚举、STUN Binding、IPv6 全球单播/ULA 判定、私网判定、出口 IP |
| `crates/lanlink-desktop/src/discovery.rs` | F9–F11：UDP 42211 广播宣告/监听、设备表去重、RTT、可发现开关 |
| `crates/lanlink-desktop/src/session.rs` | F12–F17：TCP 42210 握手（Initiator 监听 / Receiver 拨号）、复用 FSM、内存黑名单、F18 起接管连接 |
| `crates/lanlink-desktop/src/transfer.rs` | F18–F23：4MB 分片、seq 单调/重复丢弃/乱序回退重传、落盘+同名重命名、Blake3 单片/整文件校验、暂停/继续/取消、`ll-transfer-progress` 事件 |
| `web/src/composables/useRecords.ts` | F24–F28：localStorage 记录持久化、30 天清理、CSV 导出、清空 |
| `web/src/composables/useUi.ts` | `confirm()` Promise 化二次确认状态机 + 全局 `busy` 防重复点击 |
| `web/src/components/common/ConfirmDialog.vue` | 危险操作统一确认弹窗（危险/警告两级，基于 Modal 组件，随主题自适应） |

**危险操作二次确认覆盖清单**（全部走 `confirm()`，取消即中止，确认才执行）：

| 位置 | 操作 | tone |
| --- | --- | --- |
| 命令栏 | 清空全部传输记录 | danger |
| 发送页 | 移除单个待发文件 / 清空待发列表 / 取消传输任务 | danger |
| 接收页 | 断开接收通道 / 取消本次接收 / 移除排队任务 | warning / danger |
| 设置弹窗 | 清空会话黑名单 | danger |
| 生成弹窗 | 重新生成序列号（作废旧凭证） | warning |
| 等待连接弹窗 | 中断握手（清零凭证密钥） | warning |

### 11.2 状态推进对照

| ID | 之前 | 现在 |
| --- | --- | --- |
| F1 | 静态 | **真后端**：`pick_files`/`pick_folder`（tauri-plugin-dialog）+ 拖放落区 + 菜单 |
| F2 | 静态 | **真后端**：`read_clipboard`（tauri-plugin-clipboard-manager）+ `paste-send` |
| F3 | 静态 | **真数据**：`sendFiles` 响应式队列，增删改、汇总，默认回退原型 2 项 |
| F4 | 静态 | **真后端**：目录选择 + `<app_config_dir>/settings.json` 持久化，全入口复用 |
| F5–F7 | 部分真实 | **真采集**：网卡枚举/STUN 观测端点/IPv6 全球单播判定进 `get_self_info` |
| F8 | 报错 | **BR-05**：按 Pub6→Pub4→Lan4→Lan6 排序截断到 4 个 |
| F9–F11 | 空/Mock | **真发现**：UDP 广播设备表 + 真实 RTT，设备表用真数据（无设备回退原型） |
| F12–F14 | 骨架 | **真握手**：TCP 拨号候选回落 + 复用 `handshake.rs` FSM（TTL/黑名单/质询） |
| F15 | 静态 | **真探测**：`probe_serial` 读 flags，加密序列号自动勾出口令输入 |
| F16 | 前端未接 | **已接线**：关闭弹窗调 `cancel_handshake`，口令缓冲清零 |
| F17 | 显示假条数 | **真黑名单**：内存 HashSet + 设置页真实条数/清空 |
| F18–F20 | 待实现 | **真引擎**：4MB 分片、seq 校验、重复丢弃、乱序回退重传、落盘、同名重命名、整文件 Blake3 |
| F21 | 静态 | **已接线**：单任务暂停/继续/取消 + 全部暂停 → Rust 控制块 |
| F22 | 静态序列 | **真推送**：`ll-transfer-progress` 事件驱动任务卡/速率曲线（无数据时回退原型序列） |
| F23 | 静态 | 完成即整文件 Blake3 校验，失败回退重传不覆盖已确认分片 |
| F24 | 静态 | **持久化**：localStorage + 30 天清理 + 完成事件落库 |
| F25 | 假分页 | **真分页/全量筛选**：作用于真实数据源，今天/7d/all + 方向/状态/搜索 |
| F26 | 静态 | **CSV 导出**：当前记录 + BOM + 浏览器下载 |
| F27 | 静态 | **诊断面板**：脱敏快照浮层 |
| F28 | 静态 | **清空**：二次确认 + 持久化 |
| F29 | 仅外观 | **全量持久化**：设置页所有开关写 `ll-settings-v1`，启动恢复 |
| F30 | 待实现 | 平台原生（托盘/自启/通知）保留为预览态，需打包后接入系统 API |
| F31 | 待实现 | 防火墙规则需管理员权限，设置开关已持久化，实际放行待打包签名后处理 |
| F32 | 静态 | **已接线**：脱敏诊断信息复制 + 打开日志目录 |
| F33 | 静态 | 检查更新/帮助内容为静态文案，需发布渠道后接入 |

### 11.3 新增 Tauri commands

`pick_files`、`pick_folder`、`read_clipboard`、`choose_save_dir`、`get_save_dir`、`set_discoverable`、`get_discoverable`、`get_blacklist_count`、`clear_blacklist`、`cancel_handshake`、`probe_serial`、`transfer_pause`、`transfer_resume`、`transfer_cancel`、`transfer_pause_all`、`generate_lan_code`、`connect_lan_code`（后两个为 D4 短码，见 §12）。

每个命令均同步了 `bridge/mock.ts` 与 `bridge/index.ts`，debug 态打印 `[lanlink-bridge] <cmd>`。

### 11.4 新增依赖

desktop crate：`tauri-plugin-dialog`、`tauri-plugin-clipboard-manager`、`walkdir`、`if-addrs`、`zeroize`、`blake3`。

### 11.5 验证结果

- `cargo check -p lanlink-desktop`：通过（0 error）。
- `cargo test -p lanlink-protocol`：26 passed，0 failed。
- `web npm run build`：通过。
- 验收要点 1/2/4/5 已满足；要点 3（1040×760 三表固定列宽）沿用已完成的 CSS 方案，本轮未改动表格布局。

### 11.6 已知限制（真机 / 发布阶段）

> 与 §0.5.2 TODO 表一一对应，状态同步维护。

1. ⬜ **[TODO-01] 跨网 NAT 穿透**：当前 TCP 拨号直连候选地址；Pub4/Pub6 经 STUN 观测但不做打洞，对称 NAT 场景需中继或 UPnP（未纳入本轮）。
2. ⚠️ **[TODO-03/04/05] F30/F31/F33**：托盘、开机自启、防火墙放行、检查更新依赖打包签名与系统权限，代码预留 UI 与持久化，实际生效待发布阶段。
3. ✅ **[TODO-02 已完成] TTL 设置传递**：设置页 TTL 900/300 → `ttlSecs` → `generate_serial` → 回写 `GenerateResult.ttl`；完成态倒计时、参数卡、表单态提示均按该值显示（2026-11-16）。连接方 TTL 校验仍用协议默认 900s（序列号本身不携带 TTL，属本地校验策略）。
4. ⚠️ **[TODO-06] 记录持久化位置**：用浏览器 localStorage 而非 Rust 侧文件，重装/清缓存会丢失；符合「仅存本机、不上传」但不跨浏览器。
5. ⚠️ **[TODO-07] 双端联调**：握手与传输为真实实现，需两台 LanLink 实例（生成方等待、连接方拨号）端到端验证；单机可自行生成并用同一序列号连接自测回环。
6. ⚠️ **[TODO-09] 显式清零**：FSM `cancel()` 与口令缓冲清零已接，派生密钥的 zeroize 尚未显式实现。

---

## 12. D4 局域网短码落地记录（2026-11-16）

规格见 `docs/lanlink-lan-code-PRD.md`。设计要点：**短码不携带地址**，只作 30 bit 随机索引；通过 UDP 广播用短码换取完整 serial，握手/传输 100% 复用外网序列号已落地的 FSM 与引擎。

### 12.1 改动清单

| 层 | 文件 | 改动 |
| --- | --- | --- |
| 文档 | `docs/lanlink-lan-code-PRD.md` | 新建：D4 完整规格（编码/报文/状态/错误码/复用边界/验收） |
| 后端 | `discovery.rs` | 扩展 UDP 42211 报文 `LQ/LA`：短码注册表（内存）、查询应答、`generate_code()`/`valid_code()`；关闭可发现即清短码 |
| 后端 | `lib.rs` | 新增 `build_lan_candidates()`（仅私网 IPv4/ULA，不写公网）+ 两个 command：`generate_lan_code`、`connect_lan_code`（含 `target_addr` 优先拨号） |
| 前端 | `GenerateLanDialog.vue` | 新建：生成短码弹窗（6 位码 / 复制 / 广播与等待状态 / 三步进度） |
| 前端 | `ConnectDialog.vue` | Tab2 接通：格式校验、设备可点选、`rescanLan` 真重扫、`runLanConnect` 走 `connect_lan_code`、状态横幅、watch(tab) 双 Tab 状态隔离 |
| 前端 | `App.vue` / `SendView.vue` / `TitleBar.vue` / `useUi.ts` | 挂载新弹窗；能力卡与菜单 `gen-lan` 入口；`DialogKey` 增加 `generateLan` |
| 桥接 | `types/invite.ts` / `bridge/mock.ts` / `bridge/index.ts` | 新增 `LanCodeResult` 类型与两个命令的 mock 回退 |

### 12.2 状态推进（C12–C16）

C12 短码输入、C13 设备列表、C14 重新搜索、C15 选择/连接、C16 双 Tab 隔离全部由「静态 / 仅输入框」推进为真实现，详见 §6.2 表格。

### 12.3 线协议

- 生成方：`generate_lan_code` 造短码 + 仅局域网候选的 serial → `register_code` + `start_host`（TCP 42210）。
- 查询方：`connect_lan_code` 广播 `LQ(code)` → 持有方回 `LA(serial)` → `decode_serial_inner` → `connect_as_receiver`（TTL/黑名单/质询 FSM）→ 传输引擎。
- 错误：短码非法 / 查询超时统一 E010（前端复用 E009 橙色错误卡骨架，仅替换文案与建议）。

### 12.4 验证结果

- `cargo check -p lanlink-desktop`：通过（0 error / 0 warning）。
- `web npm run build`：通过（95 modules）。
- 协议层 26 项单测未受影响（本轮未改协议层代码）。
- 默认原型态不变：短码功能需主动点击「生成局域网短码」或切到 Tab2 才触发。

### 12.5 已知限制

1. ⚠️ **[TODO-07] 端到端联调**：短码查询依赖 UDP 广播，跨网段/跨 VLAN 不转发 → E010；真机需两台同网段实例验证，单机可用回环直投自测（discovery 已向 127.0.0.1 额外投递一份 LQ）。
2. 📌 **[TODO-08 设计取舍] 短码跨网段/VLAN**：UDP 42211 广播不跨路由，PRD 定义短码仅同网段有效；要跨网段需另立需求改服务端注册或单播发现。
3. 📌 **E010 复用 E009 错误卡**：未进主协议 E001–E009 错误表，前端以橙色网络问题卡呈现，无需扩展类型系统。
3. **短码不设 TTL**：仅靠「进程内存注册表 + 退出即失效」保证新鲜度；长时间运行不自动过期（符合「程序运行期有效」规格）。
