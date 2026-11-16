//! 二进制 `Payload` 结构（PRD 4.3）与（反）序列化。
//!
//! 布局（大端 / 网络字节序）：
//!
//! ```text
//! 0x00  1   flags            （位图，bit0=COMPRESS, bit1=ENCRYPT；见 lib.rs）
//! 0x01 16   session_id       UUID v4，防重放核心
//! 0x11  4   ts               u32，Unix 秒
//! 0x15  1   candidate_count u8 ≤ 4
//! 0x16  N   candidates[]     循环体
//! 末尾  4   crc32             （仅明文模式；加密模式由 GCM Tag 承担）
//! ```
//!
//! 说明：本模块只负责 `[session_id][ts][count][candidates]` 的（反）序列化
//! （`flags` 与 `crc32` 由 `codec` 统一在块首/块尾管理，避免双重 flags）。

use crate::{error::ProtocolError, MAX_CANDIDATES};
use std::io::{Error, Write};

/// 地址类型枚举（PRD 4.3.2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AddrKind {
    /// 内网 IPv4（addr 4 字节）
    Lan4 = 0,
    /// 公网 IPv4（STUN 观测端点，addr 4 字节）
    Pub4 = 1,
    /// 内网 IPv6（addr 16 字节）
    Lan6 = 2,
    /// 公网 IPv6（全球单播，addr 16 字节）
    Pub6 = 3,
}

impl AddrKind {
    /// 由 u8 解析，非法值返回 None。
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(AddrKind::Lan4),
            1 => Some(AddrKind::Pub4),
            2 => Some(AddrKind::Lan6),
            3 => Some(AddrKind::Pub6),
            _ => None,
        }
    }

    /// 该类型地址的二进制长度。
    pub fn addr_len(self) -> usize {
        match self {
            AddrKind::Lan4 | AddrKind::Pub4 => 4,
            AddrKind::Lan6 | AddrKind::Pub6 => 16,
        }
    }
}

/// 单个候选地址（PRD 4.3.2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub kind: AddrKind,
    /// 端口（大端）
    pub port: u16,
    /// 地址二进制，长度必须等于 `kind.addr_len()`
    pub addr: Vec<u8>,
}

impl Candidate {
    /// 校验 addr 长度与 kind 是否匹配。
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.addr.len() != self.kind.addr_len() {
            return Err(ProtocolError::e007());
        }
        Ok(())
    }
}

/// 邀请凭证载荷（不含 flags 与 crc，二者由 codec 管理）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvitePayload {
    pub session_id: [u8; 16],
    pub ts: u32,
    pub candidates: Vec<Candidate>,
}

impl InvitePayload {
    /// 序列化主体：`[session_id(16)][ts(4)][count(1)][candidates...]`。
    ///
    /// 不含 flags 与 crc（由 codec 层负责）。
    pub fn serialize_body(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(21 + self.candidates.len() * 19);
        buf.extend_from_slice(&self.session_id);
        buf.extend_from_slice(&self.ts.to_be_bytes());
        buf.push(self.candidates.len() as u8);
        for c in &self.candidates {
            buf.push(c.kind as u8);
            buf.extend_from_slice(&c.port.to_be_bytes());
            buf.extend_from_slice(&c.addr);
        }
        buf
    }

    /// 反序列化主体（输入不含 flags 与 crc）。
    pub fn deserialize_body(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let mut cur = std::io::Cursor::new(bytes);
        let mut read = |n: usize| -> Result<Vec<u8>, ProtocolError> {
            let mut b = vec![0u8; n];
            use std::io::Read;
            cur.read_exact(&mut b).map_err(|_| ProtocolError::e007())?;
            Ok(b)
        };

        let sid = read(16)?;
        let mut session_id = [0u8; 16];
        session_id.copy_from_slice(&sid);

        let ts_bytes = read(4)?;
        let ts = u32::from_be_bytes([ts_bytes[0], ts_bytes[1], ts_bytes[2], ts_bytes[3]]);

        let count_bytes = read(1)?;
        let count = count_bytes[0] as usize;
        if count > MAX_CANDIDATES {
            return Err(ProtocolError::e007());
        }

        let mut candidates = Vec::with_capacity(count);
        for _ in 0..count {
            let kind_byte = read(1)?[0];
            let kind = AddrKind::from_u8(kind_byte).ok_or_else(ProtocolError::e007)?;
            let port_bytes = read(2)?;
            let port = u16::from_be_bytes([port_bytes[0], port_bytes[1]]);
            let addr = read(kind.addr_len())?;
            let c = Candidate {
                kind,
                port,
                addr,
            };
            c.validate()?;
            candidates.push(c);
        }

        // 校验没有多余字节
        use std::io::Read;
        let mut tail = [0u8; 1];
        if cur.read(&mut tail).map_err(|_| ProtocolError::e007())? != 0 {
            return Err(ProtocolError::e007());
        }

        Ok(Self {
            session_id,
            ts,
            candidates,
        })
    }

    /// 计算本载荷的 CRC32 覆盖区：`[flags][session_id][ts][count][candidates]`。
    ///
    /// `flags` 字节由调用方传入（明文模式可能为 0 / COMPRESS；加密模式 ENCRYPT）。
    pub fn crc_input(flags: u8, body: &[u8]) -> Vec<u8> {
        let mut v = Vec::with_capacity(body.len() + 1);
        v.push(flags);
        v.extend_from_slice(body);
        v
    }

    /// 调试辅助：把 session_id 前 8 位转十六串（日志红线要求只打前 8 位）。
    pub fn session_short(&self) -> String {
        let mut s = String::with_capacity(16);
        for b in &self.session_id[..8] {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }
}

/// 仅为屏蔽未使用导入告警（`Write` trait 在扩展时可能用到）。
#[allow(dead_code)]
fn _assert_write(_w: &mut dyn Write) -> Result<(), Error> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> InvitePayload {
        InvitePayload {
            session_id: [0x11u8; 16],
            ts: 1_700_000_000,
            candidates: vec![
                Candidate {
                    kind: AddrKind::Pub6,
                    port: 42210,
                    addr: [0x20u8; 16].to_vec(),
                },
                Candidate {
                    kind: AddrKind::Pub4,
                    port: 42211,
                    addr: [0x30u8; 4].to_vec(),
                },
            ],
        }
    }

    #[test]
    fn body_roundtrip() {
        let p = sample();
        let body = p.serialize_body();
        let back = InvitePayload::deserialize_body(&body).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn addr_len_validation() {
        let mut p = sample();
        p.candidates[0].addr = vec![0u8; 4]; // 错误长度（Pub6 应为 16）
        assert_eq!(p.candidates[0].validate(), Err(ProtocolError::e007()));
        assert!(InvitePayload::deserialize_body(&p.serialize_body()).is_err());
    }

    #[test]
    fn count_over_limit() {
        let mut cands = Vec::new();
        for _ in 0..5 {
            cands.push(Candidate {
                kind: AddrKind::Pub4,
                port: 1,
                addr: [0u8; 4].to_vec(),
            });
        }
        let p = InvitePayload {
            session_id: [0u8; 16],
            ts: 0,
            candidates: cands,
        };
        assert_eq!(InvitePayload::deserialize_body(&p.serialize_body()), Err(ProtocolError::e007()));
    }

    #[test]
    fn trailing_bytes_rejected() {
        let p = sample();
        let mut body = p.serialize_body();
        body.push(0x99); // 多余字节
        assert_eq!(InvitePayload::deserialize_body(&body), Err(ProtocolError::e007()));
    }
}
