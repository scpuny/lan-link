//! 错误码与提示文案（PRD 第 5 节）。
//!
//! 所有失败分支携带 `code`（E001~E009）+ `message`（中文用户提示）。
//! UI 仅按 `code` 映射文案与颜色，**禁止前端硬编码判断**。

/// 协议层错误。包含错误码与用户提示文案。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolError {
    /// 错误码，如 `"E001"`
    pub code: &'static str,
    /// 用户可见中文提示（与 PRD 第 5 节一致）
    pub message: &'static str,
}

impl ProtocolError {
    pub fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }

    // ---- 构造器：每个错误码对应 PRD 表 5 ----

    /// E001 Base32 解码失败
    pub fn e001() -> Self {
        Self::new("E001", "序列号损坏或复制不全，请重新复制")
    }
    /// E002 CRC32 不匹配（明文模式内容被篡改）
    pub fn e002() -> Self {
        Self::new("E002", "序列号内容被篡改，连接已拒绝")
    }
    /// E003 GCM Tag 校验失败（口令错误或序列号被篡改）
    pub fn e003() -> Self {
        Self::new("E003", "口令错误或序列号被篡改")
    }
    /// E004 TTL 超时
    pub fn e004() -> Self {
        Self::new("E004", "该序列号已过期，请重新生成")
    }
    /// E005 session 黑名单命中
    pub fn e005() -> Self {
        Self::new("E005", "该序列号已完成握手，不可重复使用")
    }
    /// E006 质询超时（10s）
    pub fn e006() -> Self {
        Self::new("E006", "握手质询超时，请检查网络后重试")
    }
    /// E007 Payload 字段非法
    pub fn e007() -> Self {
        Self::new("E007", "序列号字段非法，无法解析")
    }
    /// E008 应答摘要不匹配
    pub fn e008() -> Self {
        Self::new("E008", "握手质询校验失败，连接终止")
    }
    /// E009 凭证合法但 TCP 不通
    pub fn e009() -> Self {
        Self::new(
            "E009",
            "凭证校验成功，但无法连接远端地址；IPv4 可能受运营商 CGNAT 限制",
        )
    }
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ProtocolError {}

/// 由错误码取用户提示文案（供 UI 按 code 映射）。
pub fn user_message(code: &str) -> &'static str {
    match code {
        "E001" => "序列号损坏或复制不全，请重新复制",
        "E002" => "序列号内容被篡改，连接已拒绝",
        "E003" => "口令错误或序列号被篡改",
        "E004" => "该序列号已过期，请重新生成",
        "E005" => "该序列号已完成握手，不可重复使用",
        "E006" => "握手质询超时，请检查网络后重试",
        "E007" => "序列号字段非法，无法解析",
        "E008" => "握手质询校验失败，连接终止",
        "E009" => "凭证校验成功，但无法连接远端地址；IPv4 可能受运营商 CGNAT 限制",
        _ => "未知错误",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_message_consistency() {
        for code in ["E001", "E002", "E003", "E004", "E005", "E006", "E007", "E008", "E009"] {
            assert!(!user_message(code).is_empty());
        }
        assert_eq!(ProtocolError::e004().code, "E004");
        assert_eq!(ProtocolError::e004().message, user_message("E004"));
    }
}
