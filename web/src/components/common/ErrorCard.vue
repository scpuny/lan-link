<script setup lang="ts">
import type { ProtocolError } from '../../types/invite'
import Icon from './Icon.vue'

defineProps<{ error: ProtocolError }>()
</script>

<template>
  <div class="err-card" :class="`is-${error.severity}`">
    <div class="err-head">
      <Icon :name="error.severity === 'warning' ? 'alertTriangle' : 'alertCircle'" :size="15" class="err-ico" />
      <span class="err-title">{{ error.title }}</span>
      <span class="err-code">{{ error.code }}</span>
    </div>
    <div class="err-row"><span class="err-key">原因</span><span class="err-val">{{ error.reason }}</span></div>
    <div class="err-row"><span class="err-key">建议</span><span class="err-val">{{ error.suggestion }}</span></div>
    <div class="err-row"><span class="err-key">说明</span><span class="err-val">{{ error.note }}</span></div>

    <div class="err-banner">
      <span class="ll-dot" :class="error.severity === 'warning' ? 'll-dot--warning' : 'll-dot--danger'" />
      <span>{{ error.banner }}</span>
      <span class="err-banner-right">
        <span v-if="error.bannerNote" class="err-banner-note">{{ error.bannerNote }}</span>
        <span class="err-banner-tag ll-mono">{{ error.bannerTag }}</span>
      </span>
    </div>
  </div>
</template>

<style scoped>
.err-card {
  border-radius: var(--ll-radius-md);
  padding: var(--ll-space-3);
  border: 1px solid var(--ll-color-danger);
  background: var(--ll-color-danger-light);
}
.err-card.is-warning { border-color: var(--ll-color-warning); background: var(--ll-color-warning-light); }
.err-head { display: flex; align-items: center; gap: var(--ll-space-2); margin-bottom: var(--ll-space-3); }
.err-ico { color: var(--ll-color-danger); flex: none; }
.is-warning .err-ico { color: var(--ll-color-warning); }
.err-title { flex: 1; font-weight: var(--ll-fw-semibold); font-size: var(--ll-fs-sm); color: var(--ll-color-danger); }
.is-warning .err-title { color: var(--ll-color-warning); }
.err-code {
  font-family: var(--ll-font-mono);
  font-weight: var(--ll-fw-bold);
  font-size: var(--ll-fs-xs);
  padding: 2px 6px;
  border-radius: var(--ll-radius-xs);
  background: var(--ll-color-danger);
  color: #fff;
}
.is-warning .err-code { background: var(--ll-color-warning); }
.err-row { display: flex; gap: var(--ll-space-3); font-size: var(--ll-fs-xs); margin-top: 5px; }
.err-key { color: var(--ll-color-text-tertiary); flex: none; width: 30px; }
.err-val { color: var(--ll-color-text-secondary); line-height: var(--ll-lh-relaxed); }

.err-banner {
  display: flex;
  align-items: center;
  gap: var(--ll-space-2);
  margin-top: var(--ll-space-3);
  padding: var(--ll-space-2) var(--ll-space-3);
  font-size: var(--ll-fs-xs);
  color: var(--ll-color-danger);
  background: var(--ll-color-danger-bg);
  border: 1px solid var(--ll-color-danger);
  border-radius: var(--ll-radius-sm);
}
.is-warning .err-banner { color: var(--ll-color-warning); background: var(--ll-color-warning-bg); border-color: var(--ll-color-warning); }
.err-banner-right { display: inline-flex; align-items: center; gap: var(--ll-space-3); margin-left: auto; white-space: nowrap; }
.err-banner-note { font-size: 10px; opacity: 0.85; }
.err-banner-tag { font-size: 10px; opacity: 0.8; }
</style>
