//! 握手状态机（PRD 第 6 节）。
//!
//! 收发双方均使用显式 `enum State` + 显式转换函数（禁用隐式跳转）。
//! 网络 I/O 不在协议层职责内：状态机通过返回值 [`ReceiverAction`] / [`InitiatorAction`]
//! 把「要发送什么 / 是否建立 / 是否失败」交给上层传输层驱动。
//!
//! 五级校验固定顺序（PRD 4.7）：
//! `清洗 → Base32 → CRC/GCM → Payload → TTL → session 黑名单 → 质询 → 应答比对`。

use crate::codec;
use crate::crypto;
use crate::error::ProtocolError;
use crate::payload::InvitePayload;
use std::collections::HashSet;

// ─────────────────────────────────────────────────────────────
// 接收方（B 端）
// ─────────────────────────────────────────────────────────────

/// 接收方状态枚举（PRD 6.1，完整保留）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverState {
    SIdle,
    SParseClean,
    SVerifyCrypto,
    SParsePayload,
    SCheckTtl,
    SCheckSessionBlack,
    SSendChallenge,
    SWaitResponse,
    SVerifyResponse,
    SEstablished,
    SFailed,
}

/// 接收方动作（状态机对上层返回的指令）。
#[derive(Debug, Clone)]
pub enum ReceiverAction {
    /// 已生成 challenge，请将其发送给发起方。
    SendChallenge([u8; 16]),
    /// 握手成功，通道可建立；调用方此时应将 session_id 写入 `seen_session`。
    Established(InvitePayload),
    /// 失败，携带错误码。
    Failed(ProtocolError),
}

/// 接收方握手状态机。
#[derive(Debug, Clone)]
pub struct ReceiverFsm {
    pub state: ReceiverState,
    invite: Option<InvitePayload>,
    challenge: Option<[u8; 16]>,
}

impl ReceiverFsm {
    pub fn new() -> Self {
        Self {
            state: ReceiverState::SIdle,
            invite: None,
            challenge: None,
        }
    }

    /// 用户粘贴序列号并点击连接：执行步骤 1~7（清洗→Base32→校验→解析→TTL→黑名单→下发挑战）。
    ///
    /// 成功则进入 `SWaitResponse` 并返回 `SendChallenge`；任何环节失败直接进入 `SFailed`。
    pub fn connect(
        &mut self,
        serial: &str,
        password: Option<&[u8]>,
        now_secs: u64,
        blacklist: &HashSet<[u8; 16]>,
        ttl_secs: u64,
    ) -> ReceiverAction {
        self.connect_with(serial, password, now_secs, blacklist, ttl_secs, true)
    }

    /// 与 [`connect`] 相同的校验链，但可关闭 TTL 时效窗口。
    ///
    /// 局域网短码（D4）按 PRD 不设有效期、可反复使用，其底层 serial 虽带 `ts`，
    /// 但语义上不应触发 E004；外网序列号仍必须保留 TTL 校验。
    pub fn connect_with(
        &mut self,
        serial: &str,
        password: Option<&[u8]>,
        now_secs: u64,
        blacklist: &HashSet<[u8; 16]>,
        ttl_secs: u64,
        check_ttl: bool,
    ) -> ReceiverAction {
        // 步骤 1-2：清洗 + Base32 解码
        self.state = ReceiverState::SParseClean;
        let block = match codec::from_serial(serial) {
            Ok(b) => b,
            Err(e) => return self.fail(e),
        };

        // 步骤 3-4：CRC/GCM 校验 + Payload 解析
        self.state = ReceiverState::SVerifyCrypto;
        let invite = match codec::decode_block(&block, password) {
            Ok(p) => p,
            Err(e) => return self.fail(e),
        };
        self.state = ReceiverState::SParsePayload;

        // 步骤 5：TTL 时效窗口（局域网短码不检查）
        self.state = ReceiverState::SCheckTtl;
        let elapsed = now_secs as i64 - invite.ts as i64;
        if check_ttl && elapsed > ttl_secs as i64 {
            return self.fail(ProtocolError::e004());
        }

        // 步骤 6：session 内存黑名单
        self.state = ReceiverState::SCheckSessionBlack;
        if blacklist.contains(&invite.session_id) {
            return self.fail(ProtocolError::e005());
        }

        // 步骤 7：下发一次性 challenge
        self.state = ReceiverState::SSendChallenge;
        let challenge = crypto::random_challenge();
        self.invite = Some(invite);
        self.challenge = Some(challenge);
        self.state = ReceiverState::SWaitResponse;
        ReceiverAction::SendChallenge(challenge)
    }

    /// 收到发起方回传的应答摘要：步骤 8-9（应答比对）。
    pub fn verify_response(&mut self, response: [u8; 32]) -> ReceiverAction {
        if self.state != ReceiverState::SWaitResponse {
            return self.fail(ProtocolError::e008());
        }
        self.state = ReceiverState::SVerifyResponse;
        let (sid, ch) = match (self.invite.as_ref(), self.challenge.as_ref()) {
            (Some(i), Some(c)) => (i.session_id, *c),
            _ => return self.fail(ProtocolError::e008()),
        };
        let expected = crypto::challenge_response(&sid, &ch);
        if expected == response {
            self.state = ReceiverState::SEstablished;
            let payload = self.invite.clone().expect("invite present after challenge");
            ReceiverAction::Established(payload)
        } else {
            self.fail(ProtocolError::e008())
        }
    }

    /// 任意阶段取消：中断、释放 challenge、回到 Idle（PRD BR-03）。
    pub fn cancel(&mut self) {
        self.state = ReceiverState::SIdle;
        self.invite = None;
        self.challenge = None;
    }

    fn fail(&mut self, e: ProtocolError) -> ReceiverAction {
        self.state = ReceiverState::SFailed;
        self.challenge = None;
        ReceiverAction::Failed(e)
    }
}

impl Default for ReceiverFsm {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────
// 发起方（A 端）
// ─────────────────────────────────────────────────────────────

/// 发起方状态枚举（PRD 6.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitiatorState {
    AIdle,
    ADial,
    AWaitChallenge,
    AGenResp,
    ASendResp,
    ATrans,
    AFailed,
}

/// 发起方动作。
#[derive(Debug, Clone)]
pub enum InitiatorAction {
    /// 已生成序列号，等待被连接。
    AwaitConnection,
    /// 已收到 challenge，请将其返回给发起方（上层透传给接收方）。
    SendResponse([u8; 32]),
    /// 握手成功，进入传输。
    Established,
    /// 失败（如超时、候选地址不可达）。
    Failed(ProtocolError),
}

/// 发起方握手状态机。
#[derive(Debug, Clone)]
pub struct InitiatorFsm {
    pub state: InitiatorState,
    payload: Option<InvitePayload>,
    challenge: Option<[u8; 16]>,
}

impl InitiatorFsm {
    pub fn new() -> Self {
        Self {
            state: InitiatorState::AIdle,
            payload: None,
            challenge: None,
        }
    }

    /// 生成序列号后等待被连接。
    pub fn on_generated(&mut self, payload: InvitePayload) -> InitiatorAction {
        self.payload = Some(payload);
        self.state = InitiatorState::ADial;
        InitiatorAction::AwaitConnection
    }

    /// 收到连接请求。
    pub fn on_incoming_connect(&mut self) -> InitiatorAction {
        if self.state != InitiatorState::ADial {
            return self.fail(ProtocolError::e008());
        }
        self.state = InitiatorState::AWaitChallenge;
        InitiatorAction::AwaitConnection // 静默等待 challenge（与 AwaitConnection 共享语义）
    }

    /// 收到 challenge：计算 `Blake3(session_id ‖ challenge)` 应答摘要。
    pub fn on_challenge(&mut self, challenge: [u8; 16]) -> InitiatorAction {
        if self.state != InitiatorState::AWaitChallenge {
            return self.fail(ProtocolError::e008());
        }
        self.state = InitiatorState::AGenResp;
        let sid = match self.payload.as_ref() {
            Some(p) => p.session_id,
            None => return self.fail(ProtocolError::e008()),
        };
        let response = crypto::challenge_response(&sid, &challenge);
        self.challenge = Some(challenge);
        self.state = InitiatorState::ASendResp;
        InitiatorAction::SendResponse(response)
    }

    /// 对方校验通过（应答被接收方接受）。
    pub fn on_response_accepted(&mut self) -> InitiatorAction {
        if self.state != InitiatorState::ASendResp {
            return self.fail(ProtocolError::e008());
        }
        self.state = InitiatorState::ATrans;
        InitiatorAction::Established
    }

    /// 探测候选地址全部不可达（PRD 6.3），或超时。
    pub fn mark_unreachable(&mut self) -> InitiatorAction {
        self.fail(ProtocolError::e009())
    }

    /// 任意阶段取消。
    pub fn cancel(&mut self) {
        self.state = InitiatorState::AIdle;
        self.payload = None;
        self.challenge = None;
    }

    fn fail(&mut self, e: ProtocolError) -> InitiatorAction {
        self.state = InitiatorState::AFailed;
        self.challenge = None;
        InitiatorAction::Failed(e)
    }
}

impl Default for InitiatorFsm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::{AddrKind, Candidate};
    use crate::codec;

    fn payload() -> InvitePayload {
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
    fn receiver_full_happy_path() {
        let p = payload();
        let serial = codec::encode_serial(&p, None);
        let mut fsm = ReceiverFsm::new();
        let mut blacklist = HashSet::new();

        // 步骤 1-7
        let action = fsm.connect(&serial, None, p.ts as u64 + 10, &blacklist, 900);
        let challenge = match action {
            ReceiverAction::SendChallenge(c) => c,
            other => panic!("expected SendChallenge, got {:?}", other),
        };
        assert_eq!(fsm.state, ReceiverState::SWaitResponse);

        // 步骤 8-9：发起方计算应答并回传
        let response = crypto::challenge_response(&p.session_id, &challenge);
        let action = fsm.verify_response(response);
        match action {
            ReceiverAction::Established(got) => assert_eq!(got, p),
            other => panic!("expected Established, got {:?}", other),
        }
        assert_eq!(fsm.state, ReceiverState::SEstablished);
        // 建立成功 → 应用层写入黑名单
        blacklist.insert(p.session_id);
    }

    #[test]
    fn receiver_wrong_response_e008() {
        let p = payload();
        let serial = codec::encode_serial(&p, None);
        let mut fsm = ReceiverFsm::new();
        let blacklist = HashSet::new();
        let _challenge = match fsm.connect(&serial, None, p.ts as u64 + 10, &blacklist, 900) {
            ReceiverAction::SendChallenge(c) => c,
            _ => panic!(),
        };
        let mut bad = [0u8; 32];
        bad[0] = 0xFF;
        let action = fsm.verify_response(bad);
        match action {
            ReceiverAction::Failed(e) => assert_eq!(e.code, "E008"),
            other => panic!("expected Failed E008, got {:?}", other),
        }
    }

    #[test]
    fn receiver_ttl_expired_e004() {
        let p = payload();
        let serial = codec::encode_serial(&p, None);
        let mut fsm = ReceiverFsm::new();
        let blacklist = HashSet::new();
        let action = fsm.connect(&serial, None, p.ts as u64 + 1000, &blacklist, 900);
        match action {
            ReceiverAction::Failed(e) => assert_eq!(e.code, "E004"),
            other => panic!("expected Failed E004, got {:?}", other),
        }
    }

    #[test]
    fn receiver_blacklist_e005() {
        let p = payload();
        let serial = codec::encode_serial(&p, None);
        let mut fsm = ReceiverFsm::new();
        let mut blacklist = HashSet::new();
        blacklist.insert(p.session_id);
        let action = fsm.connect(&serial, None, p.ts as u64 + 10, &blacklist, 900);
        match action {
            ReceiverAction::Failed(e) => assert_eq!(e.code, "E005"),
            other => panic!("expected Failed E005, got {:?}", other),
        }
    }

    #[test]
    fn initiator_roundtrip() {
        let p = payload();
        let mut a = InitiatorFsm::new();
        assert!(matches!(a.on_generated(p.clone()), InitiatorAction::AwaitConnection));
        assert!(matches!(a.on_incoming_connect(), InitiatorAction::AwaitConnection));
        let challenge = [9u8; 16];
        let resp = match a.on_challenge(challenge) {
            InitiatorAction::SendResponse(r) => r,
            _ => panic!(),
        };
        assert_eq!(resp, crypto::challenge_response(&p.session_id, &challenge));
        assert!(matches!(a.on_response_accepted(), InitiatorAction::Established));
    }

    /// 回归：局域网短码复用带 ts 的 serial，但按 PRD 无有效期，
    /// 即使时间戳远超 TTL 也不得判 E004。
    #[test]
    fn lan_code_skips_ttl() {
        let p = payload();
        let serial = codec::encode_serial(&p, None);
        let mut fsm = ReceiverFsm::new();
        let blacklist = HashSet::new();
        // 时间推进一天，远超 900s TTL
        let far_future = p.ts as u64 + 86400;
        let action = fsm.connect_with(&serial, None, far_future, &blacklist, 900, false);
        match action {
            ReceiverAction::SendChallenge(_) => {}
            other => panic!("short code must ignore TTL, got {:?}", other),
        }
        // 对照组：外网序列号在同一时刻必须被判过期
        let mut serial_fsm = ReceiverFsm::new();
        match serial_fsm.connect(&serial, None, far_future, &blacklist, 900) {
            ReceiverAction::Failed(e) => assert_eq!(e.code, "E004"),
            other => panic!("serial must enforce TTL, got {:?}", other),
        }
    }

    #[test]
    fn cancel_resets_state() {
        let p = payload();
        let mut fsm = ReceiverFsm::new();
        let blacklist = HashSet::new();
        let _ = fsm.connect(&codec::encode_serial(&p, None), None, p.ts as u64 + 10, &blacklist, 900);
        fsm.cancel();
        assert_eq!(fsm.state, ReceiverState::SIdle);
    }
}
