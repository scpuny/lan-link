//! 编解码主流水线（PRD 4.1 / 4.4 / 4.5 / 4.7）。
//!
//! 块结构（单一 flags 字节位于块首，始终明文 —— 见 lib.rs 设计决策 1）：
//!
//! ```text
//! 明文模式（无压缩）: [flags(1,=0)][session_id(16)][ts(4)][count(1)][candidates][crc32(4)]
//! 明文模式（压缩）  : [flags(1,COMPRESS)][lz4([flags0][sid][ts][count][cands][crc])]
//! 加密模式         : [flags(1,ENCRYPT)][nonce(12)][ciphertext][tag(16)]
//!                    其中 ciphertext = AES-GCM( key, nonce, AD="", [flags0][sid][ts][count][cands][crc] )
//! ```
//!
//! 其中 `[flags0][...]` 为明文模式下相同的「flags0 + 主体 + crc」字节流，加密时作为明文整体加密。

use crate::base32;
use crate::crypto;
use crate::error::ProtocolError;
use crate::payload::InvitePayload;
use crate::{FLAG_COMPRESS, FLAG_ENCRYPT, MAX_CANDIDATES};
use crc32fast::Hasher as CrcHasher;
use lz4_flex::block::{compress, decompress};
use rand::Rng;

/// LZ4 解压最大展开上限，防止恶意构造的块耗尽内存（实际载荷 < 200B）。
const MAX_DECOMPRESSED: usize = 64 * 1024;

/// 构造「flags0 + 主体 + crc」明文字节流（压缩/加密的输入）。
fn build_plaintext(flags0: u8, payload: &InvitePayload) -> Vec<u8> {
    let body = payload.serialize_body();
    let crc_region = InvitePayload::crc_input(flags0, &body);
    let mut h = CrcHasher::new();
    h.update(&crc_region);
    let crc = h.finalize();
    let mut pt = crc_region;
    pt.extend_from_slice(&crc.to_be_bytes());
    pt
}

/// 校验 `[flags0][主体][crc]` 明文流的 CRC，并返回解析出的 `InvitePayload`。
fn verify_and_parse(plaintext: &[u8]) -> Result<InvitePayload, ProtocolError> {
    if plaintext.len() < 1 + 21 + 4 {
        // 至少 flags(1)+sid(16)+ts(4)+count(1) + crc(4)
        return Err(ProtocolError::e002());
    }
    let n = plaintext.len();
    let crc_region = &plaintext[..n - 4];
    let expected = u32::from_be_bytes([
        plaintext[n - 4],
        plaintext[n - 3],
        plaintext[n - 2],
        plaintext[n - 1],
    ]);

    let mut h = CrcHasher::new();
    h.update(crc_region);
    if h.finalize() != expected {
        return Err(ProtocolError::e002());
    }

    // flags0 在 plaintext[0]，主体在 [1..n-4]
    InvitePayload::deserialize_body(&plaintext[1..n - 4])
}

/// 编码为二进制块（在 Base32 之前）。
///
/// - `password = None` → 明文模式 + LZ4 自适应压缩。
/// - `password = Some(pw)` → AES-GCM-256 加密模式（不做压缩，因密文已随机）。
pub fn encode_block(payload: &InvitePayload, password: Option<&[u8]>) -> Vec<u8> {
    match password {
        None => {
            // 明文：先尝试压缩，按 4.4 自适应判定
            let plaintext = build_plaintext(0, payload);
            let compressed = compress(&plaintext);
            if compressed.len() < plaintext.len() {
                let mut block = Vec::with_capacity(compressed.len() + 1);
                block.push(FLAG_COMPRESS);
                block.extend_from_slice(&compressed);
                block
            } else {
                let mut block = Vec::with_capacity(plaintext.len() + 1);
                block.push(0);
                block.extend_from_slice(&plaintext);
                block
            }
        }
        Some(pw) => {
            let plaintext = build_plaintext(0, payload); // 加密模式 flags0 = 0
            let key = crypto::derive_key(pw);
            let mut nonce = [0u8; 12];
            rand::thread_rng().fill(&mut nonce);
            let ciphertext = crypto::aes_gcm_encrypt(&key, &nonce, b"", &plaintext)
                .expect("AES-GCM encrypt must succeed with valid 32-byte key");
            let mut block = Vec::with_capacity(1 + 12 + ciphertext.len());
            block.push(FLAG_ENCRYPT);
            block.extend_from_slice(&nonce);
            block.extend_from_slice(&ciphertext);
            block
        }
    }
}

/// 解码二进制块（Base32 之后），完成 CRC/GCM 校验与 Payload 解析。
///
/// - 加密块需要提供 `password`，否则返回 `E003`。
/// - 返回解析出的 `InvitePayload`；校验失败返回对应错误码。
pub fn decode_block(block: &[u8], password: Option<&[u8]>) -> Result<InvitePayload, ProtocolError> {
    if block.is_empty() {
        return Err(ProtocolError::e001());
    }
    let flags = block[0];
    let body = &block[1..];

    let plaintext = if flags & FLAG_ENCRYPT != 0 {
        // 加密模式
        let pw = password.ok_or_else(ProtocolError::e003)?;
        if body.len() < 12 + 16 {
            return Err(ProtocolError::e003());
        }
        let (nonce, ciphertext) = body.split_at(12);
        let mut n = [0u8; 12];
        n.copy_from_slice(nonce);
        let key = crypto::derive_key(pw);
        crypto::aes_gcm_decrypt(&key, &n, b"", ciphertext).map_err(|_| ProtocolError::e003())?
    } else if flags & FLAG_COMPRESS != 0 {
        // 压缩明文
        decompress(body, MAX_DECOMPRESSED).map_err(|_| ProtocolError::e002())?
    } else {
        // 未压缩明文
        body.to_vec()
    };

    verify_and_parse(&plaintext)
}

/// 将二进制块转换为展示用序列号字符串：Base32 大写 + 每 6 字符插入 `-`。
pub fn to_serial(block: &[u8]) -> String {
    let b32 = base32::encode(block);
    let mut out = String::with_capacity(b32.len() + b32.len() / 6);
    for (i, c) in b32.chars().enumerate() {
        if i != 0 && i % 6 == 0 {
            out.push('-');
        }
        out.push(c);
    }
    out
}

/// 序列号反清洗（PRD 4.7 步骤 1）：去 `-`/空格/制表/换行/回车，统一大写，再 Base32 解码。
///
/// 容错清洗不报错、不阻断；仅当清洗后仍含非法字符或解码失败才返回 `E001`。
pub fn from_serial(serial: &str) -> Result<Vec<u8>, ProtocolError> {
    let cleaned: String = serial
        .chars()
        .filter(|c| !matches!(c, '-' | ' ' | '\t' | '\n' | '\r'))
        .map(|c| c.to_ascii_uppercase())
        .collect();
    base32::decode(&cleaned)
}

/// 一站式编码：payload → 序列号字符串。
pub fn encode_serial(payload: &InvitePayload, password: Option<&[u8]>) -> String {
    to_serial(&encode_block(payload, password))
}

/// 一站式解码：序列号字符串 → 校验后的 Payload。
pub fn decode_serial(serial: &str, password: Option<&[u8]>) -> Result<InvitePayload, ProtocolError> {
    let block = from_serial(serial)?;
    decode_block(&block, password)
}

/// 生成候选地址采集后的示例 payload（供应用层调用，本层仅消费）。
///
/// `session_id` 必须由调用方保证 UUID v4（PRD BR-04：禁止自增/时间戳派生）。
pub fn make_payload(session_id: [u8; 16], ts: u32, candidates: Vec<crate::payload::Candidate>) -> InvitePayload {
    debug_assert!(candidates.len() <= MAX_CANDIDATES);
    InvitePayload {
        session_id,
        ts,
        candidates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::{AddrKind, Candidate};

    fn pub6_payload() -> InvitePayload {
        InvitePayload {
            session_id: [0xABu8; 16],
            ts: 1_700_000_000,
            candidates: vec![Candidate {
                kind: AddrKind::Pub6,
                port: 42210,
                addr: [0x20u8; 16].to_vec(),
            }],
        }
    }

    #[test]
    fn plaintext_roundtrip() {
        let p = pub6_payload();
        let serial = encode_serial(&p, None);
        assert!(serial.contains('-'));
        let back = decode_serial(&serial, None).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn encrypted_roundtrip() {
        let p = pub6_payload();
        let serial = encode_serial(&p, Some(b"827451"));
        // 加密串不应出现明文 IP 痕迹：至少能解码且内容一致
        let back = decode_serial(&serial, Some(b"827451")).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn wrong_password_fails() {
        let p = pub6_payload();
        let serial = encode_serial(&p, Some(b"827451"));
        // 无口令或错误口令 → E003
        assert_eq!(decode_serial(&serial, None), Err(ProtocolError::e003()));
        assert_eq!(decode_serial(&serial, Some(b"000000")), Err(ProtocolError::e003()));
    }

    #[test]
    fn missing_dash_and_lowercase_tolerated() {
        let p = pub6_payload();
        let serial = encode_serial(&p, None).to_lowercase().replace('-', " ");
        let back = decode_serial(&serial, None).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn corrupted_serial_invalid_char_e001() {
        // 引入非法字符 '0' → Base32 解码失败 → E001
        let p = pub6_payload();
        let serial = encode_serial(&p, None);
        let bad = format!("{serial}0");
        assert_eq!(decode_serial(&bad, None), Err(ProtocolError::e001()));
    }

    #[test]
    fn truncated_serial_is_rejected() {
        // 截断后解码为短块，CRC / 长度校验失败（E001 或 E002 均表示失败）
        let p = pub6_payload();
        let serial = encode_serial(&p, None);
        let half = &serial[..serial.len() / 2];
        assert!(decode_serial(half, None).is_err());
    }

    #[test]
    fn tampered_plaintext_e002() {
        let p = pub6_payload();
        let mut block = encode_block(&p, None);
        // 翻转主体某个字节（非 flags），明文模式 CRC 必失败
        let idx = block.len() / 2;
        block[idx] ^= 0xFF;
        assert_eq!(decode_block(&block, None), Err(ProtocolError::e002()));
    }

    #[test]
    fn format_grouping_is_six() {
        let p = pub6_payload();
        let serial = encode_serial(&p, None);
        let groups: Vec<&str> = serial.split('-').collect();
        assert!(!groups.is_empty());
        // 除最后一组外，每组必须恰好 6 字符（PRD 4.2）
        for g in &groups[..groups.len() - 1] {
            assert_eq!(g.len(), 6, "group '{}' len {}", g, g.len());
        }
        // 末组长度 1~6（总长度不保证被 6 整除）
        assert!((1..=6).contains(&groups.last().unwrap().len()));
    }
}
