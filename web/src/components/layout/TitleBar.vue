<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import AppMenu, { type MenuGroup } from './AppMenu.vue'
import Icon from '../common/Icon.vue'
import { useUi } from '../../composables/useUi'
import { useTheme, type ThemePref } from '../../composables/useTheme'
import { IS_MAC } from '../../composables/usePlatform'
import { PALETTES, VISUAL_STYLES } from '../../data/themes'

const { state, openDialog, pickFiles, pickFolder, pasteSend } = useUi()
const { setTheme, setPalette } = useTheme()

const menus = computed<MenuGroup[]>(() => [
  {
    label: '文件',
    items: [
      { label: '选择文件…', key: 'open-file', hint: 'Ctrl+O' },
      { label: '选择文件夹…', key: 'open-folder' },
      { label: '从剪贴板发送', key: 'paste-send', hint: 'Ctrl+V' },
      { sep: true, label: '' },
      { label: '传输记录', key: 'view-history', hint: '18' },
      { label: '打开下载目录', key: 'open-dl' },
      { sep: true, label: '' },
      { label: '退出', key: 'quit', hint: 'Alt+F4' },
    ],
  },
  {
    label: '设备',
    items: [
      { label: '扫描局域网设备', key: 'scan', hint: 'F5' },
      { label: '附近设备', key: 'nearby', hint: '3 台' },
      { sep: true, label: '' },
      { label: '生成外网序列号…', key: 'gen', hint: 'Ctrl+G' },
      { label: '生成局域网短码…', key: 'gen-lan', hint: 'Ctrl+L' },
      { label: '连接到远程设备…', key: 'connect', hint: 'Ctrl+K' },
      { sep: true, label: '' },
      { label: '本机网络信息', key: 'netinfo' },
      { label: '端口与防火墙检查', key: 'portcheck' },
    ],
  },
  {
    label: '视图',
    items: [
      { label: '发送文件', key: 'view-send', checked: state.mainView === 'send' },
      { label: '接收文件', key: 'view-recv', checked: state.mainView === 'receive' },
      { label: '传输记录', key: 'view-history-2', checked: state.mainView === 'history' },
      { sep: true, label: '' },
      { label: '显示侧栏', key: 'toggle-rail', checked: true },
      { label: '显示速率曲线', key: 'toggle-rate', checked: true },
      { sep: true, label: '' },
      { label: '外观', key: 'theme', submenu: ['跟随系统', '浅色', '深色'] },
      { label: '视觉风格', key: 'palette', submenu: [...VISUAL_STYLES.map((s) => s.short), '原型基准'] },
    ],
  },
  {
    label: '帮助',
    items: [
      { label: '使用指南', key: 'help', hint: 'F1' },
      { label: '快捷键一览', key: 'shortcuts' },
      { label: '错误码说明 E001–E009', key: 'errcodes' },
      { label: '协议与安全说明', key: 'protocol' },
      { sep: true, label: '' },
      { label: '检查更新', key: 'update', hint: 'v0.9.4' },
      { label: '关于 LanLink', key: 'about' },
    ],
  },
])

const themeMap: Record<string, ThemePref> = { 浅色: 'light', 深色: 'dark', 跟随系统: 'system' }

function onAction(p: { key: string; value?: string }) {
  switch (p.key) {
    case 'open-file': pickFiles(); break
    case 'open-folder': pickFolder(); break
    case 'paste-send': pasteSend(); break
    case 'gen': openDialog('generate'); break
    case 'gen-lan': openDialog('generateLan'); break
    case 'connect': openDialog('connect'); break
    case 'settings':
    case 'about': openDialog('settings'); break
    case 'view-send': state.mainView = 'send'; break
    case 'view-recv': state.mainView = 'receive'; break
    case 'view-history':
    case 'view-history-2': state.mainView = 'history'; break
    case 'scan': state.centerState = 'idle_with_lan'; break
    case 'open-dl': useUi().openSaveDir(); break
    // 窗口内菜单传中文短名，macOS 原生菜单传 machine id，两者都接受
    case 'theme': {
      const pref = p.value ? themeMap[p.value] ?? (p.value as ThemePref) : undefined
      if (pref) setTheme(pref)
      break
    }
    case 'palette': {
      if (!p.value) break
      const hit = PALETTES.find((x) => x.short === p.value || x.id === p.value)
      if (hit) setPalette(hit.id)
      break
    }
  }
}

// ---------------------------------------------------------------------
// macOS 原生菜单栏
// ---------------------------------------------------------------------
// 菜单由 Rust 侧构建成系统 NSMenu（屏幕顶部），点击后通过 `ll-menu` 事件回传
// `{ key, value }`，在这里并入同一个 onAction 分发，业务逻辑与窗口内菜单共用。
// 浏览器预览态没有 Tauri 事件通道，listen 会失败，静默忽略即可。
let unlistenMenu: UnlistenFn | undefined

onMounted(async () => {
  if (!IS_MAC) return
  try {
    unlistenMenu = await listen<{ key: string; value?: string | null }>('ll-menu', (e) => {
      onAction({ key: e.payload.key, value: e.payload.value ?? undefined })
    })
  } catch (e) {
    console.warn('[titlebar] native menu listener unavailable', e)
  }
})

onBeforeUnmount(() => {
  unlistenMenu?.()
  unlistenMenu = undefined
})

// 「传输中」标记仅在发送视图的传输态出现；接收/记录视图由状态栏表达
const transferring = computed(() => state.mainView === 'send' && state.centerState === 'transferring')

// ---------------------------------------------------------------------
// 自定义窗体：窗口控制
// macOS 保留系统交通灯（不渲染右侧按钮）；Windows / Linux 自绘三键。
// 浏览器预览态无 Tauri 窗口，点击静默忽略。
// ---------------------------------------------------------------------
const isNative = state.bridgeMode === 'tauri'

async function onMinimize() {
  if (!isNative) return
  try {
    await getCurrentWindow().minimize()
  } catch (e) {
    console.warn('[titlebar] minimize failed', e)
  }
}

async function onToggleMax() {
  if (!isNative) return
  try {
    await getCurrentWindow().toggleMaximize()
  } catch (e) {
    console.warn('[titlebar] toggleMaximize failed', e)
  }
}

async function onClose() {
  if (!isNative) return
  try {
    await getCurrentWindow().close()
  } catch (e) {
    console.warn('[titlebar] close failed', e)
  }
}
</script>

<template>
  <header
    class="titlebar"
    :class="{ 'is-macos': IS_MAC, 'is-native': isNative }"
    data-tauri-drag-region
  >
    <div class="tb-left" data-tauri-drag-region>
      <!-- 品牌标识已移到命令栏左侧；标题栏只保留窗口语义 -->
      <!-- Windows / Linux：没有全局菜单栏，菜单紧贴左侧 -->
      <div v-if="!IS_MAC" class="tb-menu"><AppMenu :menus="menus" @action="onAction" /></div>
    </div>

    <!-- 居中区绝对定位，保证名称相对整个窗口居中，不受左右内容宽度影响 -->
    <div class="tb-center">
      <!-- macOS：系统标题被隐藏（hiddenTitle），这里补一个窗口标题替身 -->
      <span v-if="IS_MAC" class="tb-title">LanLink</span>
      <span v-if="transferring" class="tb-channel">
        <span class="ll-dot ll-dot--success ll-dot--pulse" /> 传输中
      </span>
    </div>

    <div class="tb-right">
      <!-- Windows / Linux：无系统装饰，自绘窗口三键；macOS 用系统交通灯 -->
      <div v-if="!IS_MAC" class="tb-win">
        <button class="tb-winbtn" title="最小化" @click="onMinimize"><Icon name="minus" :size="14" /></button>
        <button class="tb-winbtn" title="最大化" @click="onToggleMax"><Icon name="external" :size="12" /></button>
        <button class="tb-winbtn tb-close" title="关闭" @click="onClose"><Icon name="close" :size="14" /></button>
      </div>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  position: relative;
  height: var(--ll-titlebar-h);
  flex: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--ll-space-3);
  background: var(--ll-color-titlebar);
  border-bottom: 1px solid var(--ll-color-border);
  /* 拖拽由 Tauri 的 data-tauri-drag-region 负责，不再用 Electron 的 -webkit-app-region */
  user-select: none;
}
/* macOS：系统交通灯浮在内容之上（titleBarStyle=Overlay），左侧留出 78px 让位 */
.titlebar.is-macos { padding-left: 78px; }
.tb-left { display: flex; align-items: center; gap: var(--ll-space-2); min-width: 0; }
/* 非 macOS：菜单紧贴标题栏左侧 */
.tb-menu { display: flex; align-self: stretch; }
/* 居中区：绝对定位铺满标题栏，内容相对窗口居中 */
.tb-center {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--ll-space-2);
  pointer-events: none;
}
.tb-title {
  font-size: var(--ll-fs-md);
  font-weight: var(--ll-fw-medium);
  color: var(--ll-color-text-secondary);
}
.tb-channel {
  display: inline-flex;
  align-items: center;
  gap: var(--ll-space-2);
  font-size: var(--ll-fs-sm);
  color: var(--ll-color-success);
  font-weight: var(--ll-fw-medium);
}
.tb-right { display: flex; align-items: center; gap: var(--ll-space-2); }
.tb-win { display: flex; align-items: center; gap: 2px; margin-right: calc(-1 * var(--ll-space-1)); }
.tb-winbtn {
  width: 28px;
  height: 24px;
  border: none;
  background: transparent;
  color: var(--ll-color-text-tertiary);
  border-radius: var(--ll-radius-xs);
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.tb-winbtn:hover { background: var(--ll-color-surface-hover); color: var(--ll-color-text); }
.tb-close:hover { background: var(--ll-color-danger); color: #fff; }
</style>
