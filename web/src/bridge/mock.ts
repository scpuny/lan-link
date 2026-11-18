// =====================================================================
// LanLink 桥接层 · 浏览器态空壳实现
// ---------------------------------------------------------------------
// 真机走 Tauri invoke（见 index.ts），本文件只保证浏览器里能打开布局。
// 所有函数保留签名，但不再返回任何原型假数据：列表为空、会话为空、
// 统计为零；需要真实数据的动作明确报错，避免在预览态误判为真实结果。
// =====================================================================
import type {
  DecodedInvite,
  GenerateResult,
  LanCodeResult,
  NearbyDevice,
  CandidateAddr,
  SessionInfo,
  SelfInfo,
  SendFile,
} from '../types/invite'

export const mockSelfInfo: SelfInfo = {
  deviceName: '—',
  osUser: '—',
  lan4: '—',
  pub6: '—',
  stun: '—',
  netCapabilities: { mdns: false, stun: false, ipv6Global: false, ipv4Direct: false },
}

const PREVIEW_ONLY = '预览态不支持此操作，请在应用中运行'

export async function mockGenerateSerial(args: {
  candidates: CandidateAddr[]
  password?: string
  ttlSecs?: number
}): Promise<GenerateResult> {
  return {
    serial: '',
    session: '',
    candidates: args.candidates || [],
    payloadBytes: 0,
    base32Chars: 0,
    createdAt: 0,
    ttl: args.ttlSecs === 300 ? 300 : 900,
    encrypted: !!args.password,
  }
}

export async function mockDecodeSerial(_args: {
  serial: string
  password?: string
}): Promise<DecodedInvite> {
  throw { code: 'E001', message: PREVIEW_ONLY }
}

export async function mockConnect(_args: {
  serial: string
  password?: string
}): Promise<SessionInfo> {
  throw new Error(PREVIEW_ONLY)
}

/** 预览态没有网络，常驻短码始终为空 */
export async function mockGetLanCode(): Promise<LanCodeResult | null> {
  return null
}

export async function mockGenerateLanCode(): Promise<LanCodeResult> {
  return { code: '', serial: '', candidates: [], createdAt: 0, deviceName: '—' }
}

export async function mockConnectLanCode(_args: { code: string }): Promise<SessionInfo> {
  throw new Error(PREVIEW_ONLY)
}

export async function mockScanLan(): Promise<NearbyDevice[]> {
  return []
}

export async function mockGetSelf(): Promise<SelfInfo> {
  return mockSelfInfo
}

/** 可发现开关：保留内存态，供预览交互（非假数据） */
let mockDiscoverable = true
export async function mockSetDiscoverable(on: boolean): Promise<boolean> {
  mockDiscoverable = on
  return mockDiscoverable
}
export async function mockGetDiscoverable(): Promise<boolean> {
  return mockDiscoverable
}

/** 会话黑名单：预览态恒为 0 */
let mockBlacklist = 0
export async function mockGetBlacklistCount(): Promise<number> {
  return mockBlacklist
}
export async function mockClearBlacklist(): Promise<number> {
  mockBlacklist = 0
  return 0
}

export async function mockCancelHandshake(): Promise<boolean> {
  return true
}

export async function mockProbeSerial(): Promise<{ encrypted: boolean; compressed: boolean; flags: number }> {
  return { encrypted: false, compressed: false, flags: 0 }
}

export async function mockTransferCtl(): Promise<boolean> {
  return true
}

export async function mockOpenSaveDir(): Promise<{ path: string }> {
  throw new Error(PREVIEW_ONLY)
}

export async function mockPickFiles(): Promise<SendFile[]> {
  return []
}

export async function mockPickFolder(): Promise<SendFile[]> {
  return []
}

export async function mockReadClipboard(): Promise<string> {
  return ''
}

export async function mockGetSaveDir(): Promise<{ path: string }> {
  return { path: '' }
}

export async function mockChooseSaveDir(): Promise<{ path: string }> {
  return { path: '' }
}
