// LanLink 前端共享类型定义

export type AddrKind = 'Pub6' | 'Pub4' | 'Lan4' | 'ULA'

export interface CandidateAddr {
  kind: AddrKind
  value: string
  ready: boolean
}

/** D4：局域网短码生成结果 */
export interface LanCodeResult {
  code: string
  serial: string
  candidates: CandidateAddr[]
  createdAt: number
  deviceName: string
}

export interface DecodedInvite {
  flags: number
  session: string
  ttl: number
  ts: number
  candidates: CandidateAddr[]
  encrypted: boolean
  compressed: boolean
  payloadBytes: number
  base32Chars: number
}

export interface GenerateResult {
  serial: string
  session: string
  candidates: CandidateAddr[]
  payloadBytes: number
  base32Chars: number
  createdAt: number
  ttl: number
  encrypted: boolean
}

export interface NearbyDevice {
  id: string
  name: string
  platform: string
  addr: string
  latency: number
}

export type VerifyStepKey = 'decode' | 'integrity' | 'parse' | 'ttl' | 'session' | 'challenge'

export interface VerifyStep {
  key: VerifyStepKey
  label: string
  state: 'idle' | 'active' | 'done' | 'error'
}

export type ErrorCode =
  | 'E001' | 'E002' | 'E003' | 'E004' | 'E005'
  | 'E006' | 'E007' | 'E008' | 'E009'

export interface ProtocolError {
  code: ErrorCode
  title: string
  reason: string
  suggestion: string
  /** 原型错误卡第三行「说明」 */
  note: string
  /** 错误卡内第二条同色横幅 */
  banner: string
  bannerTag: string
  /** 第二条横幅右侧补充（如 E006 剩余有效期） */
  bannerNote?: string
  /** 页面最底部灰色提示 */
  footerHint: string
  /** 主按钮文案 */
  primaryLabel: string
  /** E006/E008/E009 顶部绿色「凭证校验全部通过」横幅 */
  topBanner?: string
  topBannerTag?: string
  severity: 'danger' | 'warning'
}

/** 错误态中间的「诊断分区」 */
export interface DiagRow {
  k: string
  v: string
  tone?: 'danger' | 'warning' | 'success' | 'muted'
  right?: string
}
export interface Diagnosis {
  title: string
  right?: string
  rows: DiagRow[]
}

export interface SessionInfo {
  peerName: string
  addr: string
  rtt: number
  encrypted: boolean
  protocol: string
  flags: string
  tookMs: number
}

export interface SelfInfo {
  deviceName: string
  osUser: string
  lan4: string
  pub6: string
  stun: string
  netCapabilities: {
    mdns: boolean
    stun: boolean
    ipv6Global: boolean
    ipv4Direct: boolean
  }
}

/** F22：传输进度事件（后端 ll-transfer-progress → 前端任务卡 / 速率曲线） */
export interface TransferProgress {
  taskId: string
  name: string
  size: number
  transferred: number
  progress: number
  speed: number
  seq: number
  status: 'transferring' | 'done' | 'cancelled' | 'error'
  error?: string
}

/** 待发送文件条目（F1/F2/F3），由原生对话框或剪贴板采集 */
export interface SendFile {
  /** 绝对路径；剪贴板纯文本项为 '' */
  path: string
  name: string
  /** 字节 */
  size: number
  /** Unix 毫秒 */
  modified: number
  /** 大写扩展名，如 PDF / MP4 */
  ext: string
}

export interface TransferTask {
  id: string
  name: string
  size: number
  progress: number
  speed: number // bytes/s
  status: 'transferring' | 'queued' | 'done'
  /** 原型任务卡第二行：862 MB · 分片 4 MB · 发送中 */
  detail?: string
  /** 原型任务卡第四行：534.4 MB / 862 MB · 剩余约 9.6s */
  transferred?: string
  /** 接收端已确认 seq */
  seq?: number
  /** 直接展示的速率文案（34.2 MB/s） */
  speedText?: string
}
