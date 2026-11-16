//! 定制字符集 Base32 编解码（PRD 4.2）。
//!
//! 字符集（锁定，32 个符号，剔除易混淆 `0/1/I/O/L`）：
//!
//! ```text
//! ABCDEFGHJKLMNPQRSTUVWXYZ23456789
//! ```
//!
//! - 每字符承载 5 bit，输出统一大写。
//! - 无填充符（`=`），按 5-bit 边界自然截断。
//! - 解码容错由 `codec::clean_serial` 负责（去 `-`/空白、统一大写），
//!   本模块仅做纯编解码，非法字符直接报错。

/// 定制 Base32 字符集（顺序即 5-bit 值 0..31）。
pub const ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// 反向映射表：`char as usize -> 5-bit 值`，非法为 `0xFF`。
const DECODE_MAP: [u8; 256] = build_decode_map();

const fn build_decode_map() -> [u8; 256] {
    let mut map = [0xFFu8; 256];
    let mut i = 0;
    while i < 32 {
        map[ALPHABET[i] as usize] = i as u8;
        i += 1;
    }
    map
}

/// 将二进制编码为定制 Base32 字符串（无填充、大写）。
pub fn encode(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() * 8 + 4) / 5);
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;
    for &b in data {
        buffer = (buffer << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            let idx = ((buffer >> bits) & 0x1F) as usize;
            out.push(ALPHABET[idx] as char);
        }
    }
    if bits > 0 {
        // 末尾不足 5 bit 的部分，左移补零后输出一个字符
        let idx = ((buffer << (5 - bits)) & 0x1F) as usize;
        out.push(ALPHABET[idx] as char);
    }
    out
}

/// 将定制 Base32 字符串解码为二进制。
///
/// 接受已清洗（大写、去分隔符）的输入；遇到非字符集字符返回 `Err`。
pub fn decode(s: &str) -> Result<Vec<u8>, crate::error::ProtocolError> {
    let mut out = Vec::with_capacity(s.len() * 5 / 8);
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;
    for ch in s.chars() {
        let c = ch as usize;
        if c >= 256 {
            return Err(crate::error::ProtocolError::e001());
        }
        let v = DECODE_MAP[c];
        if v == 0xFF {
            return Err(crate::error::ProtocolError::e001());
        }
        buffer = (buffer << 5) | v as u32;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xFF) as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alphabet_has_32() {
        assert_eq!(ALPHABET.len(), 32);
        // 以锁定字符集字符串为准：真正被剔除的是 '0' '1' 'I' 'O'（decode 值应为 0xFF）。
        // 注：PRD 正文括注还写了「排除 L」，但锁定字符集字符串本身含 'L'，此处以字符串为准。
        for bad in ['0', '1', 'I', 'O'] {
            assert_eq!(DECODE_MAP[bad as usize], 0xFF, "char {bad} should be invalid");
        }
        // 'L' 实际保留在字符集中（索引 10）
        assert_eq!(DECODE_MAP['L' as usize], 10);
    }

    #[test]
    fn roundtrip_various_lengths() {
        for len in 0..40usize {
            let data: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
            let enc = encode(&data);
            let dec = decode(&enc).unwrap();
            assert_eq!(data, dec, "len={}", len);
        }
    }

    #[test]
    fn rejects_invalid_char() {
        assert!(decode("K7G2S90").is_err()); // 含 '0'
        assert!(decode("K7G2S1").is_err()); // 含 '1'
    }
}
