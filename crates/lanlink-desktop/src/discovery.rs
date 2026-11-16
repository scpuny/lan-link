//! 局域网设备发现（F9–F11）+ 短码查询应答（D4）。
//!
//! 自包含 UDP 实现（端口 42211，不引入 mDNS 依赖）：
//!  - 服务启动时绑定 UDP 42211：后台线程每 2s 广播一次 ANNOUNCE，监听线程处理入站报文；
//!  - `scan_lan` 立即广播一次并短暂收集，返回设备表；RTT 用 ANNOUNCE 的 `t0` 估算。
//!  - 短码（D4）：持有方把 `短码 → serial` 注册进内存表；查询方广播 `LQ(code)`，
//!    持有方回 `LA(serial)`；查询方拿到 serial 后走标准握手 FSM，短码只负责递送凭证。
//!  - 可发现开关（F10）：关闭后不广播、不应答 LQ，但仍可主动发现 / 查询别人。

use crate::net;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const DISCOVERY_PORT: u16 = 42211;
const MAGIC: &str = "LANLINK1";

/// UDP 线上报文：ANNOUNCE（设备宣告）/ LQ（短码查询）/ LA（短码应答）共用。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Packet {
    m: String, // magic
    #[serde(default)]
    k: String, // ""/"ANN"=宣告, "LQ"=查询, "LA"=应答
    #[serde(default)]
    id: String, // 发送方稳定 id
    #[serde(default)]
    n: String, // device name
    #[serde(default)]
    os: String, // platform
    #[serde(default)]
    ip: String, // Lan4
    #[serde(default)]
    port: u16, // TCP 数据端口
    #[serde(default)]
    t0: u128, // 发送时刻（Unix millis）
    #[serde(default)]
    c: String, // 短码
    #[serde(default)]
    s: String, // serial（LA 应答）
    #[serde(default)]
    q: String, // 查询 id（关联 LQ 与其 LA）
}

#[derive(Debug, Clone)]
pub struct SeenDevice {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub addr: String,
    pub latency_ms: u32,
    pub last_seen_ms: u128,
}

#[derive(Debug, Clone)]
struct LanAnswer {
    serial: String,
    addr: String,
}

#[derive(Clone)]
pub struct Discovery {
    inner: Arc<Mutex<HashMap<String, SeenDevice>>>,
    /// D4：本机持有的短码 → serial（运行期内存，退出即失效）
    codes: Arc<Mutex<HashMap<String, String>>>,
    /// D4：按 qid 归集的短码应答
    answers: Arc<Mutex<HashMap<String, Vec<LanAnswer>>>>,
    discoverable: Arc<AtomicBool>,
    self_id: String,
    self_name: String,
    self_os: String,
    self_lan4: String,
}

impl Discovery {
    /// 创建发现服务（不启动线程）。参数为本机展示信息。
    pub fn new(name: String, os: String, lan4: String) -> Self {
        let self_id = stable_id(&name);
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            codes: Arc::new(Mutex::new(HashMap::new())),
            answers: Arc::new(Mutex::new(HashMap::new())),
            discoverable: Arc::new(AtomicBool::new(true)),
            self_id,
            self_name: name,
            self_os: os,
            self_lan4: lan4,
        }
    }

    pub fn set_discoverable(&self, on: bool) {
        self.discoverable.store(on, Ordering::Relaxed);
        if !on {
            self.codes.lock().unwrap().clear();
        }
    }

    pub fn is_discoverable(&self) -> bool {
        self.discoverable.load(Ordering::Relaxed)
    }

    /// D4：注册本机持有的短码（重新生成会覆盖旧码）。
    pub fn register_code(&self, code: String, serial: String) {
        self.codes.lock().unwrap().insert(code, serial);
    }

    #[allow(dead_code)] // 预留：后续「作废短码」入口会用到；当前关闭可发现时整体清空
    pub fn unregister_code(&self, code: &str) {
        self.codes.lock().unwrap().remove(code);
    }

    /// 生成一个 ANNOUNCE 报文（JSON）。
    fn announce_json(&self) -> String {
        serde_json::to_string(&Packet {
            m: MAGIC.into(),
            k: "ANN".into(),
            id: self.self_id.clone(),
            n: self.self_name.clone(),
            os: self.self_os.clone(),
            ip: self.self_lan4.clone(),
            port: 42210,
            t0: now_millis(),
            ..Default::default()
        })
        .unwrap_or_default()
    }

    /// 向所有网卡的 IPv4 广播地址发送报文。
    fn broadcast(&self, sock: &UdpSocket, payload: &str) {
        let mut targets: Vec<SocketAddr> = Vec::new();
        for it in net::enumerate_interfaces() {
            if it.is_loopback {
                continue;
            }
            if let Some(b) = it.broadcast {
                targets.push(SocketAddr::from((b, DISCOVERY_PORT)));
            }
        }
        targets.push(SocketAddr::from((Ipv4Addr::BROADCAST, DISCOVERY_PORT)));
        targets.sort();
        targets.dedup();
        for t in targets {
            let _ = sock.send_to(payload.as_bytes(), t);
        }
    }

    /// 处理一条入站报文，返回是否成功收录为设备。
    fn on_packet(&self, payload: &str, from: SocketAddr, reply: &UdpSocket) -> bool {
        let p: Packet = match serde_json::from_str(payload) {
            Ok(v) => v,
            Err(_) => return false,
        };
        if p.m != MAGIC {
            return false;
        }
        match p.k.as_str() {
            "LQ" => {
                if !self.is_discoverable() || p.c.is_empty() {
                    return false;
                }
                let serial = self.codes.lock().unwrap().get(&p.c).cloned();
                if let Some(s) = serial {
                    let la = Packet {
                        m: MAGIC.into(),
                        k: "LA".into(),
                        id: self.self_id.clone(),
                        c: p.c,
                        s,
                        q: p.q,
                        ..Default::default()
                    };
                    if let Ok(j) = serde_json::to_string(&la) {
                        let _ = reply.send_to(j.as_bytes(), from);
                    }
                }
                false
            }
            "LA" => {
                if p.q.is_empty() || p.s.is_empty() || p.id == self.self_id {
                    return false;
                }
                self.answers.lock().unwrap().entry(p.q).or_default().push(LanAnswer {
                    serial: p.s,
                    addr: from.ip().to_string(),
                });
                false
            }
            _ => {
                if p.id == self.self_id {
                    return false;
                }
                let recv_ms = now_millis();
                let rtt = if p.t0 <= recv_ms {
                    (recv_ms - p.t0).min(u32::MAX as u128) as u32
                } else {
                    0
                };
                let addr = if p.ip.is_empty() { from.ip().to_string() } else { p.ip };
                let dev = SeenDevice {
                    id: p.id.clone(),
                    name: p.n,
                    platform: p.os,
                    addr,
                    latency_ms: rtt,
                    last_seen_ms: recv_ms,
                };
                self.inner.lock().unwrap().insert(p.id, dev);
                true
            }
        }
    }

    /// 后台服务：绑定端口、周期广播、监听回应。由调用方 spawn 保持线程存活。
    pub fn spawn(self) {
        let sock = match UdpSocket::bind((IpAddr::V4(Ipv4Addr::UNSPECIFIED), DISCOVERY_PORT)) {
            Ok(s) => s,
            Err(e) => {
                #[cfg(debug_assertions)]
                eprintln!("[lanlink-discovery] bind {DISCOVERY_PORT} failed: {e}");
                return;
            }
        };
        let _ = sock.set_broadcast(true);
        let _ = sock.set_read_timeout(Some(Duration::from_millis(300)));

        let rx = self.clone();
        let rx_sock = sock.try_clone().expect("clone udp socket");
        std::thread::Builder::new()
            .name("lanlink-discovery-rx".into())
            .spawn(move || {
                let mut buf = [0u8; 8192];
                loop {
                    match rx_sock.recv_from(&mut buf) {
                        Ok((n, from)) => {
                            if let Ok(s) = std::str::from_utf8(&buf[..n]) {
                                rx.on_packet(s, from, &rx_sock);
                            }
                        }
                        Err(ref e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                || e.kind() == std::io::ErrorKind::TimedOut => {}
                        Err(_) => std::thread::sleep(Duration::from_millis(200)),
                    }
                }
            })
            .ok();

        let tx = self;
        let tx_sock = sock;
        std::thread::Builder::new()
            .name("lanlink-discovery-tx".into())
            .spawn(move || loop {
                if tx.is_discoverable() {
                    let payload = tx.announce_json();
                    tx.broadcast(&tx_sock, &payload);
                }
                std::thread::sleep(Duration::from_secs(2));
            })
            .ok();
    }

    /// F9/F11：立即广播一次宣告，短暂收集后返回当前设备表（剔除 8s 未刷新的陈旧项）。
    pub fn scan(&self, wait: Duration) -> Vec<SeenDevice> {
        if let Ok(sock) = UdpSocket::bind((IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)) {
            let _ = sock.set_broadcast(true);
            self.broadcast(&sock, &self.announce_json());
            let _ = sock.send_to(
                self.announce_json().as_bytes(),
                SocketAddr::from((Ipv4Addr::LOCALHOST, DISCOVERY_PORT)),
            );
        }
        std::thread::sleep(wait);
        let now = now_millis();
        let stale = now.saturating_sub(8000);
        let mut map = self.inner.lock().unwrap();
        map.retain(|_, d| d.last_seen_ms >= stale);
        let mut out: Vec<SeenDevice> = map.values().cloned().collect();
        out.sort_by(|a, b| a.latency_ms.cmp(&b.latency_ms));
        out
    }

    /// D4：广播查询短码持有方，返回 `(serial, 持有方地址)`；超时返回 None。
    /// 额外向 127.0.0.1 直投一份，保证单机回环自测也能拿到应答。
    pub fn query_code(&self, code: &str, wait: Duration) -> Option<(String, String)> {
        if !self.is_discoverable() {
            return None;
        }
        let qid = {
            let b = Uuid::new_v4().into_bytes();
            format!("{:02x}{:02x}{:02x}{:02x}", b[0], b[1], b[2], b[3])
        };
        self.answers.lock().unwrap().remove(&qid);
        let sock = UdpSocket::bind((IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)).ok()?;
        let _ = sock.set_broadcast(true);
        let _ = sock.set_read_timeout(Some(Duration::from_millis(80)));
        let lq = Packet {
            m: MAGIC.into(),
            k: "LQ".into(),
            id: self.self_id.clone(),
            c: code.to_ascii_uppercase(),
            q: qid.clone(),
            ..Default::default()
        };
        let json = serde_json::to_string(&lq).ok()?;
        self.broadcast(&sock, &json);
        let _ = sock.send_to(
            json.as_bytes(),
            SocketAddr::from((Ipv4Addr::LOCALHOST, DISCOVERY_PORT)),
        );

        let deadline = Instant::now() + wait;
        let mut buf = [0u8; 8192];
        while Instant::now() < deadline {
            match sock.recv_from(&mut buf) {
                Ok((n, from)) => {
                    if let Ok(p) = serde_json::from_slice::<Packet>(&buf[..n]) {
                        if p.k == "LA" && p.q == qid && !p.s.is_empty() && p.id != self.self_id {
                            return Some((p.s, from.ip().to_string()));
                        }
                    }
                }
                Err(_) => {}
            }
        }
        // 兜底：监听线程也可能已把应答写入 answers（例如经本机回环）
        let hit = self
            .answers
            .lock()
            .unwrap()
            .get(&qid)
            .and_then(|v| v.first())
            .cloned();
        hit.map(|a| (a.serial, a.addr))
    }
}

/// D4：生成 6 位短码（30 bit 随机 → Base32 字符集，6×5=30）。
pub fn generate_code() -> String {
    let b = Uuid::new_v4().into_bytes();
    let mut v = u32::from_be_bytes([b[0], b[1], b[2], b[3]]) & 0x3FFF_FFFFu32;
    let ab = lanlink_protocol::base32::ALPHABET;
    let mut s = String::with_capacity(6);
    for _ in 0..6 {
        s.push(ab[(v & 0x1F) as usize] as char);
        v >>= 5;
    }
    s
}

/// D4：校验短码格式（6 位、字符均在 Base32 字符集内）。
pub fn valid_code(code: &str) -> bool {
    if code.len() != 6 {
        return false;
    }
    let ab = lanlink_protocol::base32::ALPHABET;
    code.bytes().all(|c| ab.contains(&c))
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// 由设备名生成稳定 id（FNV-1a 32 位，够本局域网去重）。
fn stable_id(s: &str) -> String {
    let mut h: u32 = 2166136261;
    for b in s.as_bytes() {
        h ^= *b as u32;
        h = h.wrapping_mul(16777619);
    }
    format!("ll-{h:08x}")
}
