<script setup lang="ts">
import type { VerifyStep } from '../../types/invite'
import Icon from './Icon.vue'

withDefaults(
  defineProps<{
    steps: VerifyStep[]
    /** 为空时不渲染状态行（成功态） */
    statusText?: string
    timeoutText?: string
    statusTone?: 'idle' | 'active' | 'done' | 'error'
  }>(),
  {
    statusText: '就绪，粘贴序列号后点击连接',
    timeoutText: '质询超时 10s',
    statusTone: 'idle',
  }
)
</script>

<template>
  <div class="verify-capsule" :class="{ 'no-foot': !statusText }">
    <!-- 原型：状态行在胶囊上方 -->
    <div v-if="statusText" class="vc-foot">
      <span class="vc-status" :class="`tone-${statusTone}`">
        <span class="ll-dot" :class="statusTone === 'done' ? 'll-dot--success' : statusTone === 'error' ? 'll-dot--danger' : statusTone === 'active' ? 'll-dot--brand' : ''" />
        {{ statusText }}
      </span>
      <span v-if="timeoutText" class="vc-timeout ll-text-tertiary">
        <Icon name="clock" :size="11" /> {{ timeoutText }}
      </span>
    </div>
    <!-- 一行六枚胶囊 -->
    <div class="vc-grid">
      <div v-for="s in steps" :key="s.key" class="vc-item ll-capsule" :class="`is-${s.state}`">
        <span v-if="s.state === 'done'" class="vc-mark"><Icon name="check" :size="11" :width="2.4" /></span>
        <span v-else-if="s.state === 'error'" class="vc-mark"><Icon name="close" :size="11" :width="2.4" /></span>
        <span v-else-if="s.state === 'active'" class="ll-spinner ll-spinner--xs" />
        <span v-else class="vc-mark vc-mark--idle">○</span>
        <span>{{ s.label }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.verify-capsule {
  padding: var(--ll-space-3);
  background: var(--ll-color-surface-2);
  border: 1px solid var(--ll-color-border);
  border-radius: var(--ll-radius-md);
}
.vc-foot {
  display: flex; align-items: center; justify-content: space-between;
  gap: var(--ll-space-3); margin-bottom: var(--ll-space-3);
  font-size: var(--ll-fs-xs);
}
.vc-status { display: inline-flex; align-items: center; gap: var(--ll-space-2); color: var(--ll-color-text-secondary); }
.vc-status.tone-error { color: var(--ll-color-danger); }
.vc-status.tone-done { color: var(--ll-color-success); }
.vc-status.tone-active { color: var(--ll-color-brand); }
.vc-timeout { display: inline-flex; align-items: center; gap: var(--ll-space-1); white-space: nowrap; }

.vc-grid { display: grid; grid-template-columns: repeat(6, 1fr); gap: var(--ll-space-2); }
.vc-item {
  width: 100%;
  height: 26px;
  border-radius: var(--ll-radius-sm);
  background: var(--ll-color-surface);
  justify-content: flex-start;
  font-size: var(--ll-fs-xs);
}
.vc-item.is-active { color: var(--ll-color-brand); background: var(--ll-color-brand-light); border-color: var(--ll-color-brand); }
.vc-item.is-done { color: var(--ll-color-success); background: var(--ll-color-success-light); border-color: var(--ll-color-success); }
.vc-item.is-error { color: var(--ll-color-danger); background: var(--ll-color-danger-light); border-color: var(--ll-color-danger); }
.vc-mark { display: inline-flex; align-items: center; flex: none; }
.vc-mark--idle { font-size: 11px; color: var(--ll-color-text-quaternary); }
.ll-spinner--xs { width: 11px; height: 11px; border-width: 1.5px; }
</style>
