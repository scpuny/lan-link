// =====================================================================
// LanLink 桥接层 · 浏览器态 Mock 实现
// ---------------------------------------------------------------------
// 真实环境（Tauri）走 @tauri-apps/api 的 invoke（见 index.ts），无需 HTTP。
// 这里在浏览器态提供内存实现，便于「先看前端效果」。
// 文案与 calicat 原型保持一致，便于逐像素比对。
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
import { RECEIVE_CHANNEL } from '../data/views'

const ALPHABET = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789'

function delay(ms: number) {
  return new Promise<void>((r) => setTimeout(r, ms))
}

function randGroups(groups: number, perGroup = 6): string {
  let out = ''
  for (let g = 0; g < groups; g++) {
    for (let i = 0; i < perGroup; i++) {
      out += ALPHABET[Math.floor(Math.random() * ALPHABET.length)]
    }
    if (g < groups - 1) out += '-'
  }
  return out
}

function randHex(n: number): string {
  let s = ''
  for (let i = 0; i < n; i++) s += Math.floor(Math.random() * 16).toString(16)
  return s
}

/** 原型 main-ready 左轨本机信息卡 */
export const mockSelfInfo: SelfInfo = {
  deviceName: 'DESKTOP-LL-01',
  osUser: 'Windows 11 · 当前用户 chen',
  lan4: '192.168.1.24',
  pub6: '2408:8207:3d4f:2a1b:9c1d',
  stun: '121.36.80.24:51820',
  netCapabilities: {
    mdns: true,
    stun: true,
    ipv6Global: true,
    ipv4Direct: false,
  },
}

/** 定版 main-ready-locked.png 附近设备表（选择目标设备卡） */
const mockNearby: NearbyDevice[] = [
  { id: 'd1', name: '李工的 MacBook Pro', platform: 'macOS · 可接收', addr: '192.168.1.36', latency: 2 },
  { id: 'd2', name: 'OFFICE-PC-07', platform: 'Windows 11 · 可接收', addr: '192.168.1.58', latency: 11 },
  { id: 'd3', name: 'chen 的 Android', platform: 'Android · 需对方确认', addr: '192.168.1.72', latency: 24 },
]

export async function mockGenerateSerial(args: {
  candidates: CandidateAddr[]
  password?: string
  ttlSecs?: number
}): Promise<GenerateResult> {
  await delay(420)
  const encrypted = !!args.password
  const session = randHex(16)
  return {
    serial: randGroups(14),
    session,
    candidates: args.candidates,
    payloadBytes: 52,
    base32Chars: 84,
    createdAt: Date.now(),
    // 与真机一致：接收设置页传入的有效期（900s / 300s）
    ttl: args.ttlSecs === 300 ? 300 : 900,
    encrypted,
  }
}

export async function mockDecodeSerial(args: {
  serial: string
  password?: string
}): Promise<DecodedInvite> {
  await delay(360)
  const s = (args.serial || '').replace(/[^A-Za-z0-9]/g, '')
  if (s.length < 40) {
    throw { code: 'E001', title: '序列号损坏 / 复制不全' } as any
  }
  const encrypted = !!args.password
  return {
    flags: encrypted ? 0x01 : 0x00,
    session: randHex(16),
    ttl: 900,
    ts: Date.now(),
    candidates: [
      { kind: 'Pub6', value: '2408:8207:3d4f:2a1b:9c1d', ready: true },
      { kind: 'Pub4', value: '121.36.80.24:51820', ready: true },
    ],
    encrypted,
    compressed: false,
    payloadBytes: 52,
    base32Chars: 84,
  }
}

export async function mockConnect(args: {
  serial: string
  password?: string
}): Promise<SessionInfo> {
  await delay(900)
  return {
    peerName: '李工的 MacBook Pro',
    addr: '2408:8207:3d4f:2a1b:9c1d',
    rtt: 38,
    encrypted: !!args.password,
    protocol: 'v1',
    flags: !!args.password ? '0x01' : '0x00',
    tookMs: 1200,
  }
}

/** D4：预览态生成 6 位短码（本地随机，走同一字符集） */
export async function mockGenerateLanCode(): Promise<LanCodeResult> {
  await delay(500)
  let code = ''
  for (let i = 0; i < 6; i++) code += ALPHABET[Math.floor(Math.random() * ALPHABET.length)]
  return {
    code,
    serial: randGroups(3),
    candidates: [{ kind: 'Lan4', value: '192.168.1.24', ready: true }],
    createdAt: Math.floor(Date.now() / 1000),
    deviceName: mockSelfInfo.deviceName,
  }
}

/** D4：预览态连接短码（模拟广播查询命中） */
export async function mockConnectLanCode(args: { code: string }): Promise<SessionInfo> {
  await delay(1100)
  return {
    peerName: '局域网设备',
    addr: '192.168.1.36',
    rtt: 6,
    encrypted: false,
    protocol: 'v1',
    flags: '0x00',
    tookMs: 1150,
  }
}

export async function mockScanLan(): Promise<NearbyDevice[]> {
  await delay(500)
  return mockNearby
}

export async function mockGetSelf(): Promise<SelfInfo> {
  await delay(80)
  return mockSelfInfo
}

/** F10：预览态可发现开关（内存态） */
let mockDiscoverable = true
export async function mockSetDiscoverable(on: boolean): Promise<boolean> {
  mockDiscoverable = on
  return mockDiscoverable
}
export async function mockGetDiscoverable(): Promise<boolean> {
  return mockDiscoverable
}

/** F17：预览态黑名单（内存态） */
let mockBlacklist = 0
export async function mockGetBlacklistCount(): Promise<number> {
  return mockBlacklist
}
export async function mockClearBlacklist(): Promise<number> {
  mockBlacklist = 0
  return 0
}
/** F16：预览态取消握手（空操作） */
export async function mockCancelHandshake(): Promise<boolean> {
  return true
}
/** F15：预览态 flags 探测（示例序列号按未加密处理） */
export async function mockProbeSerial(): Promise<{ encrypted: boolean; compressed: boolean; flags: number }> {
  await delay(60)
  return { encrypted: false, compressed: false, flags: 0 }
}
/** F21：预览态传输控制（空操作，返回成功） */
export async function mockTransferCtl(): Promise<boolean> { return true }

/** 打开接收文件保存目录：预览态不操作文件系统，只返回原型中的目录用于反馈 */
export async function mockOpenSaveDir(): Promise<{ path: string }> {
  await delay(120)
  return { path: RECEIVE_CHANNEL.saveDir }
}

/** F1：预览态模拟选择文件（返回原型待发表对应的条目） */
export async function mockPickFiles(): Promise<SendFile[]> {
  await delay(200)
  return [
    { path: '/Users/chen/Downloads/项目资料/需求规格-v2.pdf', name: '需求规格-v2.pdf', size: 4_404_019, modified: Date.now(), ext: 'PDF' },
    { path: '/Users/chen/Downloads/素材包.zip', name: '素材包.zip', size: 134_217_728, modified: Date.now(), ext: 'ZIP' },
  ]
}

/** F1：预览态模拟选择文件夹 */
export async function mockPickFolder(): Promise<SendFile[]> {
  await delay(200)
  return [
    { path: '/Users/chen/Downloads/LanLink文档/设计说明.pdf', name: '设计说明.pdf', size: 2_097_152, modified: Date.now(), ext: 'PDF' },
    { path: '/Users/chen/Downloads/LanLink文档/演示.mp4', name: '演示.mp4', size: 903_874_560, modified: Date.now(), ext: 'MP4' },
    { path: '/Users/chen/Downloads/LanLink文档/说明.txt', name: '说明.txt', size: 2048, modified: Date.now(), ext: 'TXT' },
  ]
}

/** F2：预览态剪贴板文本 */
export async function mockReadClipboard(): Promise<string> {
  await delay(80)
  return '会议纪要_1116.txt'
}

/** F4：预览态保存目录读写（内存态） */
let mockSaveDir = RECEIVE_CHANNEL.saveDir
export async function mockGetSaveDir(): Promise<{ path: string }> {
  await delay(80)
  return { path: mockSaveDir }
}
export async function mockChooseSaveDir(): Promise<{ path: string }> {
  await delay(160)
  mockSaveDir = '/Users/chen/LanLink/接收文件'
  return { path: mockSaveDir }
}
