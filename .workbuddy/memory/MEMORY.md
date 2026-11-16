# LanLink 项目长期记忆

> 详细规格权威来源：`docs/prototype-review.md`（§13–§24）与 `docs/lanlink-invite-serial-PRD.md`。本文件只记易踩坑点与当前状态，不重复规格细节。

## 定位与架构
- PC GUI 局域网/跨网 P2P 文件传输，无服务器；连接凭证为「外网序列号」（自包含候选地址的 Base32 串）。Tauri v2 + Rust + Vue3/Vite。
- Cargo workspace：`crates/lanlink-protocol`（纯 Rust 协议层 D3）/ `crates/lanlink-desktop`（Tauri 壳）/ `web`（Vue3 前端）/ `docs`。
- 前端无组件库、无 Arco 依赖：手写 `ui.css` 的 `.ll-table` + 纯 CSS design token（`data-theme` 明暗 × `data-palette` 6 套视觉风格）。状态 `useUi.ts`（reactive 单例），文案集中 `data/*.ts`。
- 前后端走 Tauri `invoke`（非 HTTP），浏览器预览回退 `bridge/mock.ts`。

## 关键协议决策
1. flags 单字节在块首且明文（加密块 = `[flags][nonce][ct][tag]`）。
2. AES-GCM AD 置空；完整性由 GCM Tag 覆盖。
3. 字符集 `ABCDEFGHJKLMNPQRSTUVWXYZ23456789`（含 L，PRD 括注矛盾以此为准）。

## 当前状态（2026-11-16）
- UI 已定版：32 个原型多轮对齐，三主视图 + 弹窗 + 6 套主题；`npm run build`、`cargo check -p lanlink-desktop`、`cargo test -p lanlink-protocol`（26/26）均通过。
- **F1–F33 功能已全部落地**：详见 `docs/feature-inventory.md` §11。后端新增 4 个模块 `net.rs`/`discovery.rs`/`session.rs`/`transfer.rs`；前端新增 `useRecords.ts`。15 个新 Tauri command 已配 bridge/mock。
- 真实链路：文件选择/剪贴板/目录持久化 → 网卡+STUN+IPv6 采集 → UDP 42211 设备发现 → TCP 42210 握手(复用 FSM+黑名单) → 4MB 分片传输(seq校验/落盘/Blake3) → `ll-transfer-progress` 事件驱动 UI → 记录持久化/CSV/诊断。
- 默认态逐像素回退原型（sendFiles/nearby/rateHistory 为空时用 `data/send.ts` 等常量），真机有数据才替换。
- **D4 局域网短码已落地**：规格 `docs/lanlink-lan-code-PRD.md`，实现见 §12 of feature-inventory。短码=30bit索引不携带地址，UDP 42211 广播 LQ/LA 换 serial→复用握手FSM/transfer。新增 `generate_lan_code`/`connect_lan_code` 命令 + `GenerateLanDialog.vue`；ConnectDialog Tab2 全接通(C12–C16)；E010 复用 E009 橙色错误卡。PRD 里程碑 D2/D3/D4 全标完成。
- 待做/限制：跨网 NAT 打洞未做(仅 STUN 直连)；F30/31/33(托盘/自启/防火墙/更新)需打包签名+系统权限，UI/持久化已预留；记录存 localStorage 非后端文件；握手/传输需双实例端到端验证；短码查询跨 VLAN 失效(E010)，单机可回环自测。TTL 设置已接通 generate_serial（设置 `ll-settings-v1.ttl` → `ttlSecs` → `GenerateResult.ttl`），连接方校验仍用默认 900s。

## 高频坑（速查，细节见 prototype-review.md）
- 启动：`tauri dev`（禁用 `-c`）；`beforeDevCommand` CWD 是 `<root>/crates`，前端前缀 `--prefix ../web`；Vite 5173 strictPort，devUrl 127.0.0.1；不用子 shell 分离。
- Tauri command 必须私有 `fn`；`generate_context!` 需 RGBA `icons/icon.png`；workspace 根需 `[workspace.package]`。
- capabilities 必须显式写窗口权限（minimize/close/toggle-maximize/start-dragging/set-theme），windows label `main`。
- 拖拽用 `data-tauri-drag-region`（非 `-webkit-app-region`），逐个标在文本节点。
- bridge 必须静态导入 `@tauri-apps/api/core`；debug 日志 `[lanlink-bridge]` / `[lanlink-window]` 是接通硬证据。
- macOS：`Overlay` + `hiddenTitle` + `trafficLightPosition` 保留交通灯/圆角；平台配置数组整体替换、macos 文件需重复完整 windows 数组；原生菜单 `menu.rs` 渲染于屏幕顶部，窗口内菜单 `v-if="!IS_MAC"`。
- 窗口主题跟应用不跟系统：`setTheme(resolved)`，否则系统 Dark + 应用 light 出 1px 黑描边。
- 三视图路由：`mainView: send|receive|history`；`centerState` 只属 send。能力卡 `.cap-card{margin-top:auto}` 贴底。
- 表格：`table-layout:fixed` + 明确列宽 + 窄列 nowrap + 单弹性列；行间/表头数据行无分割线；hover 整行 `--ll-color-row-hover`；表头 `--ll-color-surface-2`。
- 协调性：色板层禁止覆盖 `--ll-color-text*` 与语义色；品牌前景用 accent 同色相 on-primary 深色，禁止 `#fff`。
- 设置弹窗固定 720×590 / 内容高 440；5 项导航，「外观」是独立第 2 项。
- 预览直达：`?view=receive|history|send&send=...`、`?palette=`、`?theme=`；`LL_MIN_SIZE=1`、`LL_MENU_PROBE=...`、`LL_SIZE_PROBE=1` 为调试开关。
