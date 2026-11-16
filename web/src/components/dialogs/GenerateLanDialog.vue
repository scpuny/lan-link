<script setup lang="ts">
import { ref, computed, onBeforeUnmount } from 'vue'
import Modal from '../common/Modal.vue'
import Icon from '../common/Icon.vue'
import { useUi } from '../../composables/useUi'
import { invoke } from '../../bridge'
import type { LanCodeResult } from '../../types/invite'

const { state, showToast } = useUi()
const emit = defineEmits<{ (e: 'close'): void }>()

type Phase = 'idle' | 'generating' | 'done'
const phase = ref<Phase>('idle')
const result = ref<LanCodeResult | null>(null)
const copied = ref(false)

async function generate() {
  if (phase.value === 'generating') return
  phase.value = 'generating'
  try {
    const r = await invoke<LanCodeResult>('generate_lan_code')
    result.value = r
    phase.value = 'done'
    await copyCode()
  } catch (e) {
    phase.value = 'idle'
    showToast?.(String(e))
  }
}

async function copyCode() {
  try {
    await navigator.clipboard.writeText(result.value?.code || '')
    copied.value = true
    setTimeout(() => (copied.value = false), 1500)
  } catch { /* ignore */ }
}

function close() { emit('close') }

// 运行期计时（仅展示用；短码本身不设 TTL，退出程序即失效）
const elapsed = ref(0)
let timer: number | null = null
function startTimer() {
  timer = window.setInterval(() => { elapsed.value += 1 }, 1000)
}
function stopTimer() { if (timer) { clearInterval(timer); timer = null } }
onBeforeUnmount(stopTimer)

const elapsedText = computed(() => `${Math.floor(elapsed.value / 60)}:${String(elapsed.value % 60).padStart(2, '0')}`)
const badge = computed(() => phase.value === 'done' ? { text: '✓ 广播中', v: 'success' as const } : null)
</script>

<template>
  <Modal
    title="生成局域网短码"
    :icon="phase === 'done' ? 'key' : 'lan'"
    :badge="badge?.text"
    :badge-variant="badge?.v"
    :width="460"
    @close="close"
  >
    <template #header-extra>
      <span class="gl-device"><Icon name="monitor" :size="12" /> {{ state.self?.deviceName || '本机' }}</span>
    </template>

    <!-- 生成前 -->
    <template v-if="phase !== 'done'">
      <div class="ll-banner gl-intro">
        <Icon name="info" :size="14" class="ll-banner__icon" />
        <span>生成 6 位短码，通过 mDNS 广播 + 子网探测发现对端。<b>不携带任何地址</b>，断外网也能直连，仅双方同一局域网时可用。</span>
      </div>

      <div class="ll-card gl-info-card">
        <div class="gl-info-row">
          <span class="ll-section-title"><Icon name="wifi" :size="13" /> 发现方式</span>
          <span class="ll-text-tertiary">mDNS 广播 · 端口 42210</span>
        </div>
        <div class="gl-info-row">
          <span class="ll-text-secondary">有效期</span>
          <span>程序运行期内有效，退出即失效</span>
        </div>
        <div class="gl-info-row">
          <span class="ll-text-secondary">候选地址</span>
          <span class="ll-mono gl-cands">仅本机局域网地址，不含公网端点</span>
        </div>
        <div class="gl-info-row">
          <span class="ll-text-secondary">安全</span>
          <span>无需口令；握手仍走质询应答防重放</span>
        </div>
      </div>

      <button
        class="ll-btn ll-btn--primary ll-btn--block gl-generate"
        :disabled="phase === 'generating'"
        @click="generate"
      >
        <span v-if="phase === 'generating'" class="ll-spinner" />
        <Icon v-else name="lan" :size="14" />
        {{ phase === 'generating' ? '正在生成并广播…' : '生成短码' }}
      </button>

      <div class="gl-warn"><Icon name="alertTriangle" :size="13" /> 跨网段 / 跨 VLAN 无法发现对方，请改用外网序列号连接</div>
    </template>

    <!-- 生成后 -->
    <template v-else>
      <div class="ll-banner ll-banner--success gl-done-banner">
        <span class="gl-done-check"><Icon name="check" :size="14" :width="2.4" /></span>
        <span>短码已生成，正在局域网内广播，等待对方输入连接</span>
      </div>

      <div class="gl-code-label">邀请短码（6 位）</div>
      <div class="gl-code-box">
        <span class="gl-code">{{ result?.code }}</span>
        <button class="ll-btn ll-btn--default ll-btn--sm" @click="copyCode">
          <Icon :name="copied ? 'check' : 'copy'" :size="13" /> {{ copied ? '已复制' : '复制' }}
        </button>
      </div>

      <div class="gl-status-grid">
        <div class="gl-status-item"><span class="ll-text-tertiary">广播状态</span><span class="gl-online"><span class="ll-dot ll-dot--success" /> mDNS 广播中</span></div>
        <div class="gl-status-item"><span class="ll-text-tertiary">已等待</span><span class="ll-mono">{{ elapsedText }}</span></div>
        <div class="gl-status-item"><span class="ll-text-tertiary">局域网地址</span><span class="ll-mono">{{ result?.candidates.map(c => c.value).join(' · ') || '—' }}</span></div>
        <div class="gl-status-item"><span class="ll-text-tertiary">有效期</span><span>程序运行期 · 无倒计时</span></div>
      </div>

      <div class="gl-steps">
        <div class="gl-step done"><span class="gl-step-n"><Icon name="check" :size="10" /></span> 生成 6 位短码</div>
        <div class="gl-step active"><span class="gl-step-n">2</span> 广播并等待对方连接</div>
        <div class="gl-step"><span class="gl-step-n">3</span> 质询握手 · 直连传输</div>
      </div>

      <div class="gl-warn"><Icon name="alertTriangle" :size="13" /> 关闭弹窗即停止广播；请先复制短码再关闭</div>
    </template>
  </Modal>
</template>

<style scoped>
.gl-device { display: inline-flex; align-items: center; gap: 4px; font-size: var(--ll-fs-xs); color: var(--ll-color-text-tertiary); }
.gl-intro { margin-bottom: var(--ll-space-4); line-height: 1.6; }
.gl-intro b { color: var(--ll-color-text); }
.gl-info-card { padding: 4px 14px; margin-bottom: var(--ll-space-4); }
.gl-info-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 38px; border-bottom: 1px solid var(--ll-color-border); font-size: var(--ll-fs-sm); }
.gl-info-row:last-child { border-bottom: none; }
.gl-cands { font-size: var(--ll-fs-xs); }
.gl-generate { margin-top: 4px; }
.gl-warn { display: flex; align-items: flex-start; gap: 6px; margin-top: 12px; font-size: var(--ll-fs-xs); color: var(--ll-color-text-tertiary); line-height: 1.5; }
.gl-warn :deep(svg) { flex: none; margin-top: 1px; }
.gl-done-banner { margin-bottom: var(--ll-space-4); }
.gl-done-check { display: inline-flex; align-items: center; justify-content: center; width: 20px; height: 20px; border-radius: 50%; background: var(--ll-color-success); color: #fff; flex: none; }
.gl-code-label { font-size: var(--ll-fs-sm); color: var(--ll-color-text-secondary); margin-bottom: 6px; }
.gl-code-box { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 14px 18px; border: 1px solid var(--ll-color-border-strong); border-radius: var(--ll-radius-md); background: var(--ll-color-surface-2); margin-bottom: var(--ll-space-4); }
.gl-code { font-family: var(--ll-font-mono, ui-monospace, monospace); font-size: 30px; font-weight: 700; letter-spacing: 6px; color: var(--ll-color-brand); }
.gl-status-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1px; background: var(--ll-color-border); border: 1px solid var(--ll-color-border); border-radius: var(--ll-radius-md); overflow: hidden; margin-bottom: var(--ll-space-4); }
.gl-status-item { display: flex; flex-direction: column; gap: 4px; padding: 10px 12px; background: var(--ll-color-surface); font-size: var(--ll-fs-sm); }
.gl-online { display: inline-flex; align-items: center; gap: 5px; }
.gl-steps { display: flex; flex-direction: column; gap: 8px; }
.gl-step { display: flex; align-items: center; gap: 8px; font-size: var(--ll-fs-sm); color: var(--ll-color-text-tertiary); }
.gl-step.done { color: var(--ll-color-success); }
.gl-step.active { color: var(--ll-color-text); }
.gl-step-n { display: inline-flex; align-items: center; justify-content: center; width: 18px; height: 18px; border-radius: 50%; border: 1px solid currentColor; font-size: 10px; flex: none; }
</style>
