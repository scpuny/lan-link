<script setup lang="ts">
import { onMounted } from 'vue'
import TitleBar from './components/layout/TitleBar.vue'
import CommandBar from './components/layout/CommandBar.vue'
import LeftRail from './components/layout/LeftRail.vue'
import CenterStage from './components/layout/CenterStage.vue'
import RightPanel from './components/layout/RightPanel.vue'
import StatusBar from './components/layout/StatusBar.vue'
import Icon from './components/common/Icon.vue'
import ConnectDialog from './components/dialogs/ConnectDialog.vue'
import GenerateDialog from './components/dialogs/GenerateDialog.vue'
import GenerateLanDialog from './components/dialogs/GenerateLanDialog.vue'
import WaitConnectDialog from './components/dialogs/WaitConnectDialog.vue'
import SettingsDialog from './components/dialogs/SettingsDialog.vue'
import { useUi } from './composables/useUi'

const { state, toast, init } = useUi()
const isNative = state.bridgeMode === 'tauri'

onMounted(() => {
  // Tauri 真机由系统绘制窗口圆角/阴影，去掉预览用的桌面画布
  if (isNative) document.getElementById('app')?.classList.add('is-native')
  init()
})
</script>

<template>
  <!-- 桌面窗口外壳：与 calicat 原型一致（画布 → 圆角白窗口） -->
  <div class="ll-window" :class="{ 'is-native': isNative }">
    <TitleBar />
    <CommandBar />

    <div class="app-body">
      <LeftRail />
      <CenterStage />
      <RightPanel />
    </div>

    <StatusBar />
  </div>

  <!-- 一次性动作反馈（如「打开保存目录」），置于窗口容器外避免被圆角裁切 -->
  <Transition name="toast">
    <div v-if="toast" class="ll-toast" :class="{ 'is-native': isNative }" role="status">
      <Icon name="folder" :size="14" />
      <span>{{ toast }}</span>
    </div>
  </Transition>

  <!-- 对话框用 fixed 定位，置于窗口容器之外避免被圆角裁切 -->
  <ConnectDialog v-if="state.dialogs.connect" @close="state.dialogs.connect = false" />
  <GenerateDialog v-if="state.dialogs.generate" @close="state.dialogs.generate = false" />
  <GenerateLanDialog v-if="state.dialogs.generateLan" @close="state.dialogs.generateLan = false" />
  <WaitConnectDialog v-if="state.dialogs.waitConnect" @close="state.dialogs.waitConnect = false" />
  <SettingsDialog v-if="state.dialogs.settings" @close="state.dialogs.settings = false" />
</template>

<style scoped>
.app-body {
  flex: 1;
  display: flex;
  min-height: 0;
}

/* 标题栏中央（tb-center 在空闲态为空白），不遮挡三栏内容 */
.ll-toast {
  position: fixed;
  top: calc(var(--ll-window-inset) + 5px);
  left: 50%;
  transform: translateX(-50%);
  z-index: 120;
  display: inline-flex;
  align-items: center;
  gap: var(--ll-space-2);
  max-width: min(520px, 76vw);
  padding: 8px var(--ll-space-4);
  border: 1px solid var(--ll-color-border);
  border-radius: var(--ll-radius-md);
  background: var(--ll-color-bg-elevated);
  color: var(--ll-color-text);
  box-shadow: var(--ll-shadow-modal, 0 6px 20px rgb(0 0 0 / 12%));
  font-size: var(--ll-fs-sm);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.ll-toast.is-native { top: 5px; }
.ll-toast :deep(svg) { flex: none; color: var(--ll-color-brand); }
.toast-enter-active, .toast-leave-active { transition: opacity .18s ease, transform .18s ease; }
.toast-enter-from, .toast-leave-to { opacity: 0; transform: translate(-50%, -6px); }
</style>
