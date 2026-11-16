<script setup lang="ts">
// =====================================================================
// 中央工作区：按左轨导航（mainView）分发到三个独立视图组件。
//   send    → 三态发送视图（就绪 / LAN 已发现 / 传输中）
//   receive → 接收文件（接收中）
//   history → 传输记录
// =====================================================================
import { computed } from 'vue'
import SendView from './views/SendView.vue'
import ReceiveView from './views/ReceiveView.vue'
import HistoryView from './views/HistoryView.vue'
import { useUi } from '../../composables/useUi'

const { state } = useUi()
const current = computed(() =>
  state.mainView === 'receive' ? ReceiveView
    : state.mainView === 'history' ? HistoryView
      : SendView
)
</script>

<template>
  <main class="center">
    <component :is="current" :key="state.mainView" />
  </main>
</template>

<style scoped>
.center {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  background: var(--ll-color-bg);
}
</style>
