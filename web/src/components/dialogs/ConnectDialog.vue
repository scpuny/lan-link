<script setup lang="ts">
import { ref, reactive, computed, watch, onBeforeUnmount } from 'vue'
import Modal from '../common/Modal.vue'
import Icon from '../common/Icon.vue'
import SerialBox from '../common/SerialBox.vue'
import VerifyCapsule from '../common/VerifyCapsule.vue'
import ErrorCard from '../common/ErrorCard.vue'
import { useUi } from '../../composables/useUi'
import { invoke } from '../../bridge'
import { ERRORS, DIAGNOSIS } from '../../data/errors'
import type { VerifyStep, VerifyStepKey, ErrorCode, ProtocolError, SessionInfo } from '../../types/invite'

const { state, openDialog, beginTransferSession } = useUi()
const emit = defineEmits<{ (e: 'close'): void }>()

const tab = ref<'serial' | 'lan'>('serial')
// 序列号从空开始，只由用户粘贴/输入填入
const serial = ref('')
const usePassword = ref(false)
const password = ref('')
const showPwd = ref(false)
const lanCode = ref('')
/** D4：输入即转大写并剔除非法字符（A-Z / 2-9），用户无需手动切换大小写 */
function onLanCodeInput(e: Event) {
  const el = e.target as HTMLInputElement
  const cleaned = el.value.toUpperCase().replace(/[^A-Z2-9]/g, '').slice(0, 6)
  lanCode.value = cleaned
  if (el.value !== cleaned) el.value = cleaned
}
/** D4：设备表选中的目标地址（仅作优先拨号目标，不替代短码），点「选择」后置入 */
const selectedLanAddr = ref('')
/** D4：重新搜索中的瞬时反馈 */
const scanning = ref(false)

type Phase = 'idle' | 'validating' | 'handshake' | 'connected' | 'error'
const phase = ref<Phase>('idle')
const error = ref<ProtocolError | null>(null)

const STEP_DEFS: { key: VerifyStepKey; label: string }[] = [
  { key: 'decode', label: '解码' },
  { key: 'integrity', label: '完整性' },
  { key: 'parse', label: '解析' },
  { key: 'ttl', label: '时效' },
  { key: 'session', label: '会话' },
  { key: 'challenge', label: '质询' },
]
const steps = reactive<VerifyStep[]>(STEP_DEFS.map((s) => ({ ...s, state: 'idle' })))

const countdown = ref(10)
const challenge = ref(randHex(16))
const sessionShort = ref(randHex(8))
const tookMs = ref(1200)
let timer: number | null = null

const isMock = computed(() => state.bridgeMode === 'mock')

const badge = computed(() => {
  switch (phase.value) {
    case 'validating': return { text: '进行中', v: 'info' as const }
    case 'handshake': return { text: '质询中', v: 'brand' as const }
    case 'connected': return { text: '✓ 已建立', v: 'success' as const }
    case 'error': return { text: '× 已终止', v: 'danger' as const }
    default: return null
  }
})

const STEP_OF: Record<ErrorCode, VerifyStepKey> = {
  E001: 'decode', E002: 'integrity', E003: 'integrity', E004: 'ttl',
  E005: 'session', E006: 'challenge', E007: 'parse', E008: 'challenge', E009: 'challenge',
}
function randHex(n: number): string {
  let s = ''
  for (let i = 0; i < n; i++) s += Math.floor(Math.random() * 16).toString(16)
  return s
}
function sleep(ms: number) { return new Promise<void>((r) => setTimeout(r, ms)) }
function resetSteps() { steps.forEach((s) => (s.state = 'idle')) }

async function step(key: VerifyStepKey, ms = 320) {
  const s = steps.find((x) => x.key === key)!
  s.state = 'active'
  await sleep(ms)
  s.state = 'done'
}

function applyDemoError(code: ErrorCode) {
  const order: VerifyStepKey[] = ['decode', 'integrity', 'parse', 'ttl', 'session', 'challenge']
  const idx = order.indexOf(STEP_OF[code])
  steps.forEach((s, i) => (s.state = i < idx ? 'done' : i === idx ? 'error' : 'idle'))
  error.value = ERRORS[code]
  phase.value = 'error'
}

function startCountdown() {
  countdown.value = 10
  timer = window.setInterval(() => {
    countdown.value -= 1
    if (countdown.value <= 0) { stopCountdown(); applyDemoError('E006') }
  }, 1000)
}
function stopCountdown() { if (timer) { clearInterval(timer); timer = null } }
onBeforeUnmount(stopCountdown)

async function runConnect() {
  if (phase.value === 'validating' || phase.value === 'handshake') return
  resetSteps()
  error.value = null

  const inject = state.demoError as ErrorCode | null
  if (inject) { applyDemoError(inject); state.demoError = null; return }

  phase.value = 'validating'
  await step('decode')
  try {
    await invoke('decode_serial', {
      serial: serial.value,
      password: usePassword.value ? password.value : undefined,
    })
  } catch (e: any) {
    applyDemoError((e?.code as ErrorCode) || 'E001')
    return
  }
  await step('integrity')
  await step('parse')
  await step('ttl')
  await step('session')

  phase.value = 'handshake'
  sessionShort.value = randHex(8)
  challenge.value = randHex(16)
  startCountdown()
  await step('challenge', 1500)
  stopCountdown()

  try {
    // F18：把当前待发送文件传给传输引擎，握手成功后后台线程真实发送
    const files = state.sendFiles
      .filter((f) => f.path)
      .map((f) => ({ path: f.path, name: f.name, size: f.size }))
    const sess = await invoke<SessionInfo>('connect', {
      serial: serial.value,
      password: usePassword.value ? password.value : undefined,
      files,
    })
    tookMs.value = sess.tookMs
    sessionShort.value = (sess as any).sessionShort || sessionShort.value
    state.session = sess
    beginTransferSession()
    phase.value = 'connected'
  } catch (e: any) {
    // 真机握手失败：从后端错误串提取 E00x，映射到现有错误卡（不新增硬编码判断）
    stopCountdown()
    const m = String(e?.message || e || '').match(/E00[1-9]/)
    applyDemoError((m?.[0] as ErrorCode) || 'E009')
  }
}

/** D4：重新扫描局域网附近设备 */
async function rescanLan() {
  if (scanning.value) return
  scanning.value = true
  try {
    state.nearby = await invoke('scan_lan')
  } catch { /* 预览态保留 mock 设备表 */ }
  scanning.value = false
}

/** D4：用 6 位短码连接局域网持有方；设备表选中项作为优先拨号地址 */
async function runLanConnect() {
  if (phase.value === 'validating' || phase.value === 'handshake') return
  const code = lanCode.value.trim().toUpperCase()
  if (!/^[A-Z2-9]{6}$/.test(code)) {
    applyLanError('短码须为 6 位字符（A-Z / 2-9）')
    return
  }
  resetSteps()
  error.value = null
  phase.value = 'validating'
  await step('decode', 200)
  await step('integrity', 200)
  await step('parse', 150)
  await step('ttl', 150)
  await step('session', 150)
  phase.value = 'handshake'
  const files = state.sendFiles
    .filter((f) => f.path)
    .map((f) => ({ path: f.path, name: f.name, size: f.size }))
  try {
    const sess = await invoke<SessionInfo>('connect_lan_code', {
      code,
      targetAddr: selectedLanAddr.value || undefined,
      files,
    })
    tookMs.value = sess.tookMs
    state.session = sess
    beginTransferSession()
    phase.value = 'connected'
  } catch (e: any) {
    const raw = String(e?.message || e || '')
    applyLanError(raw)
  }
}

/** D4：短码错误复用 E009 错误卡骨架（橙色 / 网络问题），只替换原因与建议文案。
 *  后端返回的 E010 属于局域网扩展码，不进主协议 E001–E009 错误表。 */
function applyLanError(detail: string) {
  stopCountdown()
  const isE010 = /E010/.test(detail) || /短码须为/.test(detail)
  error.value = {
    ...ERRORS.E009,
    reason: isE010 ? (detail.replace(/^E010:\s*/, '') || ERRORS.E009.reason) : detail,
    suggestion: isE010
      ? '确认双方在同一局域网、对方程序正在运行且短码未失效；必要时改用外网序列号连接'
      : ERRORS.E009.suggestion,
    primaryLabel: '重新搜索',
    severity: 'warning',
  }
  phase.value = 'error'
}

/** 状态行文案（原型各态） */
const statusText = computed(() => {
  if (phase.value === 'connected') return ''
  if (phase.value === 'validating') return '正在校验序列号…（CRC32 校验中）'
  if (phase.value === 'handshake') return '正在握手质询… 已下发 challenge，等待 Blake3 摘要应答'
  if (phase.value === 'error') return `${error.value?.code} ${error.value?.severity === 'warning' ? '网络不可达' : '校验终止'} · 未建立通道`
  return '就绪，粘贴序列号后点击连接'
})
const statusTone = computed(() => {
  if (phase.value === 'error') return 'error' as const
  if (phase.value === 'validating' || phase.value === 'handshake') return 'active' as const
  return 'idle' as const
})
const timeoutText = computed(() =>
  phase.value === 'handshake' ? `剩余 ${countdown}s` : '质询超时 10s'
)

/** 错误态诊断分区（E001 无独立诊断卡，输入框本身即诊断） */
const diagnosis = computed(() =>
  phase.value === 'error' && error.value ? DIAGNOSIS[error.value.code] : undefined
)
/** E009 原型不显示六态胶囊，仅显示橙色诊断面板 */
const noCapsule = computed(() => phase.value === 'error' && error.value?.code === 'E009')

const footerHint = computed(() => {
  if (phase.value === 'connected') return '序列号与临时缓冲已释放，仅保留必要会话标识'
  if (phase.value === 'error') return error.value?.footerHint || ''
  return '序列号仅在内存中停留，关闭弹窗即销毁'
})

function openTransferPanel() {
  state.centerState = 'transferring'
  emit('close')
}
function onPrimary() {
  if (phase.value === 'connected') { openTransferPanel(); return }
  if (phase.value === 'error') {
    // E005 / E008 / E004 需重新生成；其余回到输入态
    if (['E004', 'E005', 'E008'].includes(error.value?.code || '')) { openDialog('generate'); return }
    error.value = null
    phase.value = 'idle'
    resetSteps()
    return
  }
  runConnect()
}
// BR-06 / C16：切换 Tab 不保留另一页的校验状态与错误卡（输入缓冲各自独立）
watch(tab, () => {
  stopCountdown()
  phase.value = 'idle'
  error.value = null
  resetSteps()
})

// F15：序列号变化时探测 flags；加密块自动勾出口令输入（FR-06）。
let probeToken = 0
watch(serial, async (v) => {
  const token = ++probeToken
  const cleaned = String(v || '').replace(/[^A-Za-z0-9]/g, '')
  if (cleaned.length < 40) return
  try {
    const r = await invoke<{ encrypted: boolean }>('probe_serial', { serial: v })
    if (token === probeToken && r.encrypted) usePassword.value = true
  } catch {
    /* 解析失败交给正式连接流程报错 */
  }
})

function close() {
  stopCountdown()
  // F16：关闭弹窗即取消握手、释放 challenge；口令缓冲清零（BR-01/03）
  invoke('cancel_handshake').catch(() => {})
  password.value = ''
  serial.value = ''
  emit('close')
}
</script>

<template>
  <Modal
    title="连接到远程设备"
    icon="link"
    :badge="badge?.text"
    :badge-variant="badge?.v"
    :width="520"
    @close="close"
  >
    <!-- Tab 隔离 + 协议版本 -->
    <div class="cd-tabs">
      <div class="cd-tabbar">
        <button :data-active="tab === 'serial'" @click="tab = 'serial'">
          <span class="tab-dot" :class="{ on: tab === 'serial' }" /> 外网序列号连接
        </button>
        <button :data-active="tab === 'lan'" @click="tab = 'lan'">
          <span class="tab-dot" :class="{ on: tab === 'lan' }" /> 局域网短码连接
        </button>
      </div>
      <span class="cd-proto ll-text-tertiary">协议 v1</span>
    </div>

    <!-- ===== Tab1：外网序列号 ===== -->
    <template v-if="tab === 'serial'">
      <!-- 成功态 -->
      <template v-if="phase === 'connected'">
        <div class="ll-banner ll-banner--success cd-success">
          <span class="cs-check"><Icon name="check" :size="15" :width="2.4" /></span>
          <div class="ll-col">
            <b>握手成功，通道已建立</b>
            <span class="ll-text-tertiary">Blake3 摘要校验匹配 · 本次握手耗时 {{ (tookMs / 1000).toFixed(1) }}s</span>
          </div>
        </div>

        <VerifyCapsule :steps="steps" status-text="" class="cd-verify" />

        <div class="ll-card cd-channel">
          <div class="cd-channel-head">
            <span class="ll-section-title"><Icon name="link" :size="13" class="ci-ico" /> 通道信息</span>
            <span class="ll-text-tertiary ll-mono">协议 v1 · flags={{ state.session?.flags || '0x00' }}</span>
          </div>
          <div class="ll-kv"><span class="ll-kv__k">对端设备</span><span class="ll-kv__v">{{ state.session?.peerName || '—' }}</span></div>
          <div class="ll-kv"><span class="ll-kv__k">连接地址</span><span class="ll-kv__v ll-mono">{{ state.session?.addr || '—' }} <span class="ll-tag ll-tag--brand">{{ (state.session?.addr || '').includes(':') ? 'IPv6 直连' : '局域网直连' }}</span></span></div>
          <div class="ll-kv"><span class="ll-kv__k">往返延迟</span><span class="ll-kv__v">{{ state.session?.rtt ?? 0 }} ms</span></div>
          <div class="ll-kv"><span class="ll-kv__k">通道加密</span><span class="ll-kv__v">{{ state.session?.encrypted ? 'AES-GCM-256（口令保护）' : '关闭（明文二进制 + CRC32 完整性）' }}</span></div>
        </div>

        <div class="cd-blacklist">
          <Icon name="alertCircle" :size="12" />
          <span>该序列号已写入会话黑名单，即刻作废，不可再次使用</span>
          <span class="ll-tag ll-tag--plain">黑名单 1 条</span>
        </div>

        <div class="cd-actions cd-actions--two">
          <button class="ll-btn ll-btn--primary" @click="openTransferPanel"><Icon name="external" :size="13" /> 打开传输面板</button>
          <button class="ll-btn ll-btn--default" @click="close">关闭</button>
        </div>

        <div class="cd-success-bar">
          <span class="ll-dot ll-dot--success" />
          <div class="ll-col">
            <b>握手成功，通道已建立</b>
            <span class="ll-text-tertiary">可开始传输，分片报文按 seq 单调递增校验</span>
          </div>
        </div>
      </template>

      <!-- 输入 / 校验 / 握手 / 错误态 -->
      <template v-else>
        <!-- 顶部蓝色说明横幅（仅初始态） -->
        <div v-if="phase === 'idle'" class="ll-banner cd-intro">
          <Icon name="info" :size="14" class="ll-banner__icon" />
          <span>外网序列号自包含连接凭证，有效期 15 分钟，一个序列号仅可完成一次握手。<br>优先尝试公网 IPv6，不通则回落公网 IPv4。</span>
        </div>

        <!-- E006 / E008 / E009：凭证校验全部通过 -->
        <div v-if="phase === 'error' && error?.topBanner" class="cd-top-ok">
          <span class="cd-top-check"><Icon name="check" :size="13" :width="2.4" /></span>
          <div class="ll-col">
            <b>{{ error.topBanner }}</b>
            <span class="ll-text-tertiary">序列号合法、未过期、未使用，可继续握手</span>
          </div>
          <span class="ll-tag ll-tag--success">{{ error.topBannerTag }}</span>
        </div>

        <!-- 校验中 / 握手质询中：蓝色进度横幅 -->
        <div v-if="phase === 'validating' || phase === 'handshake'" class="ll-banner cd-progress">
          <span class="ll-spinner" />
          <div class="ll-col">
            <b>{{ phase === 'validating' ? '正在校验序列号…' : '正在握手质询…' }}</b>
            <span class="ll-text-tertiary">
              {{ phase === 'validating' ? '正在校验格式，解码 Base32 与 CRC32 完整性' : '已向对端发起连接并下发 challenge，等待 Blake3 摘要应答' }}
            </span>
          </div>
          <span class="cd-progress-tag ll-mono">{{ phase === 'validating' ? '0.1s' : '超时 10s' }}</span>
        </div>

        <!-- 动态质询卡片（握手态） -->
        <div v-if="phase === 'handshake'" class="ll-card cd-challenge">
          <div class="cd-challenge-head">
            <span class="ll-tag ll-tag--brand">Blake3 Challenge-Response</span>
            <span class="cd-countdown"><Icon name="clock" :size="12" /> 剩余 {{ countdown }}s</span>
          </div>
          <div class="ll-kv"><span class="ll-kv__k">session 前 8 位</span><span class="ll-kv__v ll-mono">{{ sessionShort.slice(0, 8) }}</span></div>
          <div class="ll-kv"><span class="ll-kv__k">16 字节 challenge</span><span class="ll-kv__v ll-mono">{{ challenge.slice(0, 16) }}</span></div>
          <div class="ll-kv"><span class="ll-kv__k">目标地址</span><span class="ll-kv__v ll-mono">{{ state.session?.addr || '等待对端应答' }}</span></div>
          <p class="ll-text-tertiary cd-challenge-note">已向对端发起连接并下发 challenge，等待 Blake3 摘要应答；成功后写入黑名单。</p>
        </div>

        <!-- 校验中解析结果 -->
        <div v-if="phase === 'validating'" class="ll-card cd-parsed">
          <div class="ll-row ll-between"><span class="ll-section-title">解析序列号</span><span class="ll-text-tertiary ll-mono">正在解码 Base32 · CRC32 校验中</span></div>
          <div class="ll-kv"><span class="ll-kv__k">输入字符</span><span class="ll-kv__v ll-mono">{{ serial.replace(/[^A-Za-z0-9]/g, '').length }} 位</span></div>
          <div class="ll-kv"><span class="ll-kv__k">候选地址</span><span class="ll-kv__v ll-mono">解析完成后显示</span></div>
        </div>

        <div class="cd-field-head">
          <span>邀请序列号</span>
          <span :class="phase === 'error' ? 'cd-head-error' : 'll-text-tertiary'">
            {{ phase === 'error' ? '解码失败 · 未发起任何网络请求' : '粘贴后自动清洗大小写 / 空格 / 换行' }}
          </span>
        </div>

        <SerialBox
          v-model="serial"
          editable
          :error="phase === 'error'"
          :corner-tag="phase === 'error' && error?.code === 'E001' ? '最后一组 5 字符' : undefined"
          :error-text="phase === 'error' && error?.code === 'E001' ? '共粘贴 34 字符 · 5 组 + 1 组残缺，Base32 解码需要完整 6 字符分组' : undefined"
        />

        <!-- 非错误态：IM 粘贴说明 + 口令区（原型错误态不显示这两块） -->
        <template v-if="phase !== 'error'">
          <div class="cd-meta">
            <label class="cd-checkbox"><input type="checkbox" checked disabled /> 可直接从微信等 IM 粘贴整段文本</label>
            <span class="ll-text-tertiary">格式：Base32 · 每 6 字符一组</span>
          </div>

          <div class="cd-pwd">
            <div class="cd-pwd-head">
              <label class="cd-checkbox"><input v-model="usePassword" type="checkbox" /> 口令保护（该序列号已加密）</label>
              <span class="ll-tag ll-tag--plain">AES-GCM-256</span>
            </div>
            <p class="ll-text-tertiary">勾选后连接时会要求输入对方单独告知的 4-6 位数字口令</p>
            <div v-if="usePassword" class="ll-input-wrap cd-pwd-input">
              <input
                v-model="password"
                class="ll-input ll-input--mono"
                :type="showPwd ? 'text' : 'password'"
                maxlength="6"
                placeholder="对方单独告知的数字口令"
              />
              <span class="ll-input-affix ll-click" @click="showPwd = !showPwd">
                <Icon :name="showPwd ? 'eyeOff' : 'eye'" :size="14" />
              </span>
            </div>
          </div>
        </template>

        <!-- 错误态：六态胶囊在错误卡之前（原型 err-E001 / E002 / E004…顺序） -->
        <VerifyCapsule
          v-if="phase === 'error' && !noCapsule"
          class="cd-verify"
          :steps="steps"
          status-text=""
          timeout-text=""
        />

        <!-- 诊断分区 -->
        <div v-if="diagnosis" class="ll-card cd-diag">
          <div class="cd-diag-head">
            <span class="ll-section-title">{{ diagnosis.title }}</span>
            <span v-if="diagnosis.right" class="ll-text-tertiary">{{ diagnosis.right }}</span>
          </div>
          <div v-for="row in diagnosis.rows" :key="row.k" class="cd-diag-row" :class="`tone-${row.tone || 'normal'}`">
            <span class="cd-diag-k">{{ row.k }}</span>
            <span class="cd-diag-v ll-mono">{{ row.v }}</span>
            <span v-if="row.right" class="cd-diag-right">{{ row.right }}</span>
          </div>
        </div>

        <!-- 错误卡（内嵌） + 错误态按钮 -->
        <ErrorCard v-if="phase === 'error' && error" class="cd-error-card" :error="error" />
        <div v-if="phase === 'error'" class="cd-actions" :class="{ 'cd-actions--two': error?.code !== 'E009' }">
          <button class="ll-btn ll-btn--primary" @click="onPrimary">
            <Icon name="refresh" :size="13" /> {{ error?.primaryLabel || '重新粘贴' }}
          </button>
          <button v-if="error?.code === 'E009'" class="ll-btn ll-btn--default" @click="tab = 'lan'">
            <Icon name="lan" :size="13" /> 改用局域网短码
          </button>
          <button class="ll-btn ll-btn--default" @click="close">关闭</button>
        </div>

        <!-- 非错误态：按钮 → 状态行 + 六态胶囊 -->
        <template v-if="phase !== 'error'">
          <div class="cd-actions">
            <button class="ll-btn ll-btn--default" @click="openDialog('generate')"><Icon name="plus" :size="13" /> 生成序列号</button>
            <button
              class="ll-btn ll-btn--primary"
              :disabled="phase === 'validating' || phase === 'handshake' || !serial"
              @click="runConnect"
            >
              <span v-if="phase === 'validating' || phase === 'handshake'" class="ll-spinner" />
              <Icon v-else name="link" :size="13" />
              {{ phase === 'validating' ? '连接中…' : phase === 'handshake' ? '等待应答…' : '连接' }}
            </button>
            <button class="ll-btn ll-btn--default" @click="close">取消</button>
          </div>

          <VerifyCapsule
            class="cd-verify"
            :steps="steps"
            :status-text="statusText"
            :status-tone="statusTone"
            :timeout-text="timeoutText"
          />
        </template>

        <!-- 校验 / 握手态底部计时提示 -->
        <div v-if="phase === 'validating' || phase === 'handshake'" class="cd-clock-hint">
          <Icon name="clock" :size="12" />
          <span>{{ phase === 'validating' ? '校验超时 5s / 已用 0.1s · CRC32 与 Payload 解析' : '质询超时 10s / 已等待 1.0s · 超时后本轮作废，TTL 内可再握手' }}</span>
        </div>
      </template>

      <!-- 页脚提示 + 预览用错误码注入 -->
      <div class="cd-footer">
        <span><Icon name="lock" :size="11" /> {{ footerHint }}</span>
        <select
          v-if="isMock && phase !== 'connected'"
          class="cd-demo"
          :value="state.demoError || ''"
          @change="state.demoError = ($event.target as HTMLSelectElement).value || null"
        >
          <option value="">错误码 E001–E009</option>
          <option v-for="c in ['E001','E002','E003','E004','E005','E006','E007','E008','E009']" :key="c" :value="c">预览 {{ c }}</option>
        </select>
        <span v-else class="ll-text-tertiary">错误码 E001–E009</span>
      </div>
    </template>

    <!-- ===== Tab2：局域网短码 ===== -->
    <template v-else>
      <div class="cd-field-head">
        <span>邀请短码（6 位）</span>
        <span class="ll-text-tertiary">必填 · 输入自动转大写（A-Z / 2-9）</span>
      </div>
      <input
        :value="lanCode"
        class="ll-input ll-input--mono cd-lan-input"
        maxlength="6"
        autocomplete="off"
        autocapitalize="characters"
        spellcheck="false"
        placeholder="XXXXXX"
        @input="onLanCodeInput"
        @keyup.enter="runLanConnect"
      />
      <div class="cd-meta"><span class="ll-text-tertiary"><span class="ll-dot ll-dot--success" /> 局域网广播中 · 发现 42211 / 传输 42210</span></div>

      <div v-if="phase === 'validating' || phase === 'handshake'" class="ll-banner cd-lan-state cd-lan-state--info">
        <span class="ll-spinner" />
        <span>{{ phase === 'validating' ? '正在广播查询短码持有方…' : '已定位持有方，正在质询握手…' }}</span>
      </div>
      <div v-else-if="phase === 'error' && error" class="ll-banner ll-banner--warning cd-lan-state cd-lan-state--warn">
        <Icon name="alertTriangle" :size="13" />
        <span><b>{{ error.reason }}</b> · {{ error.suggestion }}</span>
      </div>
      <div v-else-if="phase === 'connected' && state.session" class="ll-banner ll-banner--success cd-lan-state">
        <Icon name="check" :size="13" />
        <span>已连接 {{ state.session.peerName }}（{{ state.session.addr }} · {{ state.session.rtt }}ms）</span>
      </div>

      <div class="ll-card cd-lan-card">
        <div class="cd-lan-head"><span class="ll-section-title">附近设备（已发现 {{ state.nearby.length || 3 }} 台）</span><span class="ll-text-tertiary">可选 · 仅指定优先拨号地址</span></div>
        <div
          v-for="d in state.nearby"
          :key="d.id"
          class="cd-lan-row"
          :class="{ 'is-selected': selectedLanAddr === d.addr }"
          @click="selectedLanAddr = d.addr"
        >
          <span class="cd-lan-ico"><Icon :name="d.platform.startsWith('Android') ? 'phone' : d.platform.startsWith('macOS') ? 'laptop' : 'monitor'" :size="14" /></span>
          <div class="ll-col"><span>{{ d.name }}</span><span class="ll-text-tertiary cd-lan-platform">{{ d.platform }}</span></div>
          <span class="ll-mono ll-text-secondary">{{ d.addr }}</span>
          <span class="cd-lan-lat">{{ d.latency }}ms</span>
          <button class="ll-btn ll-btn--text ll-btn--sm" @click.stop="selectedLanAddr = d.addr">
            {{ selectedLanAddr === d.addr ? '已选' : '选择' }}
          </button>
        </div>
      </div>

      <div class="cd-lan-note">
        <p><Icon name="info" :size="12" /> 短码必须输入：它是对方给你的接收授权，设备表不能代替；下方设备只决定优先拨号地址</p>
        <p><Icon name="info" :size="12" /> 6 位短码不携带地址，通过 UDP 42211 广播发现持有方；跨网段 / 跨 VLAN 请改用外网序列号</p>
      </div>

      <div class="cd-actions">
        <button class="ll-btn ll-btn--default" :disabled="scanning" @click="rescanLan"><Icon name="refresh" :size="13" /> {{ scanning ? '搜索中…' : '重新搜索' }}</button>
        <button class="ll-btn ll-btn--primary" :disabled="phase === 'validating' || phase === 'handshake'" @click="phase === 'connected' ? openTransferPanel() : runLanConnect()"><Icon name="link" :size="13" /> {{ phase === 'connected' ? '打开传输面板' : '连接' }}</button>
        <button class="ll-btn ll-btn--default" @click="close">取消</button>
      </div>

      <div class="cd-lan-status">
        <span><span class="ll-dot" /> 就绪，输入对方提供的 6 位短码后连接</span>
        <span class="ll-text-tertiary"><Icon name="clock" :size="11" /> 程序运行期有效</span>
      </div>
    </template>
  </Modal>
</template>

<style scoped>
.cd-tabs { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--ll-space-4); }
.cd-tabbar { flex: 1; display: flex; gap: var(--ll-space-5); }
.cd-tabbar button {
  position: relative; border: none; background: transparent;
  padding: 0 0 var(--ll-space-2);
  display: inline-flex; align-items: center; gap: var(--ll-space-2);
  font-size: var(--ll-fs-md); color: var(--ll-color-text-secondary);
}
.cd-tabbar button[data-active="true"] { color: var(--ll-color-brand); font-weight: var(--ll-fw-semibold); }
.cd-tabbar button[data-active="true"]::after {
  content: ''; position: absolute; left: 0; right: 0; bottom: 0;
  height: 2px; border-radius: 2px; background: var(--ll-color-brand);
}
.cd-proto { font-size: var(--ll-fs-xs); }
.tab-dot { width: 7px; height: 7px; border-radius: 50%; background: var(--ll-color-text-quaternary); }
.tab-dot.on { background: var(--ll-color-brand); }

.cd-intro { margin-bottom: var(--ll-space-4); font-size: var(--ll-fs-xs); }
.cd-field-head { display: flex; align-items: baseline; justify-content: space-between; gap: var(--ll-space-3); margin-bottom: var(--ll-space-2); font-size: var(--ll-fs-sm); }
.cd-head-error { color: var(--ll-color-danger); font-size: var(--ll-fs-xs); text-align: right; }
.cd-meta { display: flex; align-items: center; justify-content: space-between; gap: var(--ll-space-3); margin-top: var(--ll-space-2); font-size: var(--ll-fs-xs); }
.cd-checkbox { display: inline-flex; align-items: center; gap: var(--ll-space-2); font-size: var(--ll-fs-xs); color: var(--ll-color-text-secondary); cursor: pointer; }
.cd-checkbox input { accent-color: var(--ll-color-brand); width: 13px; height: 13px; margin: 0; }

.cd-pwd { margin-top: var(--ll-space-4); padding: var(--ll-space-3); border: 1px solid var(--ll-color-border); border-radius: var(--ll-radius-md); background: var(--ll-color-surface-2); }
.cd-pwd-head { display: flex; align-items: center; justify-content: space-between; gap: var(--ll-space-3); }
.cd-pwd p { font-size: var(--ll-fs-xs); margin-top: var(--ll-space-2); }
.cd-pwd-input { margin-top: var(--ll-space-3); }

.cd-actions { display: flex; gap: var(--ll-space-2); margin-top: var(--ll-space-4); }
.cd-actions > .ll-btn:first-child { flex: 1.1; }
.cd-actions > .ll-btn--primary { flex: 1.6; }
.cd-actions > .ll-btn:last-child { flex: 0.9; }
.cd-actions--two > .ll-btn { flex: 1 !important; }
.cd-verify { margin-top: var(--ll-space-4); }

.cd-footer { display: flex; align-items: center; justify-content: space-between; gap: var(--ll-space-3); margin-top: var(--ll-space-4); font-size: 10px; color: var(--ll-color-text-quaternary); }
.cd-footer > span { display: inline-flex; align-items: center; gap: var(--ll-space-1); }
.cd-demo { border: none; background: transparent; color: var(--ll-color-text-quaternary); font-size: 10px; cursor: pointer; outline: none; }

.cd-success { align-items: center; margin-bottom: var(--ll-space-3); }
.cd-success b { font-size: var(--ll-fs-md); color: var(--ll-color-success); }
.cs-check { width: 24px; height: 24px; border-radius: 50%; background: var(--ll-color-success); color: #fff; display: inline-flex; align-items: center; justify-content: center; flex: none; }
.cd-channel { padding: var(--ll-space-3); margin-top: var(--ll-space-3); }
.cd-channel-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--ll-space-2); font-size: var(--ll-fs-xs); }
.cd-channel-head .ll-section-title { display: inline-flex; align-items: center; gap: var(--ll-space-2); }
.ci-ico { color: var(--ll-color-text-tertiary); }
.cd-channel .ll-kv__k { width: 64px; }
.cd-blacklist {
  display: flex; align-items: center; gap: var(--ll-space-2);
  margin-top: var(--ll-space-3); padding: var(--ll-space-2) var(--ll-space-3);
  background: var(--ll-color-surface-2); border: 1px solid var(--ll-color-border);
  border-radius: var(--ll-radius-sm); font-size: var(--ll-fs-xs); color: var(--ll-color-text-tertiary);
}
.cd-blacklist > span:nth-child(2) { flex: 1; }
.cd-success-bar { display: flex; align-items: flex-start; gap: var(--ll-space-2); margin-top: var(--ll-space-3); padding: var(--ll-space-3); background: var(--ll-color-success-light); border: 1px solid var(--ll-color-success); border-radius: var(--ll-radius-md); }
.cd-success-bar .ll-dot { margin-top: 5px; }
.cd-success-bar b { font-size: var(--ll-fs-sm); color: var(--ll-color-success); }
.cd-success-bar .ll-text-tertiary { font-size: var(--ll-fs-xs); }

.cd-challenge, .cd-parsed { padding: var(--ll-space-3); margin-bottom: var(--ll-space-4); }
.cd-challenge-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--ll-space-2); }
.cd-countdown { display: inline-flex; align-items: center; gap: var(--ll-space-1); color: var(--ll-color-warning); font-size: var(--ll-fs-xs); }
.cd-challenge .ll-kv__k, .cd-parsed .ll-kv__k { width: 110px; }
.cd-challenge-note { font-size: var(--ll-fs-xs); margin-top: var(--ll-space-2); }
.cd-error-card { margin-top: var(--ll-space-4); }

.cd-lan-input { font-size: var(--ll-fs-xl); letter-spacing: 6px; height: 44px; text-align: center; }
.cd-lan-card { margin-top: var(--ll-space-4); padding: var(--ll-space-3); }
.cd-lan-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--ll-space-2); font-size: var(--ll-fs-xs); }
.cd-lan-row { display: flex; align-items: center; gap: var(--ll-space-3); padding: var(--ll-space-2) 0; border-top: 1px solid var(--ll-color-divider); font-size: var(--ll-fs-sm); }
.cd-lan-ico { color: var(--ll-color-brand); display: inline-flex; }
.cd-lan-row .ll-col { flex: 1; min-width: 0; }
.cd-lan-platform { font-size: 10px; }
.cd-lan-lat { color: var(--ll-color-success); font-size: var(--ll-fs-xs); }
.cd-lan-banner { margin-top: var(--ll-space-3); font-size: var(--ll-fs-xs); }

/* 凭证校验通过横幅 */
.cd-top-ok {
  display: flex; align-items: center; gap: var(--ll-space-2);
  margin-bottom: var(--ll-space-4); padding: var(--ll-space-3);
  background: var(--ll-color-success-light); border: 1px solid var(--ll-color-success);
  border-radius: var(--ll-radius-md);
}
.cd-top-ok b { font-size: var(--ll-fs-sm); color: var(--ll-color-success); }
.cd-top-ok .ll-text-tertiary { font-size: var(--ll-fs-xs); }
.cd-top-ok .ll-col { flex: 1; }
.cd-top-check {
  width: 22px; height: 22px; flex: none; border-radius: 50%;
  background: var(--ll-color-success); color: #fff;
  display: inline-flex; align-items: center; justify-content: center;
}

/* 校验 / 握手蓝色进度横幅 */
.cd-progress { align-items: center; margin-bottom: var(--ll-space-4); }
.cd-progress b { font-size: var(--ll-fs-sm); color: var(--ll-color-brand); }
.cd-progress .ll-col { flex: 1; }
.cd-progress .ll-text-tertiary { font-size: var(--ll-fs-xs); }
.cd-progress-tag { font-size: var(--ll-fs-xs); color: var(--ll-color-brand); white-space: nowrap; }

/* 诊断分区 */
.cd-diag { padding: var(--ll-space-3); margin-top: var(--ll-space-4); }
.cd-diag-head { display: flex; align-items: center; justify-content: space-between; gap: var(--ll-space-3); margin-bottom: var(--ll-space-2); font-size: var(--ll-fs-xs); }
.cd-diag-row {
  display: flex; align-items: center; gap: var(--ll-space-3);
  padding: var(--ll-space-2) var(--ll-space-2);
  border-top: 1px solid var(--ll-color-divider);
  font-size: var(--ll-fs-xs);
}
.cd-diag-k { width: 92px; flex: none; color: var(--ll-color-text-tertiary); }
.cd-diag-v { flex: 1; min-width: 0; color: var(--ll-color-text-secondary); font-size: 11px; }
.cd-diag-right { flex: none; font-size: 10px; color: var(--ll-color-text-tertiary); }
.cd-diag-row.tone-danger { background: var(--ll-color-danger-light); }
.cd-diag-row.tone-danger .cd-diag-right, .cd-diag-row.tone-danger .cd-diag-v { color: var(--ll-color-danger); }
.cd-diag-row.tone-warning .cd-diag-right { color: var(--ll-color-warning); }
.cd-diag-row.tone-success .cd-diag-right { color: var(--ll-color-success); }
.cd-diag-row.tone-muted { color: var(--ll-color-text-quaternary); }

.cd-clock-hint { display: flex; align-items: center; gap: var(--ll-space-2); margin-top: var(--ll-space-3); font-size: 10px; color: var(--ll-color-text-tertiary); }

/* Tab2 灰底说明框 + 状态条 */
.cd-lan-note { margin-top: var(--ll-space-3); padding: var(--ll-space-3); background: var(--ll-color-surface-2); border: 1px solid var(--ll-color-border); border-radius: var(--ll-radius-md); }
.cd-lan-note p { display: flex; align-items: flex-start; gap: var(--ll-space-2); font-size: var(--ll-fs-xs); color: var(--ll-color-text-secondary); line-height: var(--ll-lh-relaxed); }
.cd-lan-note p + p { margin-top: var(--ll-space-2); }
.cd-lan-note svg { flex: none; margin-top: 1px; color: var(--ll-color-text-tertiary); }
.cd-lan-status { display: flex; align-items: center; justify-content: space-between; margin-top: var(--ll-space-4); font-size: var(--ll-fs-xs); color: var(--ll-color-text-secondary); }
.cd-lan-status > span { display: inline-flex; align-items: center; gap: var(--ll-space-2); }
/* D4：设备行可选中 + 连接状态横幅 */
.cd-lan-row { cursor: pointer; }
.cd-lan-row.is-selected { background: var(--ll-color-row-hover); }
.cd-lan-row.is-selected .cd-lan-ico { color: var(--ll-color-brand); }
.cd-lan-state { margin: var(--ll-space-3) 0; font-size: var(--ll-fs-xs); line-height: 1.5; }
.cd-lan-state--info { color: var(--ll-color-brand); }
.cd-lan-state--warn { color: var(--ll-color-warning); }
.cd-lan-state b { font-weight: 600; }
</style>
