<script setup lang="ts">
import { computed } from 'vue'
import Icon from '../common/Icon.vue'
import { useUi } from '../../composables/useUi'

const { state, statusText } = useUi()
const transferring = computed(() => state.mainView === 'send' && state.centerState === 'transferring')
const dotClass = computed(() => (transferring.value || state.mainView === 'receive' ? 'll-dot ll-dot--success ll-dot--pulse' : 'll-dot ll-dot--success'))

/** 状态栏中段按主视图切换（原型三个视图文案不同） */
const midItems = computed(() => {
  if (state.mainView === 'receive') {
    return [
      { icon: 'settings', text: 'TCP 42210' },
      { icon: 'hardDrive', text: '剩余空间 218 GB' },
    ]
  }
  if (state.mainView === 'history') {
    return [
      { icon: 'settings', text: 'TCP 42210' },
      { icon: 'hardDrive', text: '本机存储 · 保留 30 天' },
    ]
  }
  return [
    { icon: 'settings', text: 'TCP 42210' },
    { icon: 'list', text: `会话黑名单 ${transferring.value ? 1 : 0} 条` },
    { icon: 'lock', text: `${transferring.value ? '通道加密' : '口令加密'}：${state.session?.encrypted ? '开' : '关'}` },
  ]
})
</script>

<template>
  <footer class="statusbar">
    <div class="sb-left">
      <span :class="dotClass" />
      <span class="sb-text">{{ statusText }}</span>
    </div>
    <div class="sb-center">
      <span v-for="m in midItems" :key="m.text">
        <Icon :name="m.icon" :size="11" /> {{ m.text }}
      </span>
    </div>
    <div class="sb-right">v0.9.4 (20261114) · 协议 v1</div>
  </footer>
</template>

<style scoped>
.statusbar {
  height: var(--ll-statusbar-h);
  flex: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--ll-space-4);
  font-size: var(--ll-fs-xs);
  color: var(--ll-color-text-tertiary);
  background: var(--ll-color-bg);
  border-top: 1px solid var(--ll-color-border);
}
.sb-left { display: flex; align-items: center; gap: var(--ll-space-2); }
.sb-text { color: var(--ll-color-text-secondary); }
.sb-center { display: flex; align-items: center; gap: var(--ll-space-5); }
.sb-center > span { display: inline-flex; align-items: center; gap: var(--ll-space-1); }
.sb-right { font-size: 10px; color: var(--ll-color-text-quaternary); }
</style>
