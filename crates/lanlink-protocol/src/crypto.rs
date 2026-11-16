//! 密码学原语（PRD 4.5 / 4.6 第三层）。
//!
//! - `PBKDF2-HMAC-SHA256`：固定盐、迭代 10000 次，派生 32 字节 AES-256 密钥。
//! - `AES-GCM-256`：可选口令加密，12 字节随机 nonce，AD 置空（见 lib.rs 设计决策 2）。
//! - `Blake3`：握手动态质询应答 `digest = Blake3(session_id ‖ challenge)`。

use crate::error::ProtocolError;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::Aes256Gcm;
use aes_gcm::Nonce;
use hmac::Hmac;
use sha2::Sha256;

/// PBKDF2 迭代次数（PRD 4.5）。
pub const PBKDF2_ITERATIONS: u32 = 10000;

/// PBKDF2 内置固定盐（PRD 4.5「内置固定盐」）。值本身非机密，仅用于抬高离线爆破成本。
pub const PBKDF2_SALT: &[u8] = b"lanlink-invite-v1-salt";

/// 由口令派生 32 字节 AES-256 密钥。
pub fn derive_key(password: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    // 固定参数（合法盐长 + 合法输出长）不会触发 InvalidLength，故 expect 安全。
    pbkdf2::pbkdf2::<Hmac<Sha256>>(password, PBKDF2_SALT, PBKDF2_ITERATIONS, &mut key)
        .expect("PBKDF2 with fixed valid parameters");
    key
}

/// AES-GCM-256 加密。
///
/// 返回 `nonce(12) ++ ciphertext ++ tag(16)`（即 `aes-gcm` crate 的 `encrypt` 输出格式，
/// tag 已自动追加在密文末尾）。
pub fn aes_gcm_encrypt(
    key: &[u8; 32],
    nonce: &[u8; 12],
    aad: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, ProtocolError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| ProtocolError::e003())?;
    let n = Nonce::from_slice(nonce);
    cipher
        .encrypt(n, Payload { msg: plaintext, aad })
        .map_err(|_| ProtocolError::e003())
}

/// AES-GCM-256 解密。`ciphertext` 须为 `ciphertext ++ tag(16)`。
pub fn aes_gcm_decrypt(
    key: &[u8; 32],
    nonce: &[u8; 12],
    aad: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, ProtocolError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| ProtocolError::e003())?;
    let n = Nonce::from_slice(nonce);
    cipher
        .decrypt(n, Payload { msg: ciphertext, aad })
        .map_err(|_| ProtocolError::e003())
}

/// 握手质询应答：`Blake3(session_id ‖ challenge)`（PRD 4.6 第三层）。
pub fn challenge_response(session_id: &[u8; 16], challenge: &[u8; 16]) -> [u8; 32] {
    let mut input = Vec::with_capacity(32);
    input.extend_from_slice(session_id);
    input.extend_from_slice(challenge);
    let h = blake3::hash(&input);
    let mut out = [0u8; 32];
    out.copy_from_slice(h.as_bytes());
    out
}

/// 生成一次性 16 字节随机 challenge（PRD 4.6）。
pub fn random_challenge() -> [u8; 16] {
    use rand::Rng;
    rand::thread_rng().gen()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_derivation_deterministic() {
        assert_eq!(derive_key(b"827451"), derive_key(b"827451"));
        assert_ne!(derive_key(b"827451"), derive_key(b"000000"));
    }

    #[test]
    fn aes_gcm_roundtrip() {
        let key = derive_key(b"827451");
        let nonce = [7u8; 12];
        let pt = b"secret-invite-payload";
        let ct = aes_gcm_encrypt(&key, &nonce, b"", pt).unwrap();
        assert_ne!(&ct, pt);
        let dec = aes_gcm_decrypt(&key, &nonce, b"", &ct).unwrap();
        assert_eq!(dec, pt);
    }

    #[test]
    fn aes_gcm_wrong_key_fails() {
        let k1 = derive_key(b"827451");
        let k2 = derive_key(b"999999");
        let nonce = [7u8; 12];
        let ct = aes_gcm_encrypt(&k1, &nonce, b"", b"data").unwrap();
        assert!(aes_gcm_decrypt(&k2, &nonce, b"", &ct).is_err());
    }

    #[test]
    fn challenge_response_deterministic_and_one_way() {
        let sid = [1u8; 16];
        let ch = [2u8; 16];
        let r1 = challenge_response(&sid, &ch);
        let r2 = challenge_response(&sid, &ch);
        assert_eq!(r1, r2);
        let ch2 = [3u8; 16];
        assert_ne!(r1, challenge_response(&sid, &ch2));
    }
}
