//! 握手会话管理（F12–F17）：真实 TCP 通道 + 复用 `lanlink_protocol::handshake` FSM。
//!
//! 角色对齐 PRD 第 6 节（与直觉相反，务必按此实现）：
//!  - **生成序列号方 = Initiator（A）**：监听 TCP 42210，等待对方连入；
//!    收到 challenge 后用 `InitiatorFsm.on_challenge` 计算 Blake3 应答并回传。
//!  - **粘贴序列号方 = Receiver（B）**：主动拨号；用 `ReceiverFsm.connect` 做
//!    清洗→CRC/GCM→Payload→TTL→黑名单→下发 challenge，再用 `verify_response` 比对。
//!
//! 线协议（长度前缀 JSON over TCP，端口 42210）：
//!   B→A `{"t":"challenge","v":"<hex16>"}`
//!   A→B `{"t":"response","v":"<hex32>"}` 或 `{"t":"error","c":"E00x"}`
//!
//! 黑名单仅存进程内存（BR-02），退出即清空；口令派生密钥用完 zeroize（BR-01）。

use crate::transfer::{TransferFile, TransferManager};
use lanlink_protocol::handshake::{InitiatorAction, InitiatorFsm, ReceiverAction, ReceiverFsm};
use lanlink_protocol::payload::InvitePayload;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const DATA_PORT: u16 = 42210;

#[derive(Debug, Serialize, Deserialize)]
struct Wire {
    t: String,
    #[serde(default)]
    v: String,
    #[serde(default)]
    c: String,
}

/// 会话全局状态：黑名单 + 当前作为 Initiator 的 host 上下文（最新一次生成覆盖旧的，FR-12）。
#[derive(Clone)]
pub struct SessionManager {
    blacklist: Arc<Mutex<HashSet<[u8; 16]>>>,
    /// host 生成的原始 payload。每次连入都用它 new 一个全新的 InitiatorFsm，
    /// 避免同一个 FSM 用过一次进入 ATrans 后无法服务第二次连接（短码可反复使用）。
    host_payload_raw: Arc<Mutex<Option<InvitePayload>>>,
    /// host 生成的 payload（含 session_id），供连入方计算应答
    host_payload: Arc<Mutex<Option<InitiatorFsm>>>,
    /// 最近一次握手结果摘要（供 UI 展示真实 RTT / 对端）
    last_rtt_ms: Arc<Mutex<u32>>,
    /// F18–F23：传输引擎（握手成功后接管同一条 TCP 连接）
    transfer: Arc<TransferManager>,
}

impl SessionManager {
    pub fn new(transfer: Arc<TransferManager>) -> Self {
        Self {
            blacklist: Arc::new(Mutex::new(HashSet::new())),
            host_payload_raw: Arc::new(Mutex::new(None)),
            host_payload: Arc::new(Mutex::new(None)),
            last_rtt_ms: Arc::new(Mutex::new(0)),
            transfer,
        }
    }

    pub fn blacklist_count(&self) -> usize {
        self.blacklist.lock().map(|s| s.len()).unwrap_or(0)
    }

    pub fn clear_blacklist(&self) {
        if let Ok(mut s) = self.blacklist.lock() {
            s.clear();
        }
    }

    /// F16：取消所有进行中的握手上下文，清零口令/密钥由调用方 zeroize。
    pub fn cancel(&self) {
        if let Ok(mut h) = self.host_payload.lock() {
            if let Some(fsm) = h.as_mut() {
                fsm.cancel();
            }
            *h = None;
        }
        if let Ok(mut raw) = self.host_payload_raw.lock() {
            *raw = None;
        }
    }

    /// 启动 host（Initiator）：注册生成的 payload，并在 DATA_PORT 监听连入。
    /// 每次生成都覆盖 host 上下文；监听线程存活于进程期，逐次读取当前上下文。
    pub fn start_host(&self, payload: InvitePayload) -> std::io::Result<()> {
        let mut fsm = InitiatorFsm::new();
        let _ = fsm.on_generated(payload.clone());
        *self.host_payload_raw.lock().unwrap() = Some(payload);
        *self.host_payload.lock().unwrap() = Some(fsm);

        // 监听线程只启动一次（端口占用即认为已在监听）
        let mgr = self.clone();
        std::thread::Builder::new()
            .name("lanlink-host-listener".into())
            .spawn(move || {
                let listener = match TcpListener::bind(("0.0.0.0", DATA_PORT)) {
                    Ok(l) => l,
                    Err(e) => {
                        #[cfg(debug_assertions)]
                        eprintln!("[lanlink-session] host bind {DATA_PORT} failed: {e}");
                        return;
                    }
                };
                #[cfg(debug_assertions)]
                eprintln!("[lanlink-session] host listening on {DATA_PORT}");
                for stream in listener.incoming() {
                    match stream {
                        Ok(s) => {
                            let m = mgr.clone();
                            std::thread::spawn(move || m.serve_initiator(s));
                        }
                        Err(_) => continue,
                    }
                }
            })?;
        Ok(())
    }

    /// Initiator 侧：收到 challenge → 计算应答 → 回传。
    fn serve_initiator(&self, mut stream: TcpStream) {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
        let msg = match read_wire(&mut stream) {
            Some(m) => m,
            None => return,
        };
        if msg.t != "challenge" {
            return;
        }
        let ch = parse_hex::<16>(&msg.v);
        let ch = match ch {
            Some(c) => c,
            None => {
                let _ = write_wire(&mut stream, &Wire { t: "error".into(), v: String::new(), c: "E008".into() });
                return;
            }
        };

        // 用当前 host 上下文计算应答（FSM 状态机驱动，不手写摘要）。
        // 每次连入都基于原始 payload 新建一个 FSM，并走完整状态链：
        //   on_generated → on_incoming_connect → on_challenge → on_response_accepted
        // 少了 on_incoming_connect 会停留在 ADial 而被 on_challenge 判 E008；
        // 复用旧 FSM 会在第二次连接时因 ATrans 失败，短码按设计需可反复使用。
        let response = {
            let raw = self.host_payload_raw.lock().unwrap().clone();
            match raw {
                Some(payload) => {
                    let mut fsm = InitiatorFsm::new();
                    let _ = fsm.on_generated(payload);
                    let _ = fsm.on_incoming_connect(); // ADial → AWaitChallenge
                    let act = fsm.on_challenge(ch);
                    match act {
                        InitiatorAction::SendResponse(r) => {
                            let _ = fsm.on_response_accepted();
                            Some(r)
                        }
                        _ => None,
                    }
                }
                None => None,
            }
        };
        match response {
            Some(r) => {
                let _ = write_wire(&mut stream, &Wire {
                    t: "response".into(), v: to_hex(&r), c: String::new(),
                });
                // F18–F23：生成方（host）作为接收方，接管连接落盘 + 整文件校验。
                #[cfg(debug_assertions)]
                eprintln!("[lanlink-session] initiator handshake OK, entering recv_files");
                self.transfer.clone().recv_files(stream);
            }
            None => {
                let _ = write_wire(&mut stream, &Wire { t: "error".into(), v: String::new(), c: "E008".into() });
            }
        }
    }

    /// Receiver 侧：拨号候选地址 → FSM 校验 → challenge → 应答比对 → 建立。
    /// 返回 (对端地址, 真实 RTT ms)；失败返回带错误码的字符串。
    /// `check_ttl`：外网序列号必须校验 900s 时效；局域网短码按 PRD 无有效期，传 false。
    /// `remember_session`：成功后是否把 session_id 写入黑名单；短码需可反复连接，传 false。
    pub fn connect_as_receiver(
        &self,
        serial: &str,
        password: Option<&[u8]>,
        candidates: &[String],
        ttl_secs: u64,
        files: Vec<TransferFile>,
        check_ttl: bool,
        remember_session: bool,
    ) -> Result<(String, u32), String> {
        let started = Instant::now();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // F14：TTL + 黑名单校验接线 —— 用 FSM 完成步骤 1..7
        let mut fsm = ReceiverFsm::new();
        let action = {
            let bl = self.blacklist.lock().unwrap();
            fsm.connect_with(serial, password, now, &bl, ttl_secs, check_ttl)
        }; // 及时释放黑名单锁，避免后续嵌套加锁死锁
        match action {
            ReceiverAction::SendChallenge(ch) => {
                let mut stream = self.dial(candidates)?;
                let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
                write_wire(&mut stream, &Wire {
                    t: "challenge".into(), v: to_hex(&ch), c: String::new(),
                }).map_err(|e| format!("E009: 发送质询失败 {e}"))?;
                let reply = read_wire(&mut stream)
                    .ok_or_else(|| "E009: 未收到对端应答".to_string())?;
                if reply.t == "error" {
                    return Err(format!("{}: 对端拒绝握手", reply.c));
                }
                let resp = parse_hex::<32>(&reply.v)
                    .ok_or_else(|| "E008: 应答格式错误".to_string())?;
                match fsm.verify_response(resp) {
                    ReceiverAction::Established(payload) => {
                        // 建立成功：写入黑名单（BR-02），防重放。
                        // 局域网短码按设计可反复连接，不写入，否则第二次即 E005。
                        if remember_session {
                            self.blacklist.lock().unwrap().insert(payload.session_id);
                        }
                        let rtt = started.elapsed().as_millis().min(u32::MAX as u128) as u32;
                        *self.last_rtt_ms.lock().unwrap() = rtt;
                        let addr = candidates.first().cloned().unwrap_or_default();

                        // F18–F23：连接方作为发送方，在后台线程发送文件；
                        // command 立即返回，进度通过 ll-transfer-progress 事件推送。
                        let transfer = self.transfer.clone();
                        std::thread::Builder::new()
                            .name("lanlink-sender".into())
                            .spawn(move || transfer.send_files(stream, files))
                            .ok();
                        return Ok((addr, rtt));
                    }
                    ReceiverAction::Failed(e) => return Err(format!("{}: {}", e.code, e.message)),
                    _ => return Err("E008: 握手状态异常".into()),
                }
            }
            ReceiverAction::Failed(e) => return Err(format!("{}: {}", e.code, e.message)),
            _ => return Err("E008: 握手状态异常".into()),
        }
    }

    /// F12：按候选顺序拨号（上层已按 Pub6→Pub4→Lan4→Lan6 排序），全失败返回 E009。
    fn dial(&self, candidates: &[String]) -> Result<TcpStream, String> {
        let mut last = String::new();
        for c in candidates {
            // 兼容带端口 / 不带端口的候选
            let target = if c.contains(':') && !c.starts_with('[') && c.matches(':').count() == 1 {
                c.clone()
            } else if c.starts_with('[') {
                c.clone()
            } else {
                format!("{c}:{DATA_PORT}")
            };
            let addrs = match target.to_socket_addrs() {
                Ok(a) => a.collect::<Vec<_>>(),
                Err(e) => { last = e.to_string(); continue; }
            };
            for addr in addrs {
                match TcpStream::connect_timeout(&addr, Duration::from_millis(900)) {
                    Ok(s) => return Ok(s),
                    Err(e) => last = e.to_string(),
                }
            }
        }
        Err(format!("E009: 所有候选地址均不可达（{}）", if last.is_empty() { "无候选" } else { &last }))
    }
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn parse_hex<const N: usize>(s: &str) -> Option<[u8; N]> {
    let s = s.trim();
    if s.len() != N * 2 { return None; }
    let mut out = [0u8; N];
    for i in 0..N {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

fn write_wire(stream: &mut TcpStream, w: &Wire) -> std::io::Result<()> {
    let body = serde_json::to_vec(w).map_err(|e| std::io::Error::other(e))?;
    let len = (body.len() as u32).to_be_bytes();
    stream.write_all(&len)?;
    stream.write_all(&body)?;
    stream.flush()
}

fn read_wire(stream: &mut TcpStream) -> Option<Wire> {
    let mut len = [0u8; 4];
    stream.read_exact(&mut len).ok()?;
    let n = u32::from_be_bytes(len) as usize;
    if n == 0 || n > 4096 { return None; }
    let mut body = vec![0u8; n];
    stream.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}
