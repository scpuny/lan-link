<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import Modal from '../common/Modal.vue'
import Icon from '../common/Icon.vue'

const emit = defineEmits<{ (e: 'close'): void }>()

const remain = ref(869) // 14:29
const elapsed = ref(9.4)
let timer: number | null = null

onMounted(() => {
  timer = window.setInterval(() => {
    remain.value = Math.max(0, remain.value - 1)
    elapsed.value = Math.min(10, elapsed.value + 0.1)
  }, 1000)
})
onBeforeUnmount(() => { if (timer) clearInterval(timer) })

const remainText = computed(() => `${Math.floor(remain.value / 60)}:${String(remain.value % 60).padStart(2, '0')}`)

const steps = [
  { label: '等待连接', state: 'done' },
  { label: '收到请求', state: 'done' },
  { label: '接收质询', state: 'done' },
  { label: '回传应答', state: 'active' },
  { label: '校验通过', state: 'idle' },
] as const

const serial = 'K7G2S9-5MNP8R-2HJK7T-9QG3S5-4WXY6A-8BCD3F-6NPQ2R-9STV4K-3HJL7M-5XCV8B-2DFG4N-7QWE9P-1AZX3C-HN8P3J'
</script>

<template>
  <Modal title="等待对方连接" icon="link" badge="握手中" badge-variant="brand" :width="540" @close="emit('close')">
    <!-- 对端提示 -->
    <div class="ll-banner wc-peer">
      <Icon name="info" :size="14" class="ll-banner__icon" />
      <div class="ll-col">
        <b>对方已收到序列号并发起连接</b>
        <span class="ll-text-tertiary">来自 [2408:8207:…:9c1d]:51820 · 已接收 16 字节质询</span>
      </div>
      <span class="ll-text-tertiary">协议 v1</span>
    </div>

    <!-- 五步进度 -->
    <div class="wc-steps">
      <template v-for="(s, i) in steps" :key="s.label">
        <div class="wc-step" :class="`is-${s.state}`">
          <span class="ws-circle">
            <Icon v-if="s.state === 'done'" name="check" :size="11" :width="2.6" />
            <span v-else-if="s.state === 'active'" class="ll-spinner ll-spinner--xs" />
            <span v-else class="ws-idx">{{ i + 1 }}</span>
          </span>
          <span class="ws-label">{{ s.label }}</span>
        </div>
        <span v-if="i < steps.length - 1" class="ws-line" :class="{ done: s.state === 'done' }" />
      </template>
    </div>

    <!-- Blake3 应答计算卡 -->
    <div class="ll-card wc-card">
      <div class="wc-card-head">
        <span class="ll-section-title"><Icon name="activity" :size="13" class="wc-ico" /> Blake3 应答计算</span>
        <span class="ll-text-tertiary">耗时 3ms</span>
      </div>
      <div class="wc-input">输入　session_id ‖ challenge</div>
      <div class="ll-kv"><span class="ll-kv__k">challenge</span><span class="ll-kv__v ll-mono">a7f2c94de1b8035 <small class="ll-text-tertiary">（16B · 一次性）</small></span></div>
      <div class="ll-kv"><span class="ll-kv__k">应答摘要</span><span class="ll-kv__v ll-mono">3e9b71c4d8f2a605 <small class="wc-sending">（32B · ↗ 回传中）</small></span></div>
      <div class="ll-kv"><span class="ll-kv__k">数据链路</span><span class="ll-kv__v">TCP 已连接 · 往返 38ms · 无中继</span></div>
    </div>

    <!-- 序列号 -->
    <div class="wc-serial">
      <div class="wc-serial-head"><span>本次序列号</span><span class="ll-text-tertiary"><Icon name="clock" :size="11" /> 有效期剩余 {{ remainText }}</span></div>
      <div class="wc-serial-box ll-mono">{{ serial }}</div>
    </div>

    <!-- 按钮 -->
    <div class="wc-actions">
      <button class="ll-btn ll-btn--danger-outline" @click="emit('close')"><Icon name="close" :size="13" /> 中断握手</button>
      <button class="ll-btn ll-btn--default" @click="emit('close')"><Icon name="minus" :size="13" /> 转到后台</button>
    </div>

    <!-- 蓝色进度横幅 -->
    <div class="ll-banner wc-progress">
      <span class="ll-spinner" />
      <div class="ll-col">
        <b>正在计算并回传质询应答…</b>
        <span class="ll-text-tertiary">对端需在 10s 内完成摘要比对，超时本轮作废</span>
      </div>
      <span class="ll-mono wc-timer">计时 {{ elapsed.toFixed(1) }}s</span>
    </div>

    <div class="wc-footer"><Icon name="lock" :size="11" /> 应答摘要与 challenge 绑定，旧报文无法重放</div>
  </Modal>
</template>

<style scoped>
.wc-peer { align-items: center; margin-bottom: var(--ll-space-4); }
.wc-peer b { font-size: var(--ll-fs-sm); color: var(--ll-color-brand); }
.wc-peer .ll-col { flex: 1; }
.wc-peer .ll-text-tertiary { font-size: var(--ll-fs-xs); }
.wc-peer > span:last-child { font-size: var(--ll-fs-xs); white-space: nowrap; }

.wc-steps { display: flex; align-items: center; margin-bottom: var(--ll-space-4); }
.wc-step { display: flex; flex-direction: column; align-items: center; gap: var(--ll-space-1); flex: none; min-width: 62px; }
.ws-circle { width: 22px; height: 22px; border-radius: 50%; display: inline-flex; align-items: center; justify-content: center; border: 1px solid var(--ll-color-border-strong); background: var(--ll-color-surface); color: #fff; }
.is-done .ws-circle { background: var(--ll-color-success); border-color: var(--ll-color-success); }
.is-active .ws-circle { background: var(--ll-color-brand); border-color: var(--ll-color-brand); color: var(--ll-color-text-inverse); }
.ws-idx { color: var(--ll-color-text-tertiary); font-size: 10px; }
.ll-spinner--xs { width: 11px; height: 11px; border-width: 1.5px; border-top-color: var(--ll-color-text-inverse); }
.ws-label { font-size: 10px; white-space: nowrap; color: var(--ll-color-text-tertiary); }
.is-done .ws-label { color: var(--ll-color-success); }
.is-active .ws-label { color: var(--ll-color-brand); font-weight: var(--ll-fw-medium); }
.ws-line { flex: 1; height: 1px; background: var(--ll-color-border); margin: 0 2px 14px; }
.ws-line.done { background: var(--ll-color-success); }

.wc-card { padding: var(--ll-space-3); margin-bottom: var(--ll-space-3); }
.wc-card-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--ll-space-2); font-size: var(--ll-fs-xs); }
.wc-card-head .ll-section-title { display: inline-flex; align-items: center; gap: var(--ll-space-2); }
.wc-ico { color: var(--ll-color-brand); }
.wc-card .ll-kv__k { width: 72px; }
.wc-input { margin-bottom: var(--ll-space-2); padding: var(--ll-space-2) var(--ll-space-3); background: var(--ll-color-surface-2); border-radius: var(--ll-radius-sm); font-size: var(--ll-fs-xs); color: var(--ll-color-text-secondary); }
.wc-sending { color: var(--ll-color-brand); }

.wc-serial { margin-bottom: var(--ll-space-3); }
.wc-serial-head { display: flex; align-items: center; justify-content: space-between; font-size: var(--ll-fs-xs); margin-bottom: var(--ll-space-2); color: var(--ll-color-text-secondary); }
.wc-serial-head > span:last-child { display: inline-flex; align-items: center; gap: var(--ll-space-1); }
.wc-serial-box { padding: var(--ll-space-2) var(--ll-space-3); background: var(--ll-serial-bg); border: 1px solid var(--ll-color-border); border-radius: var(--ll-radius-sm); font-size: var(--ll-fs-xs); line-height: 1.8; color: var(--ll-color-serial-text); word-break: break-all; }

.wc-actions { display: flex; gap: var(--ll-space-2); margin-bottom: var(--ll-space-3); }
.wc-actions .ll-btn { flex: 1; }
.wc-progress { align-items: center; }
.wc-progress b { font-size: var(--ll-fs-sm); color: var(--ll-color-brand); }
.wc-progress .ll-col { flex: 1; }
.wc-progress .ll-text-tertiary { font-size: var(--ll-fs-xs); }
.wc-timer { font-size: var(--ll-fs-xs); color: var(--ll-color-brand); white-space: nowrap; }
.wc-footer { display: flex; align-items: center; gap: var(--ll-space-1); margin-top: var(--ll-space-3); font-size: 10px; color: var(--ll-color-text-quaternary); }
</style>
