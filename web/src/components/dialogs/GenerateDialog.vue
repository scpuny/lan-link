<script setup lang="ts">
import { ref, reactive, computed, onBeforeUnmount } from 'vue'
import Modal from '../common/Modal.vue'
import Icon from '../common/Icon.vue'
import { useUi } from '../../composables/useUi'
import { invoke } from '../../bridge'
import type { GenerateResult, CandidateAddr } from '../../types/invite'

const { state, openDialog, showToast, confirm } = useUi()
const emit = defineEmits<{ (e: 'close'): void }>()

type Phase = 'form' | 'generating' | 'done'
const phase = ref<Phase>('form')
const result = ref<GenerateResult | null>(null)

/** 候选地址选项：实际地址由后端 get_self_info / generate_serial 采集，此处只有类型说明 */
const cands = reactive<
  { kind: CandidateAddr['kind']; label: string; desc: string; checked: boolean; optional?: boolean }[]
>([
  { kind: 'Pub6', label: 'Pub6', desc: '全球单播 IPv6 · 跨网首选', checked: true },
  { kind: 'Pub4', label: 'Pub4', desc: 'STUN 观测端点（由本机网络采集）', checked: true },
  { kind: 'Lan4', label: '', desc: '包含内网地址（由网卡采集）', checked: false, optional: true },
])
const selectedCount = computed(() => cands.filter((c) => c.checked).length)

const usePassword = ref(false)
const pwd = ref('')
const pwd2 = ref('')
const showPwd = ref(false)
const pwdValid = computed(() => /^\d{4,6}$/.test(pwd.value) && pwd.value === pwd2.value)
const pwdHint = computed(() => {
  if (!pwd.value) return { text: '', ok: false }
  if (!/^\d{4,6}$/.test(pwd.value)) return { text: '需为 4–6 位数字', ok: false }
  if (pwd.value !== pwd2.value) return { text: '两次输入不一致', ok: false }
  return { text: '两次一致 · 强度可用', ok: true }
})

/** 与 SettingsDialog 共用 localStorage 键，读取「外网序列号有效期」设置（900s / 300s） */
function readSavedTtl(): number {
  try {
    const s = JSON.parse(localStorage.getItem('ll-settings-v1') || '{}')
    return s.ttl === 300 ? 300 : 900
  } catch { return 900 }
}
const savedTtl = readSavedTtl()
const ttlMinutesText = computed(() => savedTtl === 300 ? '5 分钟' : '15 分钟')

const remaining = ref(savedTtl)
let timer: number | null = null
function startTtl() {
  remaining.value = result.value?.ttl ?? savedTtl
  timer = window.setInterval(() => { remaining.value = Math.max(0, remaining.value - 1) }, 1000)
}
function stopTtl() { if (timer) { clearInterval(timer); timer = null } }
onBeforeUnmount(stopTtl)
const ttlText = computed(() => `${Math.floor(remaining.value / 60)}:${String(remaining.value % 60).padStart(2, '0')}`)

const copied = ref(false)
async function copySerial() {
  try {
    await navigator.clipboard.writeText(result.value?.serial || '')
    copied.value = true
    setTimeout(() => (copied.value = false), 1500)
    showToast('序列号已复制到剪贴板')
  } catch {
    showToast('复制失败 · 请手动选择序列号复制')
  }
}

async function generate() {
  if (phase.value === 'generating') return
  phase.value = 'generating'
  const selected: CandidateAddr[] = cands
    .filter((c) => c.checked)
    .map((c) => ({ kind: c.kind, value: c.desc, ready: true }))
  const r = await invoke<GenerateResult>('generate_serial', {
    candidates: selected,
    password: usePassword.value ? pwd.value : undefined,
    ttlSecs: savedTtl,
  })
  result.value = r
  phase.value = 'done'
  startTtl()
  copySerial()
}
/** 危险操作：重新生成会作废旧序列号（若已发给对方将无法连接） */
async function regen() {
  const ok = await confirm({
    title: '重新生成序列号',
    message: '当前序列号将立即失效。如果已发送给对方，对方将无法再用它连接。',
    confirmText: '重新生成',
    tone: 'warning',
  })
  if (!ok) return
  stopTtl()
  phase.value = 'form'
  result.value = null
}
function close() { stopTtl(); emit('close') }

const badge = computed(() =>
  phase.value === 'done'
    ? { text: '✓ 已生成', v: 'success' as const }
    : usePassword.value
      ? { text: '口令保护已开启', v: 'info' as const }
      : null
)
</script>

<template>
  <Modal
    title="生成外网连接序列号"
    :icon="phase === 'done' ? 'key' : 'link'"
    :badge="badge?.text"
    :badge-variant="badge?.v"
    :width="480"
    @close="close"
  >
    <template #header-extra>
      <span v-if="phase !== 'done'" class="gd-device"><Icon name="monitor" :size="12" /> {{ state.self?.deviceName || '—' }}</span>
    </template>

    <!-- ===== 表单态 ===== -->
    <template v-if="phase !== 'done'">
      <div class="ll-banner gd-intro">
        <Icon name="info" :size="14" class="ll-banner__icon" />
        <span>将生成一条自包含候选地址的序列号，通过 IM 发给对方即可直连，无需任何服务器。<br>有效期 {{ ttlMinutesText }}（设置页可调），仅可成功握手一次。</span>
      </div>

      <!-- 候选地址 -->
      <div class="ll-card gd-card">
        <div class="gd-card-head">
          <span class="ll-section-title"><Icon name="package" :size="13" class="gd-ico" /> 将打包的候选地址</span>
          <span class="ll-text-tertiary ll-mono">{{ selectedCount }} / 4 条</span>
        </div>
        <label v-for="c in cands" :key="c.kind" class="gd-cand" :class="{ 'is-off': c.optional }">
          <span class="gd-check" :class="{ on: c.checked }" @click.prevent="c.checked = !c.checked">
            <Icon v-if="c.checked" name="check" :size="10" :width="3" />
          </span>
          <span v-if="c.label" class="ll-tag" :class="c.kind === 'Pub6' ? 'll-tag--brand' : 'll-tag--plain'">{{ c.label }}</span>
          <span class="gd-cand-desc">{{ c.desc }}</span>
          <span v-if="c.optional" class="ll-tag ll-tag--warning">不推荐</span>
          <span v-else class="gd-ready">已就绪</span>
          <input v-model="c.checked" type="checkbox" class="sr-only" />
        </label>
      </div>

      <!-- 口令保护 -->
      <div class="ll-card gd-card gd-pwd-card" :class="{ 'is-on': usePassword }">
        <div class="gd-card-head">
          <span class="ll-section-title"><Icon name="lock" :size="13" class="gd-ico" /> 口令保护</span>
          <span class="ll-text-tertiary">{{ usePassword ? 'PBKDF2 10000 次 · AES-GCM-256' : 'AES-GCM-256' }}</span>
        </div>
        <p class="gd-pwd-desc ll-text-tertiary">启用后需 AES-GCM-256 解密，无口令者无法还原地址</p>
        <label class="gd-pwd-toggle">
          <input v-model="usePassword" type="checkbox" />
          <span>启用口令保护（可选）</span>
        </label>
        <p v-if="!usePassword" class="gd-pwd-hint ll-text-tertiary">开启后需设置 4–6 位数字口令并单独告知对方</p>

        <template v-if="usePassword">
          <p class="gd-rule">设置 4–6 位数字口令，纯数字、不可含字母</p>
          <div class="gd-pwd-fields">
            <div class="ll-field">
              <label>设置 4–6 位数字口令</label>
              <div class="ll-input-wrap">
                <input v-model="pwd" class="ll-input ll-input--mono" :type="showPwd ? 'text' : 'password'" maxlength="6" placeholder="●●●●●●" />
                <span class="ll-input-affix ll-click" @click="showPwd = !showPwd"><Icon :name="showPwd ? 'eyeOff' : 'eye'" :size="13" /></span>
              </div>
            </div>
            <div class="ll-field">
              <label>再次确认</label>
              <input v-model="pwd2" class="ll-input ll-input--mono" :type="showPwd ? 'text' : 'password'" maxlength="6" placeholder="●●●●●●" />
            </div>
          </div>
          <p v-if="pwdHint.text" class="gd-pwd-state" :class="pwdHint.ok ? 'ok' : 'bad'">
            <Icon :name="pwdHint.ok ? 'check' : 'alertCircle'" :size="11" /> {{ pwdHint.text }}
          </p>

          <div class="gd-chain">
            <div class="gd-chain-title">加密链路</div>
            <div class="gd-chain-flow">
              <span>52B 二进制 Payload</span><Icon name="arrowRight" :size="12" />
              <span>PBKDF2 32B 密钥</span><Icon name="arrowRight" :size="12" />
              <span>AES-GCM-256</span><Icon name="arrowRight" :size="12" />
              <span>密文 + 16B Tag</span><Icon name="arrowRight" :size="12" />
              <span>Base32 输出</span>
            </div>
            <p class="ll-text-tertiary">口令请单独发一条消息，切勿与序列号一起发送。输错仅提示「口令错误或序列号被篡改」；派生密钥不使用即清除。</p>
          </div>
        </template>
      </div>

      <button
        class="ll-btn ll-btn--primary ll-btn--block gd-generate"
        :disabled="phase === 'generating' || (usePassword && !pwdValid)"
        @click="generate"
      >
        <span v-if="phase === 'generating'" class="ll-spinner" />
        <Icon v-else :name="usePassword ? 'lock' : 'key'" :size="14" />
        {{ usePassword ? '加密并生成序列号' : '生成序列号' }}
      </button>

      <div class="gd-result-empty">
        <div class="gd-result-head"><span>外网序列号</span><span class="ll-text-tertiary">—</span></div>
        <div class="gd-result-box">
          <Icon name="key" :size="16" />
          <span>{{ usePassword ? '尚未生成 · 将加密后输出 Base32 序列号' : '尚未生成 · 点击下方按钮生成一次性连接序列号' }}</span>
        </div>
      </div>

      <div class="gd-warn">
        <Icon name="alertTriangle" :size="13" />
        <span>仅使用最新一次生成的序列号；关闭弹窗即内存销毁，请先复制再关闭</span>
      </div>
      <div v-if="usePassword" class="gd-warn">
        <Icon name="alertTriangle" :size="13" />
        <span>口令请通过另一条消息单独发送，避免与序列号同时泄露</span>
      </div>
      <div v-if="usePassword" class="gd-warn">
        <Icon name="alertTriangle" :size="13" />
        <span>加密仅保护地址与时间戳；防重放仍依赖 TTL + 会话黑名单 + 动态质询</span>
      </div>
    </template>

    <!-- ===== 完成态 ===== -->
    <template v-else>
      <div class="ll-banner ll-banner--success gd-done-banner">
        <span class="gd-done-check"><Icon name="check" :size="14" :width="2.4" /></span>
        <div class="ll-col">
          <b>序列号已复制到剪贴板，可直接粘贴到微信发送</b>
          <span class="ll-text-tertiary"><Icon name="clock" :size="11" /> {{ ttlText }} 后失效</span>
        </div>
      </div>

      <div class="gd-result-head">
        <span>外网序列号</span>
        <span class="ll-text-tertiary">{{ result?.encrypted ? '已加密' : '未加密' }} / 未压缩 / {{ result?.base32Chars || 84 }} 字符 · 14 组</span>
      </div>
      <div class="gd-serial ll-mono">{{ result?.serial }}</div>
      <div class="gd-serial-foot ll-text-tertiary">
        <span>12 组 / 每 6 字符一组，超长横向滚动不折行</span>
        <span>生成于 10:33:12</span>
      </div>

      <div class="gd-actions">
        <button class="ll-btn ll-btn--primary" @click="copySerial"><Icon name="copy" :size="13" /> {{ copied ? '已复制' : '复制序列号' }}</button>
        <button class="ll-btn ll-btn--default" @click="regen"><Icon name="refresh" :size="13" /> 重新生成</button>
        <button class="ll-btn ll-btn--default" @click="close">关闭</button>
      </div>

      <div class="ll-card gd-param">
        <div class="gd-param-title">凭证参数</div>
        <div class="ll-kv"><span class="ll-kv__k">session</span><span class="ll-kv__v ll-mono">{{ result?.session?.slice(0, 16) }}</span></div>
        <div class="ll-kv"><span class="ll-kv__k">候选地址</span><span class="ll-kv__v">Pub6×1 / Pub4×1</span></div>
        <div class="ll-kv"><span class="ll-kv__k">Payload</span><span class="ll-kv__v">52 字节 → Base32 84 字符（JSON 约 200 字节）</span></div>
        <div class="ll-kv"><span class="ll-kv__k">有效期</span><span class="ll-kv__v">{{ result?.ttl }}s · 握手成功即作废</span></div>
      </div>

      <div class="gd-warn"><Icon name="alertTriangle" :size="13" /><span>仅使用最新一次生成的序列号；关闭弹窗即内存销毁，请先复制再关闭</span></div>
      <div class="gd-warn"><Icon name="alertTriangle" :size="13" /><span>口令请通过另一条消息单独发送，避免与序列号同时泄露</span></div>
    </template>

    <!-- 页脚 -->
    <div class="gd-footer">
      <span><Icon name="lock" :size="11" /> 台账不落盘：本次生成的凭证不会写入任何本地文件</span>
      <span class="gd-footer-right">
        <button v-if="phase === 'done'" class="gd-wait-link" @click="openDialog('waitConnect'); emit('close')">等待对方连接 ↗</button>
        <span class="ll-text-tertiary">协议 v1</span>
      </span>
    </div>
  </Modal>
</template>

<style scoped>
.gd-device { display: inline-flex; align-items: center; gap: var(--ll-space-1); font-size: var(--ll-fs-xs); color: var(--ll-color-text-tertiary); background: var(--ll-color-surface-3); border-radius: var(--ll-radius-xs); padding: 2px 6px; }
.gd-intro { margin-bottom: var(--ll-space-3); font-size: var(--ll-fs-xs); }
.gd-card { padding: var(--ll-space-3); margin-bottom: var(--ll-space-3); }
.gd-card-head { display: flex; align-items: center; justify-content: space-between; gap: var(--ll-space-2); margin-bottom: var(--ll-space-2); font-size: var(--ll-fs-xs); }
.gd-card-head .ll-section-title { display: inline-flex; align-items: center; gap: var(--ll-space-2); }
.gd-ico { color: var(--ll-color-text-tertiary); }
.gd-cand { display: flex; align-items: center; gap: var(--ll-space-2); padding: var(--ll-space-2) var(--ll-space-1); border-top: 1px solid var(--ll-color-divider); font-size: var(--ll-fs-xs); cursor: pointer; }
.gd-cand:first-of-type { border-top: none; }
.gd-cand.is-off { color: var(--ll-color-text-tertiary); }
.gd-check { width: 15px; height: 15px; flex: none; display: inline-flex; align-items: center; justify-content: center; border: 1px solid var(--ll-color-border-strong); border-radius: 3px; background: var(--ll-color-surface); color: #fff; }
.gd-check.on { background: var(--ll-color-success); border-color: var(--ll-color-success); }
.gd-cand-desc { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.gd-ready { flex: none; color: var(--ll-color-success); font-size: 10px; }
.sr-only { position: absolute; width: 1px; height: 1px; opacity: 0; }

.gd-pwd-card.is-on { border-color: var(--ll-color-brand); }
.gd-pwd-desc { font-size: var(--ll-fs-xs); margin-bottom: var(--ll-space-2); }
.gd-pwd-toggle { display: inline-flex; align-items: center; gap: var(--ll-space-2); font-size: var(--ll-fs-xs); color: var(--ll-color-text-secondary); cursor: pointer; }
.gd-pwd-toggle input { accent-color: var(--ll-color-brand); margin: 0; }
.gd-pwd-hint { font-size: 10px; margin-top: var(--ll-space-2); }
.gd-rule { font-size: var(--ll-fs-xs); color: var(--ll-color-brand); margin: var(--ll-space-3) 0 var(--ll-space-2); }
.gd-pwd-fields { display: grid; grid-template-columns: 1fr 1fr; gap: var(--ll-space-2); }
.gd-pwd-fields label { font-size: 10px; }
.gd-pwd-state { display: inline-flex; align-items: center; gap: var(--ll-space-1); font-size: var(--ll-fs-xs); margin-top: var(--ll-space-2); }
.gd-pwd-state.ok { color: var(--ll-color-success); }
.gd-pwd-state.bad { color: var(--ll-color-danger); }
.gd-chain { margin-top: var(--ll-space-3); padding-top: var(--ll-space-3); border-top: 1px solid var(--ll-color-divider); }
.gd-chain-title { font-size: var(--ll-fs-xs); font-weight: var(--ll-fw-semibold); margin-bottom: var(--ll-space-2); }
.gd-chain-flow { display: flex; flex-wrap: wrap; align-items: center; gap: var(--ll-space-1); margin-bottom: var(--ll-space-2); }
.gd-chain-flow span { font-size: 10px; color: var(--ll-color-brand); background: var(--ll-color-brand-light); border-radius: var(--ll-radius-xs); padding: 2px 5px; }
.gd-chain-flow svg { color: var(--ll-color-text-quaternary); }
.gd-chain p { font-size: 10px; line-height: var(--ll-lh-relaxed); }

.gd-generate { height: 34px; margin-bottom: var(--ll-space-3); }
.gd-result-empty { margin-bottom: var(--ll-space-3); }
.gd-result-head { display: flex; align-items: center; justify-content: space-between; font-size: var(--ll-fs-xs); margin-bottom: var(--ll-space-2); color: var(--ll-color-text-secondary); }
.gd-result-box { display: flex; align-items: center; justify-content: center; gap: var(--ll-space-2); min-height: 62px; padding: var(--ll-space-3); border: 1px dashed var(--ll-color-border-strong); border-radius: var(--ll-radius-md); background: var(--ll-color-surface-2); color: var(--ll-color-text-quaternary); font-size: var(--ll-fs-xs); text-align: center; }
.gd-warn { display: flex; align-items: flex-start; gap: var(--ll-space-2); padding: var(--ll-space-2) var(--ll-space-3); margin-top: var(--ll-space-2); background: var(--ll-color-warning-light); border: 1px solid var(--ll-color-warning); border-radius: var(--ll-radius-sm); color: var(--ll-color-warning); font-size: var(--ll-fs-xs); line-height: var(--ll-lh-relaxed); }
.gd-warn svg { flex: none; margin-top: 1px; }

.gd-done-banner { align-items: center; margin-bottom: var(--ll-space-4); }
.gd-done-banner b { font-size: var(--ll-fs-sm); color: var(--ll-color-success); }
.gd-done-banner .ll-text-tertiary { display: inline-flex; align-items: center; gap: var(--ll-space-1); font-size: var(--ll-fs-xs); }
.gd-done-check { width: 22px; height: 22px; flex: none; border-radius: 50%; background: var(--ll-color-success); color: #fff; display: inline-flex; align-items: center; justify-content: center; }
.gd-serial { padding: var(--ll-space-3); background: var(--ll-serial-bg); border: 1px solid var(--ll-color-border); border-radius: var(--ll-radius-sm); font-size: var(--ll-fs-md); line-height: 1.9; color: var(--ll-color-serial-text); overflow-x: auto; white-space: normal; word-break: break-all; }
.gd-serial-foot { display: flex; align-items: center; justify-content: space-between; font-size: 10px; margin-top: var(--ll-space-2); }
.gd-actions { display: flex; gap: var(--ll-space-2); margin: var(--ll-space-3) 0; }
.gd-actions .ll-btn--primary { flex: 1.4; }
.gd-actions .ll-btn--default { flex: 1; }
.gd-param { padding: var(--ll-space-3); }
.gd-param-title { font-size: var(--ll-fs-sm); font-weight: var(--ll-fw-semibold); margin-bottom: var(--ll-space-2); }
.gd-param .ll-kv__k { width: 72px; }
.gd-footer { display: flex; align-items: center; justify-content: space-between; gap: var(--ll-space-2); margin-top: var(--ll-space-4); font-size: 10px; color: var(--ll-color-text-quaternary); }
.gd-footer > span:first-child { display: inline-flex; align-items: center; gap: var(--ll-space-1); }
.gd-footer-right { display: inline-flex; align-items: center; gap: var(--ll-space-3); }
.gd-wait-link { border: none; background: transparent; color: var(--ll-color-brand); font-size: 10px; cursor: pointer; padding: 0; }
.gd-wait-link:hover { text-decoration: underline; }
</style>
