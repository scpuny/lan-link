# LanLink 局域网短码协议｜技术规格（D4）

| 项 | 值 |
| --- | --- |
| 文档编号 | D4 |
| 关联文档 | `lanlink-invite-serial-PRD.md` 附录 A（对照速览）、`prototype-review.md` §5（对话框规格） |
| 定位 | 独立协议，非外网序列号的变体；两者**不共用凭证、输入缓冲、状态区**（BR-06） |
| 状态 | ✅ 规格定稿，已落地实现（对应 F34–F38） |
| 使用网络 | 仅局域网，断外网可用；不经过任何服务器 |

---

## 1. 设计目标

外网序列号（长串 Base32）适合跨网，但 handshake 前提是要先有一条可信的候选地址通道。局域网内两台设备处于同一广播域时，可以只交换一个**6 位短码**完成地址发现，避免手动抄写 IP。

核心取舍：**短码不携带任何地址**，它只是一个 30 bit 的随机索引；地址通过 UDP 广播查询动态解析。这样短码足够短、口令般易念，且不会把内网地址写进可能被转发的凭证里。

---

## 2. 凭证编码

| 项 | 规格 |
| --- | --- |
| 形态 | 6 位大写字符，如 `K7G2S9` |
| 字符集 | `ABCDEFGHJKLMNPQRSTUVWXYZ23456789`（32 符号，同外网序列号，`base32::ALPHABET`） |
| 信息量 | 每字符 5 bit × 6 = **30 bit 索引**，不编码地址、不编码 session |
| 生成 | OS 随机源取 4 字节，取低 30 bit，逐 5 bit 映射字符集 |
| 校验 | 仅字符集 + 长度 6；无校验位，输错由查询无应答兜底（表现为超时） |
| 有效期 | **程序生成期内有效**（进程内存注册表），退出即失效；无 TTL 倒计时 |

> 与外网序列号的隔离：短码不能喂给 `decode_serial`，序列号不能喂给 `connect_lan_code`；字符集相同但语义完全不同。切换 Tab 不保留输入缓冲。

---

## 3. 发现与交换机制

复用现有发现服务的 UDP **42211** 端口与广播器（`discovery.rs`），在其上报文中扩展两种 kind。TCP 数据端口仍为 **42210**，握手与传输引擎 100% 复用现有实现。

### 3.1 报文（JSON over UDP，magic `LANLINK1`）

| kind | 方向 | 关键字段 | 含义 |
| --- | --- | --- | --- |
| `ANNOUNCE` | 组播/广播 | `id,n,os,ip,port,t0` | 已有：宣告设备在线，用于附近设备表 |
| `LQ` | 查询方 → 广播 | `c`（6 位短码）、`r`（回复用的临时 UDP 端口） | 「谁持有短码 K7G2S9？」 |
| `LA` | 持有方 → 单播 | `c`（短码）、`s`（完整外网序列号，含 Lan4/Lan6 候选） | 「我持有，这是可直接握手的候选包」 |

关键设计：**持有方回传的是完整 serial，而不是裸 IP**。这样连接方拿到后直接走 `decode_serial_inner` → `connect_as_receiver` 的标准 FSM（TTL / 黑名单 / 质询 / 应答），无需为短码单开一套校验逻辑。短码只负责"把 serial 安全地在局域网内递过去"这一段。

### 3.2 持有方（生成短码的一端）

1. `generate_lan_code` 生成 6 位短码，构造含本机 Lan4 + Lan6 候选的 `InvitePayload`（无口令、无压缩标记），生成标准 serial。
2. 调用 `SessionManager.start_host(payload)` 启动 TCP 42210 监听（与外网序列号共用同一 Initiator 上下文，最新一次覆盖旧的）。
3. 在发现服务注册 `短码 → serial` 到**内存注册表**；持有期间在 ANNOUNCE 中携带 `hasCode` 标记（供附近设备表提示"该设备可短码连接"）。
4. 收到 `LQ` 且 `c` 命中注册表 → 单播回 `LA`（携带缓存的 serial）。

### 3.3 查询方（输入短码的一端）

1. `connect_lan_code(code, files)` → 发现服务广播 `LQ`，监听 `LA`，在窗口期（默认 ~1.5s）收集应答。
2. 若在设备表选中了具体设备，则改为对该设备 IP 单播 `LQ`，只接受该来源的 `LA`（避免同网段重码串台）。
3. 拿到 `LA.s` → `decode_serial_inner`（校验 CRC/GCM/Payload）→ 提取候选地址 → `connect_as_receiver` 完成握手 → `TransferManager` 接管同一条 TCP 连接传文件。
4. 收集到的短码在内存去重；无应答返回 `E010`（见 §5）。

---

## 4. 状态与生命周期

```
生成方                              连接方
generate_lan_code()                 connect_lan_code(code)
  ├ 造短码 + serial                    ├ 广播 LQ(code)
  ├ start_host(TCP 42210)             ├ 收 LA → decode_serial
  ├ 注册 code→serial(内存)             ├ connect_as_receiver(FSM 握手)
  └ 广播 ANNOUNCE(hasCode)             └ TransferManager 传文件
        │      LA(serial)  ──────────────┘
        └ serve_initiator → recv_files 落盘
```

| 事件 | 处理 |
| --- | --- |
| 进程退出 | 注册表清空，短码立即失效（无持久化、无 TTL 文件） |
| 重新生成短码 | 覆盖注册表与 host 上下文，旧短码失效 |
| 跨子网 / 跨 VLAN | UDP 多播不转发 → 查询无应答 → E010；提示改用外网序列号 |
| 关闭"允许被发现" | 不再回 `LA`（仍可主动查询别人），与 F10 开关一致 |

---

## 5. 错误码

沿用主协议错误码空间，新增一个局域网专用码：

| 码 | 含义 | 触发 | UI 处理 |
| --- | --- | --- | --- |
| `E010` | 短码查询超时 / 无持有方应答 | 广播 LQ 后窗口期内无 LA | 橙：提示确认双方同网段、对方程序在运行；主按钮「重新搜索」/「改用外网序列号」 |
| `E009` | serial 内候选地址不可达 | LA 已拿到但 TCP 全候选拨号失败 | 复用主协议 E009 |
| `E001–E008` | 握手/凭证校验失败 | 走 `decode_serial_inner` / FSM | 复用主协议对应错误卡 |

> 客户端校验顺序固定：短码格式（字符集+6 位）→ 查询应答 → serial 完整性 → TTL → 黑名单 → 质询。不可调换。

---

## 6. 与现有模块的复用边界

| 模块 | 是否复用 | 说明 |
| --- | --- | --- |
| `net.rs` 网卡/Lan4 采集 | ✅ 复用 | 生成 serial 的候选来源 |
| `discovery.rs` UDP 广播 | ✅ 扩展 | 增加 LQ/LA 报文与短码注册表 |
| `session.rs` TCP 握手 | ✅ 复用 | Initiator/Receiver FSM 原样使用 |
| `transfer.rs` 传输引擎 | ✅ 复用 | 分片/落盘/校验/进度事件原样使用 |
| `codec/base32/crypto` | ✅ 复用 | serial 编解码与校验原样使用 |

新增代码集中在 `discovery.rs`（短码查询应答 + 注册表）与少量 Tauri command，前端新增一个生成短码弹窗、接通短码 Tab。

---

## 7. 验收要点

1. 两台同网段实例：A 生成短码 → B 输入短码 → 建立会话 → B 选文件发送 → A 落盘，全程断外网可用。
2. B 从附近设备表选中 A 再连接，只回应 A 的 LA，不受同网段其他实例干扰。
3. 切换 Tab 后，外网序列号输入缓冲与短码输入互不影响。
4. 进程退出后旧短码再查询返回 E010。
5. `cargo test -p lanlink-protocol` 全绿；`cargo check -p lanlink-desktop`、`npm run build` 通过。
