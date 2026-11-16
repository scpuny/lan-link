//! 候选地址采集（F5–F8）：网卡枚举、STUN 观测端点、IPv6 全球单播判定、候选优先级截断。
//!
//! 纯 std 实现，STUN 用最小 RFC5389 Binding 请求，不引入重型依赖。
//! 采集结果供 `get_self_info` 与 `generate_serial` 复用。

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::time::Duration;

/// 本机一张网卡的采集结果。
#[derive(Debug, Clone)]
pub struct NetIf {
    pub ip: IpAddr,
    /// IPv4 网卡的子网广播地址（用于 LAN 发现），IPv6 为 None
    pub broadcast: Option<Ipv4Addr>,
    pub is_loopback: bool,
}

/// F5：枚举所有网卡（IPv4 + IPv6），附带 IPv4 子网广播地址。
pub fn enumerate_interfaces() -> Vec<NetIf> {
    let mut out = Vec::new();
    if let Ok(ifaces) = if_addrs::get_if_addrs() {
        for it in ifaces {
            match it.addr {
                if_addrs::IfAddr::V4(v4) => out.push(NetIf {
                    ip: IpAddr::V4(v4.ip),
                    broadcast: v4.broadcast,
                    is_loopback: v4.ip.is_loopback(),
                }),
                if_addrs::IfAddr::V6(v6) => out.push(NetIf {
                    ip: IpAddr::V6(v6.ip),
                    broadcast: None,
                    is_loopback: v6.ip.is_loopback(),
                }),
            }
        }
    }
    out
}

/// F7：IPv6 是否全球单播（2000::/3，排除 ULA fc00::/7 / link-local fe80::/10）。
pub fn is_global_v6(ip: &Ipv6Addr) -> bool {
    let o = ip.octets();
    // 2000::/3 → 最高 3 位为 001，即首字节 0x20..=0x3f
    (0x20..=0x3f).contains(&o[0])
}

/// ULA（fc00::/7）：首字节 0xfc 或 0xfd。
pub fn is_ula_v6(ip: &Ipv6Addr) -> bool {
    let o = ip.octets();
    o[0] & 0xfe == 0xfc
}

/// 私网 IPv4（RFC1918）：10/8、172.16/12、192.168/16。
pub fn is_private_v4(ip: &Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 10
        || (o[0] == 172 && (16..=31).contains(&o[1]))
        || (o[0] == 192 && o[1] == 168)
}

/// F6：STUN Binding 请求，解析 XOR-MAPPED-ADDRESS 得到公网映射端点。
/// 超时快速失败（默认 1.2s），离线时返回 None，不阻塞 UI。
pub fn stun_public_endpoint(
    server: &str,
    timeout: Duration,
) -> Option<SocketAddr> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.set_read_timeout(Some(timeout)).ok()?;
    sock.set_write_timeout(Some(timeout)).ok()?;
    let server_addr = resolve_first(server)?;
    sock.connect(server_addr).ok()?;

    // RFC5389 Binding Request：20B 头 + 8B transaction-id（用时间戳+计数器填充）
    let mut req = [0u8; 20];
    req[0] = 0x00;
    req[1] = 0x01; // message type = binding request
    req[2] = 0x00;
    req[3] = 0x00; // message length = 0
    req[4..8].copy_from_slice(&0x2112_A442u32.to_be_bytes()); // magic cookie
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos() as u64;
    req[8..16].copy_from_slice(&seed.to_be_bytes());
    req[16..20].copy_from_slice(&(seed as u32).wrapping_mul(2654435761).to_be_bytes());

    sock.send(&req).ok()?;
    let mut buf = [0u8; 1500];
    let n = sock.recv(&mut buf).ok()?;
    if n < 20 {
        return None;
    }
    // 校验 magic cookie
    if &buf[4..8] != &0x2112_A442u32.to_be_bytes() {
        return None;
    }
    let mut i = 20usize;
    while i + 4 <= n {
        let attr_type = u16::from_be_bytes([buf[i], buf[i + 1]]);
        let len = u16::from_be_bytes([buf[i + 2], buf[i + 3]]) as usize;
        if i + 4 + len > n {
            break;
        }
        if attr_type == 0x0020 && len >= 8 {
            // XOR-MAPPED-ADDRESS: [reserved][family][x-port(2)][x-addr(4|16)]
            let family = buf[i + 5];
            let xport = u16::from_be_bytes([buf[i + 6], buf[i + 7]]) ^ 0x2112;
            if family == 0x01 && len >= 8 {
                let xip = &buf[i + 8..i + 12];
                let ip = Ipv4Addr::new(
                    xip[0] ^ 0x21,
                    xip[1] ^ 0x12,
                    xip[2] ^ 0xa4,
                    xip[3] ^ 0x42,
                );
                return Some(SocketAddr::from((ip, xport)));
            }
        }
        // 属性按 4 字节对齐
        i += 4 + ((len + 3) / 4) * 4;
    }
    None
}

fn resolve_first(host_port: &str) -> Option<SocketAddr> {
    use std::net::ToSocketAddrs;
    host_port.to_socket_addrs().ok()?.next()
}

/// 出口 IPv4（UDP connect 查路由表，不实际发包）。
pub fn outbound_ipv4() -> Option<Ipv4Addr> {
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("8.8.8.8:80").ok()?;
    match s.local_addr().ok()?.ip() {
        IpAddr::V4(v) => Some(v),
        _ => None,
    }
}

/// 出口 IPv6。
pub fn outbound_ipv6() -> Option<Ipv6Addr> {
    let s = UdpSocket::bind("[::]:0").ok()?;
    s.connect("[2001:4860:4860::8888]:80").ok()?;
    match s.local_addr().ok()?.ip() {
        IpAddr::V6(v) if !v.is_loopback() && !is_ula_v6(&v) => Some(v),
        _ => None,
    }
}
