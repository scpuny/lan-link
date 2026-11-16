//! LanLink 桌面端（Tauri v2）后端入口。
//!
//! 职责：Tauri 外壳 + 把 `lanlink-protocol` 协议层暴露给 Vue 前端（通过 `invoke` 桥接，
//! 全程进程内 IPC，不用 HTTP）。真网络传输 / mDNS / STUN 探测属于后续模块，本层先给出
//! 可跑通骨架：序列号生成与解码走真实协议编码，设备发现与连接给出结构正确的骨架数据。
//!
//! 所有 command 的返回结构**严格对齐** `web/src/types/invite.ts`（字段统一 camelCase）。
//! 注意：`#[tauri::command]` 函数必须保持私有，否则宏会生成重名符号。

use lanlink_protocol::payload::{AddrKind, Candidate, InvitePayload};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

mod discovery;
mod net;
mod session;
mod transfer;

// macOS 菜单栏是「应用级、屏幕顶部」的系统菜单，必须由 Rust 侧构建原生 NSMenu；
// Windows / Linux 没有全局菜单栏，菜单仍在窗口标题栏内由前端绘制。
#[cfg(target_os = "macos")]
mod menu;

// ---------------------------------------------------------------------------
// 与 web/src/types/invite.ts 一一对应的 IPC 数据结构
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateAddr {
    /// Pub6 / Pub4 / Lan4 / ULA（对应协议层 Pub6 / Pub4 / Lan4 / Lan6）
    pub kind: String,
    pub value: String,
    pub ready: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateResult {
    pub serial: String,
    pub session: String,
    pub candidates: Vec<CandidateAddr>,
    pub payload_bytes: usize,
    pub base32_chars: usize,
    pub created_at: u64,
    pub ttl: u32,
    pub encrypted: bool,
}

/// D4：生成局域网短码的结果。`serial` 是短码背后的完整凭证（经 UDP 递送给查询方），
/// `code` 是展示给用户的 6 位短码；局域网场景无口令、无 TTL 倒计时（运行期有效）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanCodeResult {
    pub code: String,
    pub serial: String,
    pub candidates: Vec<CandidateAddr>,
    pub created_at: u64,
    pub device_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedInvite {
    pub flags: u8,
    pub session: String,
    pub ttl: u32,
    pub ts: u64,
    pub candidates: Vec<CandidateAddr>,
    pub encrypted: bool,
    pub compressed: bool,
    pub payload_bytes: usize,
    pub base32_chars: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub peer_name: String,
    pub addr: String,
    pub rtt: u32,
    pub encrypted: bool,
    pub protocol: String,
    pub flags: String,
    pub took_ms: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NearbyDevice {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub addr: String,
    pub latency: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetCapabilities {
    pub mdns: bool,
    pub stun: bool,
    pub ipv6_global: bool,
    pub ipv4_direct: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfInfo {
    pub device_name: String,
    pub os_user: String,
    pub lan4: String,
    pub pub6: String,
    pub stun: String,
    pub net_capabilities: NetCapabilities,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDirResult {
    pub path: String,
}

/// 待发送文件条目（F1/F2/F3）：由原生对话框 / 剪贴板采集，前端负责 size/time/tone 的展示格式化。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    /// 绝对路径；剪贴板纯文本项为空（表示虚拟文本文件）
    pub path: String,
    pub name: String,
    pub size: u64,
    /// Unix 毫秒，前端格式化为 MM-DD HH:mm
    pub modified: u64,
    /// 大写扩展名，如 PDF / MP4；前端据此映射图标 tone
    pub ext: String,
}

/// 持久化到 <app_config_dir>/settings.json 的设置（F4 + F29 复用）。
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    #[serde(default)]
    save_dir: Option<String>,
}

// ---------------------------------------------------------------------------
// 工具函数
// ---------------------------------------------------------------------------

fn now_secs() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// 由 PathBuf 构造前端待发送条目；元数据失败时退化为 0 值而非整批失败。
fn file_entry_from_path(p: &std::path::Path) -> FileEntry {
    let name = p
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.to_string_lossy().into_owned());
    let ext = p
        .extension()
        .map(|s| s.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_default();
    let meta = std::fs::metadata(p).ok();
    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let modified = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    FileEntry {
        path: p.to_string_lossy().into_owned(),
        name,
        size,
        modified,
        ext,
    }
}

/// 设置文件路径：<app_config_dir>/settings.json。app_config_dir 不存在时返回空 PathBuf，
/// 调用方据此跳过持久化（不让设置失败阻断主流程）。
fn settings_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    use tauri::Manager;
    app.path()
        .app_config_dir()
        .map(|p| p.join("settings.json"))
        .unwrap_or_default()
}

fn load_settings(app: &tauri::AppHandle) -> AppSettings {
    let p = settings_path(app);
    if p.as_os_str().is_empty() {
        return AppSettings::default();
    }
    std::fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_settings(app: &tauri::AppHandle, s: &AppSettings) -> Result<(), String> {
    let p = settings_path(app);
    if p.as_os_str().is_empty() {
        return Ok(());
    }
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create config dir failed: {e}"))?;
    }
    std::fs::write(&p, serde_json::to_string_pretty(s).map_err(|e| e.to_string())?)
        .map_err(|e| format!("write settings failed: {e}"))
}

/// 生效的保存目录：优先用户在设置里持久化的路径，否则用默认 <HOME>/Downloads/LanLink。
fn effective_save_dir(app: &tauri::AppHandle) -> std::path::PathBuf {
    load_settings(app)
        .save_dir
        .filter(|s| !s.trim().is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_save_dir)
}

/// 开发构建下打印每次 bridge 调用，用于确认前端走的是 Tauri IPC 而不是 mock。
/// 发布构建编译成空函数，零开销、不污染日志。
#[cfg(debug_assertions)]
fn bridge_log(cmd: &str) {
    eprintln!("[lanlink-bridge] {}", cmd);
}
#[cfg(not(debug_assertions))]
fn bridge_log(_cmd: &str) {}

fn device_name() -> String {
    #[cfg(target_os = "windows")]
    let key = "COMPUTERNAME";
    #[cfg(not(target_os = "windows"))]
    let key = "HOSTNAME";
    std::env::var(key).unwrap_or_else(|_| "LanLink-Device".into())
}

/// 前端候选种类 → 协议层地址类型。ULA 归到 Lan6（都在 IPv6 段）。
fn kind_from_label(label: &str) -> Option<AddrKind> {
    match label {
        "Pub6" => Some(AddrKind::Pub6),
        "Pub4" => Some(AddrKind::Pub4),
        "Lan4" => Some(AddrKind::Lan4),
        "ULA" => Some(AddrKind::Lan6),
        _ => None,
    }
}

/// 采集本机所有可用候选地址（真实网卡 + STUN + IPv6）。每项含协议候选 + 展示文案 + 优先级。
/// 优先级（BR-05）：Pub6=0 → Pub4=1 → Lan4=2 → Lan6/ULA=3。
struct CandidatePool {
    kind: AddrKind,
    label: String,
    ip: IpAddr,
    shown: String,
    priority: u8,
}

/// F5–F8：一次真实采集，供 generate_serial 与 get_self_info 复用。
/// `with_stun=true` 时才做 STUN 探测（generate_serial 需要 Pub4，get_self_info 也展示）。
fn collect_candidates(with_stun: bool) -> Vec<CandidatePool> {
    let mut pool: Vec<CandidatePool> = Vec::new();
    let ifaces = net::enumerate_interfaces();

    // IPv4：区分私网 Lan4 与公网出口
    let mut seen_v4: Vec<Ipv4Addr> = Vec::new();
    for it in &ifaces {
        if it.is_loopback {
            continue;
        }
        if let IpAddr::V4(ip) = it.ip {
            if seen_v4.contains(&ip) {
                continue;
            }
            seen_v4.push(ip);
            if net::is_private_v4(&ip) {
                pool.push(CandidatePool {
                    kind: AddrKind::Lan4,
                    label: "Lan4".into(),
                    ip: IpAddr::V4(ip),
                    shown: ip.to_string(),
                    priority: 2,
                });
            }
        }
    }
    // 出口公网 v4（若出口 IP 本身非私网）
    if let Some(ip) = net::outbound_ipv4() {
        if !net::is_private_v4(&ip) && !seen_v4.contains(&ip) {
            pool.push(CandidatePool {
                kind: AddrKind::Pub4,
                label: "Pub4".into(),
                ip: IpAddr::V4(ip),
                shown: ip.to_string(),
                priority: 1,
            });
        }
    }

    // IPv6：全球单播 → Pub6；ULA → Lan6
    let mut seen_v6: Vec<Ipv6Addr> = Vec::new();
    for it in &ifaces {
        if it.is_loopback {
            continue;
        }
        if let IpAddr::V6(ip) = it.ip {
            // 排除 link-local（fe80::/10）与未指定
            if ip.is_unicast_link_local() || ip.is_unspecified() {
                continue;
            }
            if seen_v6.contains(&ip) {
                continue;
            }
            seen_v6.push(ip);
            if net::is_global_v6(&ip) {
                pool.push(CandidatePool {
                    kind: AddrKind::Pub6,
                    label: "Pub6".into(),
                    ip: IpAddr::V6(ip),
                    shown: ip.to_string(),
                    priority: 0,
                });
            } else if net::is_ula_v6(&ip) {
                pool.push(CandidatePool {
                    kind: AddrKind::Lan6,
                    label: "ULA".into(),
                    ip: IpAddr::V6(ip),
                    shown: ip.to_string(),
                    priority: 3,
                });
            }
        }
    }

    // F6：STUN 观测端点 → Pub4（优先级 1，覆盖直连出口）
    if with_stun {
        if let Some(ep) = net::stun_public_endpoint(
            "stun.l.google.com:19302",
            Duration::from_millis(1200),
        ) {
            if let IpAddr::V4(ip) = ep.ip() {
                pool.push(CandidatePool {
                    kind: AddrKind::Pub4,
                    label: "Pub4".into(),
                    ip: IpAddr::V4(ip),
                    shown: ep.to_string(),
                    priority: 1,
                });
            }
        }
    }

    // BR-05：按优先级排序，同类按 kind 稳定，再截断到 MAX_CANDIDATES
    pool.sort_by_key(|c| c.priority);
    pool.truncate(lanlink_protocol::MAX_CANDIDATES);
    pool
}

/// 按前端选中的候选类别，从真实采集池里挑选可用的协议候选。
/// 前端 `value` 是描述文案，仅作为回退展示；真实地址来自采集池。
fn build_real_candidates(selected: &[CandidateAddr]) -> (Vec<Candidate>, Vec<CandidateAddr>) {
    let pool = collect_candidates(true);
    let mut proto = Vec::new();
    let mut display = Vec::new();

    for c in selected {
        let want = match kind_from_label(&c.kind) {
            Some(k) => k,
            None => continue,
        };
        // 找到同类首个可用候选
        if let Some(hit) = pool.iter().find(|p| p.kind == want) {
            proto.push(Candidate {
                kind: hit.kind,
                port: 42210,
                addr: match hit.ip {
                    IpAddr::V4(ip) => ip.octets().to_vec(),
                    IpAddr::V6(ip) => ip.octets().to_vec(),
                },
            });
            display.push(CandidateAddr {
                kind: c.kind.clone(),
                value: hit.shown.clone(),
                ready: true,
            });
        }
    }
    // 若前端一个都没勾或都不匹配，回退到采集池首个，避免生成失败
    if proto.is_empty() {
        if let Some(hit) = pool.first() {
            proto.push(Candidate {
                kind: hit.kind,
                port: 42210,
                addr: match hit.ip {
                    IpAddr::V4(ip) => ip.octets().to_vec(),
                    IpAddr::V6(ip) => ip.octets().to_vec(),
                },
            });
            display.push(CandidateAddr {
                kind: hit.label.clone().into(),
                value: hit.shown.clone(),
                ready: true,
            });
        }
    }
    (proto, display)
}

/// D4：仅采集局域网候选（私网 IPv4 / ULA IPv6，绝不写公网端点 —— PRD §516 安全要求）。
/// 短码通过局域网广播递送，不需要也不应该暴露 STUN 观测到的公网地址。
fn build_lan_candidates() -> (Vec<Candidate>, Vec<CandidateAddr>) {
    let mut proto = Vec::new();
    let mut display = Vec::new();
    for it in net::enumerate_interfaces() {
        if it.is_loopback || proto.len() >= lanlink_protocol::MAX_CANDIDATES {
            continue;
        }
        let (cand, kind_label) = match it.ip {
            IpAddr::V4(ip) if net::is_private_v4(&ip) => (
                Candidate { kind: AddrKind::Lan4, port: 42210, addr: ip.octets().to_vec() },
                "Lan4",
            ),
            IpAddr::V6(ip) if net::is_ula_v6(&ip) => (
                Candidate { kind: AddrKind::Lan6, port: 42210, addr: ip.octets().to_vec() },
                "ULA",
            ),
            _ => continue,
        };
        proto.push(cand);
        display.push(CandidateAddr {
            kind: kind_label.into(),
            value: it.ip.to_string(),
            ready: true,
        });
    }
    (proto, display)
}

// ---------------------------------------------------------------------------
// Tauri commands（供前端 invoke 调用）
// ---------------------------------------------------------------------------

/// 生成外网序列号（PRD FR-01）。`password` 为 Some 时启用 AES-GCM 口令加密。
#[tauri::command]
fn generate_serial(
    candidates: Vec<CandidateAddr>,
    password: Option<String>,
    ttl_secs: Option<u32>,
    sessions: tauri::State<'_, Arc<session::SessionManager>>,
) -> Result<GenerateResult, String> {
    bridge_log("generate_serial");
    let (proto_cands, display) = build_real_candidates(&candidates);
    if proto_cands.is_empty() {
        return Err("E009: 未采集到可用候选地址（检查网卡 / IPv6 配置）".into());
    }
    if proto_cands.len() > lanlink_protocol::MAX_CANDIDATES {
        return Err(format!(
            "候选地址数量超过上限 {}",
            lanlink_protocol::MAX_CANDIDATES
        ));
    }

    let session: [u8; 16] = Uuid::new_v4().into_bytes(); // BR-04：必须 UUID v4
    let payload = InvitePayload {
        session_id: session,
        ts: now_secs(),
        candidates: proto_cands,
    };
    let pw = password.as_deref().map(str::as_bytes);
    let block = lanlink_protocol::codec::encode_block(&payload, pw);
    let payload_bytes = block.len();
    let serial = lanlink_protocol::codec::to_serial(&block);
    let base32_chars = serial.chars().filter(|c| *c != '-').count();

    // F12/F13：生成方作为 Initiator 启动 TCP 监听，等待对方连入完成质询握手。
    let _ = sessions.start_host(payload);

    Ok(GenerateResult {
        serial,
        session: hex(&session),
        candidates: display,
        payload_bytes,
        base32_chars,
        created_at: now_secs() as u64,
        // 设置页「外网序列号有效期」经前端 ttl_secs 传入（900s / 300s），缺省回落协议默认
        ttl: ttl_secs.unwrap_or(lanlink_protocol::DEFAULT_TTL_SECS as u32),
        encrypted: password.is_some(),
    })
}

/// 解码并真实校验序列号（PRD FR-04）。校验失败返回带错误码的字符串。
#[tauri::command]
fn decode_serial(serial: String, password: Option<String>) -> Result<DecodedInvite, String> {
    bridge_log("decode_serial");
    decode_serial_inner(&serial, password.as_deref())
}

/// F12–F14：连接远端并建立会话。真实 TCP 拨号 + 复用握手 FSM（TTL/黑名单/质询）。
/// 全候选不可达返回 E009；校验失败返回对应 E00x 错误码字符串。
#[tauri::command]
fn connect(
    serial: String,
    password: Option<String>,
    files: Option<Vec<transfer::TransferFile>>,
    sessions: tauri::State<'_, Arc<session::SessionManager>>,
) -> Result<SessionInfo, String> {
    bridge_log("connect");
    // 先解码拿到候选地址供拨号（真正的五级校验在 FSM 内再做一次，不重复判断）
    let invite = decode_serial_inner(&serial, password.as_deref())?;
    let candidates: Vec<String> = invite.candidates.iter().map(|c| c.value.clone()).collect();
    if candidates.is_empty() {
        return Err("E009: 凭证中无可用候选地址".into());
    }
    let pw = password.as_deref().map(str::as_bytes);
    let (addr, rtt) = sessions.connect_as_receiver(
        &serial,
        pw,
        &candidates,
        lanlink_protocol::DEFAULT_TTL_SECS,
        files.unwrap_or_default(),
    )?;
    Ok(SessionInfo {
        peer_name: "远端设备".into(),
        addr,
        rtt,
        encrypted: password.is_some(),
        protocol: format!("v{}", lanlink_protocol::PROTOCOL_VERSION),
        flags: if password.is_some() { "0x02" } else { "0x00" }.into(),
        took_ms: rtt,
    })
}

/// D4：生成 6 位局域网短码。只写局域网候选（不暴露公网地址），
/// 构造标准 serial 注册到发现服务，并启动 TCP host 等待查询方连入握手。
#[tauri::command]
fn generate_lan_code(
    sessions: tauri::State<'_, Arc<session::SessionManager>>,
    discovery: tauri::State<'_, Arc<discovery::Discovery>>,
) -> Result<LanCodeResult, String> {
    bridge_log("generate_lan_code");
    let (proto, display) = build_lan_candidates();
    if proto.is_empty() {
        return Err("E010: 未采集到局域网地址（检查是否接入网络）".into());
    }
    let code = discovery::generate_code();
    let session_id: [u8; 16] = Uuid::new_v4().into_bytes();
    let payload = InvitePayload {
        session_id,
        ts: now_secs(),
        candidates: proto,
    };
    let serial = lanlink_protocol::codec::to_serial(&lanlink_protocol::codec::encode_block(&payload, None));
    // 注册短码 → serial，并启动 Initiator 监听（与外网序列号共用 host 上下文）
    discovery.register_code(code.clone(), serial.clone());
    let _ = sessions.start_host(payload);

    Ok(LanCodeResult {
        code,
        serial,
        candidates: display,
        created_at: now_secs() as u64,
        device_name: device_name(),
    })
}

/// D4：用 6 位短码连接局域网持有方。先广播 LQ 换取 serial，再走标准握手 FSM。
/// 格式非法 / 查询无应答 → E010；serial 校验与握手失败 → 复用对应 E00x。
#[tauri::command]
fn connect_lan_code(
    code: String,
    target_addr: Option<String>,
    files: Option<Vec<transfer::TransferFile>>,
    sessions: tauri::State<'_, Arc<session::SessionManager>>,
    discovery: tauri::State<'_, Arc<discovery::Discovery>>,
) -> Result<SessionInfo, String> {
    bridge_log("connect_lan_code");
    let code = code.trim().to_ascii_uppercase();
    if !discovery::valid_code(&code) {
        return Err("E010: 短码须为 6 位字符（A-Z / 2-9）".into());
    }
    // 广播查询持有方，窗口期 2s；拿到 serial 后完全复用外网序列号的解码与握手
    let (serial, addr) = discovery
        .query_code(&code, Duration::from_millis(2000))
        .ok_or_else(|| "E010: 未发现持有该短码的设备（确认双方同网段、对方程序在运行）".to_string())?;
    let invite = decode_serial_inner(&serial, None)?;
    let candidates: Vec<String> = invite.candidates.iter().map(|c| c.value.clone()).collect();
    if candidates.is_empty() {
        return Err("E010: 短码对应的候选地址为空".into());
    }
    // 优先用「设备表选中的地址」拨号，其次应答来源地址（同一局域网最可靠），其余候选回落
    let mut ordered: Vec<String> = Vec::new();
    if let Some(t) = target_addr.filter(|s| !s.trim().is_empty()) {
        ordered.push(t.trim().to_string());
    }
    ordered.push(addr);
    for c in candidates {
        if !ordered.contains(&c) {
            ordered.push(c);
        }
    }
    let (peer, rtt) = sessions.connect_as_receiver(
        &serial,
        None,
        &ordered,
        lanlink_protocol::DEFAULT_TTL_SECS,
        files.unwrap_or_default(),
    )?;
    Ok(SessionInfo {
        peer_name: "局域网设备".into(),
        addr: peer,
        rtt,
        encrypted: false,
        protocol: format!("v{}", lanlink_protocol::PROTOCOL_VERSION),
        flags: "0x00".into(),
        took_ms: rtt,
    })
}

/// decode_serial 的内部实现，避免 command 之间互相调用时重复打印 bridge 日志。
fn decode_serial_inner(serial: &str, password: Option<&str>) -> Result<DecodedInvite, String> {
    let pw = password.map(str::as_bytes);
    let payload = lanlink_protocol::codec::decode_serial(serial, pw)
        .map_err(|e| format!("{}: {}", e.code, e.message))?;
    let flags = if pw.is_some() { lanlink_protocol::FLAG_ENCRYPT } else { 0 };
    let candidates = payload
        .candidates
        .iter()
        .map(|c| {
            let (kind, value) = match c.kind {
                AddrKind::Lan4 | AddrKind::Pub4 => {
                    let a = if c.addr.len() == 4 {
                        Ipv4Addr::new(c.addr[0], c.addr[1], c.addr[2], c.addr[3]).to_string()
                    } else { hex(&c.addr) };
                    (if c.kind == AddrKind::Lan4 { "Lan4" } else { "Pub4" }, a)
                }
                AddrKind::Lan6 | AddrKind::Pub6 => {
                    let a = if c.addr.len() == 16 {
                        let mut o = [0u8; 16]; o.copy_from_slice(&c.addr); Ipv6Addr::from(o).to_string()
                    } else { hex(&c.addr) };
                    (if c.kind == AddrKind::Lan6 { "ULA" } else { "Pub6" }, a)
                }
            };
            CandidateAddr { kind: kind.into(), value, ready: true }
        })
        .collect();
    let base32_chars = serial.chars().filter(|c| *c != '-').count();
    Ok(DecodedInvite {
        flags,
        session: hex(&payload.session_id),
        ttl: lanlink_protocol::DEFAULT_TTL_SECS as u32,
        ts: payload.ts as u64,
        candidates,
        encrypted: pw.is_some(),
        compressed: flags & lanlink_protocol::FLAG_COMPRESS != 0,
        payload_bytes: 21 + payload.candidates.len() * 19,
        base32_chars,
    })
}

/// F9/F11：扫描同网段设备。立即广播一次宣告并收集真实回应，RTT 为测量值。
#[tauri::command]
fn scan_lan(discovery: tauri::State<'_, Arc<discovery::Discovery>>) -> Result<Vec<NearbyDevice>, String> {
    bridge_log("scan_lan");
    let seen = discovery.scan(Duration::from_millis(700));
    Ok(seen
        .into_iter()
        .map(|d| NearbyDevice {
            id: d.id,
            name: d.name,
            platform: d.platform,
            addr: d.addr,
            latency: d.latency_ms,
        })
        .collect())
}

/// F10：设置本机是否可被同网段发现。
#[tauri::command]
fn set_discoverable(
    on: bool,
    discovery: tauri::State<'_, Arc<discovery::Discovery>>,
) -> Result<bool, String> {
    bridge_log("set_discoverable");
    discovery.set_discoverable(on);
    Ok(discovery.is_discoverable())
}

/// F10：读取当前可发现状态。
#[tauri::command]
fn get_discoverable(
    discovery: tauri::State<'_, Arc<discovery::Discovery>>,
) -> Result<bool, String> {
    bridge_log("get_discoverable");
    Ok(discovery.is_discoverable())
}

/// F21：暂停单个传输任务
#[tauri::command]
fn transfer_pause(id: String, tm: tauri::State<'_, Arc<transfer::TransferManager>>) -> Result<bool, String> {
    bridge_log("transfer_pause");
    Ok(tm.pause(&id))
}
/// F21：继续单个传输任务
#[tauri::command]
fn transfer_resume(id: String, tm: tauri::State<'_, Arc<transfer::TransferManager>>) -> Result<bool, String> {
    bridge_log("transfer_resume");
    Ok(tm.resume(&id))
}
/// F21：取消单个传输任务
#[tauri::command]
fn transfer_cancel(id: String, tm: tauri::State<'_, Arc<transfer::TransferManager>>) -> Result<bool, String> {
    bridge_log("transfer_cancel");
    Ok(tm.cancel(&id))
}
/// F21：命令栏「全部暂停」
#[tauri::command]
fn transfer_pause_all(tm: tauri::State<'_, Arc<transfer::TransferManager>>) -> Result<bool, String> {
    bridge_log("transfer_pause_all");
    tm.pause_all();
    Ok(true)
}

/// F15：只读取序列号的 flags（不解密、不校验 TTL），供前端判断是否需要弹出口令输入。
/// 返回 { encrypted, compressed }；解析失败返回 E001。
#[tauri::command]
fn probe_serial(serial: String) -> Result<serde_json::Value, String> {
    bridge_log("probe_serial");
    let block = lanlink_protocol::codec::from_serial(&serial)
        .map_err(|e| format!("{}: {}", e.code, e.message))?;
    let flags = block.first().copied().unwrap_or(0);
    Ok(serde_json::json!({
        "encrypted": flags & lanlink_protocol::FLAG_ENCRYPT != 0,
        "compressed": flags & lanlink_protocol::FLAG_COMPRESS != 0,
        "flags": flags,
    }))
}

/// F17：查询会话黑名单条数（设置页展示真实值）。
#[tauri::command]
fn get_blacklist_count(sessions: tauri::State<'_, Arc<session::SessionManager>>) -> Result<usize, String> {
    bridge_log("get_blacklist_count");
    Ok(sessions.blacklist_count())
}

/// F17：清空会话黑名单。
#[tauri::command]
fn clear_blacklist(sessions: tauri::State<'_, Arc<session::SessionManager>>) -> Result<usize, String> {
    bridge_log("clear_blacklist");
    sessions.clear_blacklist();
    Ok(0)
}

/// F16：取消当前握手，释放 challenge / 口令上下文（口令零化由调用方处理）。
#[tauri::command]
fn cancel_handshake(sessions: tauri::State<'_, Arc<session::SessionManager>>) -> Result<bool, String> {
    bridge_log("cancel_handshake");
    sessions.cancel();
    Ok(true)
}

/// 本机信息（设备名 / 网卡 / STUN / IPv6 能力），全部真实采集（F5–F7）。
#[tauri::command]
fn get_self_info() -> Result<SelfInfo, String> {
    bridge_log("get_self_info");
    let v4 = net::outbound_ipv4();
    let v6 = net::outbound_ipv6();
    // 从网卡里挑一个私网 Lan4 展示（优先于出口 IP 的公网语义）
    let lan4 = net::enumerate_interfaces()
        .into_iter()
        .find_map(|it| match it.ip {
            IpAddr::V4(ip) if !it.is_loopback && net::is_private_v4(&ip) => Some(ip),
            _ => None,
        })
        .or(v4);
    // STUN 观测端点（与 generate_serial 一致，1.2s 超时）
    let stun = net::stun_public_endpoint("stun.l.google.com:19302", Duration::from_millis(1200));
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "当前用户".into());
    Ok(SelfInfo {
        device_name: device_name(),
        os_user: format!("{} · {}", std::env::consts::OS, user),
        lan4: lan4.map(|ip| ip.to_string()).unwrap_or_default(),
        pub6: v6.map(|ip| ip.to_string()).unwrap_or_default(),
        stun: stun.map(|ep| ep.to_string()).unwrap_or_default(),
        net_capabilities: NetCapabilities {
            mdns: true,
            stun: stun.is_some(),
            ipv6_global: v6.is_some(),
            ipv4_direct: v4.map(|ip| !net::is_private_v4(&ip)).unwrap_or(false),
        },
    })
}

/// F1：选择文件（多选）→ 追加到待发送表。原生对话框，取消返回空数组。
#[tauri::command]
fn pick_files(app: tauri::AppHandle) -> Result<Vec<FileEntry>, String> {
    bridge_log("pick_files");
    use tauri_plugin_dialog::DialogExt;
    let picked = app.dialog().file().blocking_pick_files();
    let mut files = Vec::new();
    for p in picked.unwrap_or_default() {
        let path = p.into_path().map_err(|e| e.to_string())?;
        files.push(file_entry_from_path(&path));
    }
    Ok(files)
}

/// F1：选择文件夹 → 递归列出其中所有文件追加到待发送表。
#[tauri::command]
fn pick_folder(app: tauri::AppHandle) -> Result<Vec<FileEntry>, String> {
    bridge_log("pick_folder");
    use tauri_plugin_dialog::DialogExt;
    let folder = app.dialog().file().blocking_pick_folder();
    let folder = match folder {
        Some(f) => f.into_path().map_err(|e| e.to_string())?,
        None => return Ok(Vec::new()),
    };
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(&folder).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            files.push(file_entry_from_path(entry.path()));
        }
    }
    Ok(files)
}

/// F2：从剪贴板读取文本。若文本是已存在的本地路径则转成文件条目，否则作为虚拟文本文件由前端处理。
#[tauri::command]
fn read_clipboard(app: tauri::AppHandle) -> Result<String, String> {
    bridge_log("read_clipboard");
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let text = app.clipboard().read_text().map_err(|e| e.to_string())?;
    Ok(text)
}

/// F4：选择保存目录并持久化，返回生效路径。取消则保持不变。
#[tauri::command]
fn choose_save_dir(
    app: tauri::AppHandle,
    tm: tauri::State<'_, Arc<transfer::TransferManager>>,
) -> Result<SaveDirResult, String> {
    bridge_log("choose_save_dir");
    use tauri_plugin_dialog::DialogExt;
    let picked = app.dialog().file().blocking_pick_folder();
    if let Some(dir) = picked {
        let mut s = load_settings(&app);
        s.save_dir = Some(
            dir.into_path()
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .into_owned(),
        );
        save_settings(&app, &s)?;
    }
    let dir = effective_save_dir(&app);
    // F19：同步给传输引擎，接收落盘立即使用新目录
    tm.set_save_dir(dir.clone());
    Ok(SaveDirResult {
        path: dir.to_string_lossy().into_owned(),
    })
}

/// F4：读取当前生效的保存目录（用于设置页 / 接收页展示）。
#[tauri::command]
fn get_save_dir(app: tauri::AppHandle) -> Result<SaveDirResult, String> {
    bridge_log("get_save_dir");
    Ok(SaveDirResult {
        path: effective_save_dir(&app).to_string_lossy().into_owned(),
    })
}

/// 打开接收文件保存目录（命令栏「打开保存目录」）。使用用户配置的生效目录。
#[tauri::command]
fn open_save_dir(app: tauri::AppHandle) -> Result<SaveDirResult, String> {
    bridge_log("open_save_dir");
    let dir = effective_save_dir(&app);
    std::fs::create_dir_all(&dir).map_err(|e| format!("create save dir failed: {e}"))?;
    reveal_in_file_manager(&dir)?;
    Ok(SaveDirResult {
        path: dir.to_string_lossy().into_owned(),
    })
}

/// 默认接收目录：<用户目录>/Downloads/LanLink，后续可由设置页持久化配置覆盖。
fn default_save_dir() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    let home = std::env::var_os("USERPROFILE");
    #[cfg(not(target_os = "windows"))]
    let home = std::env::var_os("HOME");

    home.map(std::path::PathBuf::from)
        .map(|p| p.join("Downloads").join("LanLink"))
        .unwrap_or_else(|| std::path::PathBuf::from("LanLink"))
}

#[cfg(target_os = "macos")]
fn reveal_in_file_manager(dir: &std::path::Path) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(dir)
        .spawn()
        .map_err(|e| format!("open Finder failed: {e}"))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn reveal_in_file_manager(dir: &std::path::Path) -> Result<(), String> {
    std::process::Command::new("explorer")
        .arg(dir)
        .spawn()
        .map_err(|e| format!("open Explorer failed: {e}"))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn reveal_in_file_manager(dir: &std::path::Path) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(dir)
        .spawn()
        .map_err(|e| format!("open file manager failed: {e}"))?;
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn reveal_in_file_manager(_dir: &std::path::Path) -> Result<(), String> {
    Err("unsupported platform".into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            // F9/F10：创建局域网发现服务并注入全局状态，后台线程持续宣告 + 监听。
            {
                let name = device_name();
                let os = std::env::consts::OS.to_string();
                let lan4 = net::enumerate_interfaces()
                    .into_iter()
                    .find_map(|it| match it.ip {
                        IpAddr::V4(ip) if !it.is_loopback && net::is_private_v4(&ip) => Some(ip),
                        _ => None,
                    })
                    .or_else(net::outbound_ipv4)
                    .map(|ip| ip.to_string())
                    .unwrap_or_default();
                let disc = Arc::new(discovery::Discovery::new(name, os, lan4));
                (*disc).clone().spawn();
                use tauri::Manager;
                app.manage(disc);
            }

            // F12–F23：注入传输引擎 + 握手会话管理（黑名单 + host 上下文 + TCP 监听）
            {
                use tauri::Manager;
                let tm = Arc::new(transfer::TransferManager::new(
                    app.handle().clone(),
                    default_save_dir(),
                ));
                app.manage(tm.clone());
                app.manage(Arc::new(session::SessionManager::new(tm)));
            }

            // 开发期输出实际生效的窗口属性，便于核对平台配置是否被合并：
            // macOS 期望 decorated=true（titleBarStyle=Overlay 保留交通灯），
            // Windows/Linux 期望 decorated=false（自绘三键）。
            #[cfg(debug_assertions)]
            {
                use tauri::Manager;
                if let Some(w) = app.get_webview_window("main") {
                    eprintln!(
                        "[lanlink-window] decorated={:?} size={:?}",
                        w.is_decorated(),
                        w.inner_size()
                    );

                    // 最小尺寸探针：输出**合并后**的窗口配置，确认平台配置里的
                    // minWidth/minHeight 真的进了运行时（而不是只写在文件里）。
                    // 注意：macOS 的 NSWindow.setContentSize 不应用 minSize，
                    // 所以「程序里 set_size 到更小值」不能用来判定约束是否生效，
                    // 只有用户拖拽边框时窗口管理器才会按 minSize 钳制。
                    if std::env::var("LL_SIZE_PROBE").is_ok() {
                        let cfg = app.config();
                        if let Some(wc) = cfg.app.windows.first() {
                            eprintln!(
                                "[lanlink-size-probe] config width={} height={} min={:?}x{:?}",
                                wc.width, wc.height, wc.min_width, wc.min_height
                            );

                            // LL_MIN_SIZE=1：直接把窗口摆到配置的最小尺寸，
                            // 用于验收「极限窄窗」下的布局，不用手拖边框。
                            if std::env::var("LL_MIN_SIZE").is_ok() {
                                if let (Some(mw), Some(mh)) = (wc.min_width, wc.min_height) {
                                    let _ = w.set_size(tauri::LogicalSize::new(mw, mh));
                                    eprintln!("[lanlink-size-probe] resized to min {mw}x{mh}");
                                }
                            }
                        }
                    }
                }
            }

            // macOS：安装原生菜单栏，并把点击转成 `ll-menu` 事件交给前端分发。
            #[cfg(target_os = "macos")]
            install_macos_menu(app)?;

            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            generate_serial,
            decode_serial,
            connect,
            generate_lan_code,
            connect_lan_code,
            scan_lan,
            get_self_info,
            open_save_dir,
            pick_files,
            pick_folder,
            read_clipboard,
            choose_save_dir,
            get_save_dir,
            set_discoverable,
            get_discoverable,
            get_blacklist_count,
            clear_blacklist,
            cancel_handshake,
            probe_serial,
            transfer_pause,
            transfer_resume,
            transfer_cancel,
            transfer_pause_all,
        ])
        .run(tauri::generate_context!())
        .expect("error while running LanLink desktop");
}

/// 安装 macOS 系统菜单栏（屏幕顶部）。
///
/// 菜单动作不在 Rust 里实现业务，只负责转发：前端 `TitleBar.onAction` 同时被
/// 窗口内菜单（Windows/Linux）和这里的事件（macOS）驱动，行为完全一致。
#[cfg(target_os = "macos")]
fn install_macos_menu(app: &mut tauri::App) -> tauri::Result<()> {
    use tauri::Emitter;

    let handle = app.handle().clone();
    let menu = menu::build(&handle)?;
    // 应用级菜单：macOS 只有一份，设置一次即可
    app.set_menu(menu)?;

    app.on_menu_event(|app, event| {
        let action = menu::split_id(&event.id().0);
        #[cfg(debug_assertions)]
        eprintln!("[lanlink-menu] {} {:?}", action.key, action.value);
        if let Err(e) = app.emit(menu::MENU_EVENT, action) {
            eprintln!("[lanlink-menu] emit failed: {e}");
        }
    });

    // 验证用探针：`LL_MENU_PROBE=1 tauri dev` 时每 3 秒发一次「接收文件」动作，
    // 用于确认「Rust emit → 前端 listen → onAction」整条链路通畅（无 UI 点击权限时用）。
    // 也可以指定动作键，例如 `LL_MENU_PROBE=view-history` 直接切到传输记录，便于截图验收。
    // 只在 debug 构建且显式设置环境变量时生效，正常启动完全不影响。
    #[cfg(debug_assertions)]
    if let Ok(v) = std::env::var("LL_MENU_PROBE") {
        let key = if v.is_empty() || v == "1" { "view-recv".to_string() } else { v };
        let h = app.handle().clone();
        std::thread::spawn(move || {
            for _ in 0..20 {
                std::thread::sleep(std::time::Duration::from_secs(3));
                let _ = h.emit(
                    menu::MENU_EVENT,
                    menu::MenuAction {
                        key: key.clone(),
                        value: None,
                    },
                );
            }
        });
    }

    Ok(())
}
