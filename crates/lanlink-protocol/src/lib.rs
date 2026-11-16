//! # lanlink-protocol
//!
//! LanLink 外网邀请序列号协议层实现（PRD D3 交付物）。
//!
//! 对应《LanLink 外网邀请序列号｜产品需求文档》V1.0，覆盖：
//!
//! - 二进制 `Payload` 序列化 / 反序列化（手写，零依赖，大端网络字节序）
//! - 定制字符集 `Base32` 编解码（剔除 `0/1/I/O/L`）
//! - `CRC32` 完整性校验（明文模式）
//! - `LZ4` 自适应压缩（有机会就省，绝不亏）
//! - `AES-GCM-256` 可选口令加密（`PBKDF2-HMAC-SHA256` 派生密钥）
//! - `Blake3` 动态质询应答（防重放第三层）
//! - 收发双方显式握手状态机（五级校验固定顺序）
//!
//! ## 设计决策（与 PRD 的必要澄清，已在代码注释标注）
//!
//! 1. **单一 flags 字节位于块首且始终明文**：接收端必须在解密前读取 `ENCRYPT`
//!    位以弹出口令框（PRD 4.7 步骤 3.1），因此加密模式下块结构为
//!    `[flags(1)][nonce(12)][ciphertext][tag(16)]`，比 PRD 4.3.1 字面描述的
//!    `[nonce][ciphertext][tag]` 多一个明文 flags 字节。这是保证 UX 流程唯一自洽的方案。
//! 2. **AES-GCM 关联数据（AD）置空**：PRD 4.5 写 `AD = session_id ‖ ts`，但二者
//!    位于密文内，解密前无法取得，形成循环依赖。改为 `AD = b""`，`session_id/ts`
//!    的完整性已由 GCM Tag 覆盖，安全等价且可落地。
//! 3. **CRC32 覆盖区域**：`[flags][session_id][ts][count][candidates]`，与 PRD 4.3 一致。

pub mod base32;
pub mod codec;
pub mod crypto;
pub mod error;
pub mod handshake;
pub mod payload;

/// flags 位：bit0 = COMPRESS（Payload 经 LZ4 压缩）
pub const FLAG_COMPRESS: u8 = 0b0000_0001;
/// flags 位：bit1 = ENCRYPT（外层已 AES-GCM 加密）
pub const FLAG_ENCRYPT: u8 = 0b0000_0010;

/// 候选地址数量上限（PRD 4.3）
pub const MAX_CANDIDATES: usize = 4;

/// 序列号有效期（TTL），单位秒，默认 15 分钟（PRD 4.6）
pub const DEFAULT_TTL_SECS: u64 = 900;

/// 握手质询超时（秒），对应状态机 `S_WAIT_RESPONSE` 固定 10s（PRD 6.4）
pub const CHALLENGE_TIMEOUT_SECS: u64 = 10;

/// 协议版本（隐式 v1，预留 flags bit5~7）
pub const PROTOCOL_VERSION: u8 = 1;
