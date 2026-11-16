<script setup lang="ts">
import { computed } from 'vue'
import Icon from '../common/Icon.vue'
import BrandBlock from './BrandBlock.vue'
import { useUi } from '../../composables/useUi'
import { SEND_LOCKED as L } from '../../data/send'

import { useRecords } from '../../composables/useRecords'

const { state, openDialog, openSaveDir, pickFiles, chooseSaveDir, setDiscoverable, rescanLan, pauseAllTasks, showToast, busy } = useUi()
const { rows: recordRows, clearAll: clearRecords, exportCsv } = useRecords()

const view = computed(() => state.mainView)
const transferring = computed(() => state.centerState === 'transferring')
const withLan = computed(() => state.centerState === 'idle_with_lan')

/** send 视图第二按钮：定版就绪态为「打开保存目录」，传输中为「全部暂停」 */
const secondAction = computed(() => {
  if (transferring.value) return { icon: 'pause', label: '全部暂停' }
  return { icon: 'folder', label: L.openSaveDirLabel }
})

const onlineText = computed(() => {
  if (view.value === 'receive') return 'mDNS 广播中 · 端口 42210'
  if (view.value === 'history') return '本机在线 · 监听 42210'
  if (transferring.value) return '本机在线 · 监听 42210'
  if (withLan.value) return L.onlineText
  return '本机在线 · 监听 42210'
})

function onSecond() {
  if (transferring.value) { pauseAllTasks(); showToast('已暂停全部传输任务') }
  else openSaveDir()
}

/** 定版命令栏第三项：重新扫描设备（真实 UDP 发现 + RTT） */
function onRescan() {
  state.centerState = 'idle_with_lan'
  rescanLan()
}

function onSearchInput(e: Event) {
  state.historyQuery = (e.target as HTMLInputElement).value
  state.historyPage = 1
}

function onClear() {
  // F28：清空记录需二次确认，防止误删本机流水
  if (!window.confirm('确定清空本机全部传输记录？此操作不可撤销。')) return
  clearRecords()
  state.historyPage = 1
  showToast('已清空全部传输记录')
}
function onExportCsv() {
  exportCsv(recordRows.value)
  showToast(`已导出 ${recordRows.value.length} 条记录`)
}
</script>

<template>
  <div class="commandbar">
    <div class="cb-left">
      <!-- 品牌标识：标题栏只承载窗口语义，产品标识落在命令栏左侧 -->
      <BrandBlock class="cb-brand" />
      <span class="cb-sep cb-sep--tall" />

      <button class="ll-btn ll-btn--primary" @click="pickFiles">
        <Icon name="plus" :size="14" /> 选择文件
      </button>

      <!-- ============ 发送视图 ============ -->
      <template v-if="view === 'send'">
        <button class="ll-btn ll-btn--default" @click="onSecond">
          <Icon :name="secondAction.icon" :size="14" /> {{ secondAction.label }}
        </button>
        <template v-if="withLan">
          <span class="cb-sep" />
          <button class="ll-btn ll-btn--default" :disabled="busy.rescan" @click="onRescan">
            <span v-if="busy.rescan" class="ll-spinner ll-spinner--xs" />
            <Icon v-else name="refresh" :size="14" :class="{ 'spin': busy.rescan }" />
            {{ busy.rescan ? '扫描中…' : L.rescanLabel }}
          </button>
        </template>
        <label v-else-if="!transferring" class="cb-search">
          <Icon name="search" :size="14" />
          <input type="text" placeholder="搜索传输记录 / 设备" />
        </label>
        <div v-if="transferring" class="cb-connected">
          <Icon name="checkCircle" :size="14" />
          <span>已连接 <b>李工的 MacBook Pro</b> · IPv6 直连 · 38ms</span>
        </div>
      </template>

      <!-- ============ 接收视图 ============ -->
      <template v-else-if="view === 'receive'">
        <button class="ll-btn ll-btn--default" @click="openSaveDir">
          <Icon name="folder" :size="14" /> 打开保存目录
        </button>
        <button class="ll-btn ll-btn--default" @click="chooseSaveDir" title="更改保存目录">
          <Icon name="settings" :size="14" /> 更改
        </button>
        <span class="cb-sep" />
        <label class="cb-switch-row">
          <button
            class="ll-switch"
            :data-on="state.discoverable"
            role="switch"
            :aria-checked="state.discoverable"
            @click="setDiscoverable(!state.discoverable)"
          />
          <span>允许被同网段设备发现</span>
        </label>
      </template>

      <!-- ============ 传输记录视图 ============ -->
      <template v-else>
        <button class="ll-btn ll-btn--default" @click="onExportCsv">
          <Icon name="download" :size="14" /> 导出 CSV
        </button>
        <button class="ll-btn ll-btn--default" @click="onClear">
          <Icon name="trash" :size="14" /> 清空
        </button>
        <label class="cb-search cb-search--wide">
          <Icon name="search" :size="14" />
          <input
            type="text"
            placeholder="搜索文件名或设备"
            :value="state.historyQuery"
            @input="onSearchInput"
          />
        </label>
      </template>
    </div>

    <div class="cb-right">
      <span class="cb-online">
        <span class="ll-dot ll-dot--success" />
        {{ onlineText }}
      </span>
      <button class="cb-gear" title="设置" @click="openDialog('settings')">
        <Icon name="settings" :size="15" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.commandbar {
  height: var(--ll-commandbar-h);
  flex: none;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--ll-space-4);
  padding: 0 var(--ll-space-4);
  background: var(--ll-color-surface);
  border-bottom: 1px solid var(--ll-color-border);
}
.cb-left { display: flex; align-items: center; gap: var(--ll-space-2); min-width: 0; }
.cb-right { display: flex; align-items: center; gap: var(--ll-space-3); flex: none; }

.cb-sep { width: 1px; height: 18px; background: var(--ll-color-border); margin: 0 var(--ll-space-1); flex: none; }
/* 品牌块与操作区之间的分隔线，需覆盖两行文字的高度 */
.cb-sep--tall { height: 24px; margin: 0 var(--ll-space-2); }
.cb-brand { margin-right: var(--ll-space-2); }
.cb-switch-row {
  display: inline-flex;
  align-items: center;
  gap: var(--ll-space-2);
  font-size: var(--ll-fs-sm);
  color: var(--ll-color-text-secondary);
  white-space: nowrap;
  cursor: pointer;
}
.cb-switch-row .ll-switch { cursor: pointer; }

.cb-search {
  display: flex;
  align-items: center;
  gap: var(--ll-space-2);
  width: 220px;
  height: 30px;
  margin-left: var(--ll-space-3);
  padding: 0 var(--ll-space-3);
  color: var(--ll-color-text-tertiary);
  background: var(--ll-color-surface-2);
  border: 1px solid var(--ll-color-border);
  border-radius: var(--ll-radius-sm);
}
.cb-search--wide { width: 232px; }
.cb-search input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
  font-size: var(--ll-fs-sm);
  color: var(--ll-color-text);
}
.cb-search input::placeholder { color: var(--ll-color-text-quaternary); }

.cb-connected {
  display: inline-flex;
  align-items: center;
  gap: var(--ll-space-2);
  margin-left: var(--ll-space-3);
  padding: 5px var(--ll-space-3);
  font-size: var(--ll-fs-sm);
  color: var(--ll-color-success);
  background: var(--ll-color-success-light);
  border: 1px solid var(--ll-color-success);
  border-radius: var(--ll-radius-sm);
  white-space: nowrap;
}
.cb-connected b { font-weight: var(--ll-fw-semibold); color: var(--ll-color-text); }

.cb-online {
  display: inline-flex;
  align-items: center;
  gap: var(--ll-space-2);
  font-size: var(--ll-fs-xs);
  color: var(--ll-color-text-secondary);
  white-space: nowrap;
}
.cb-gear {
  width: 28px;
  height: 28px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: var(--ll-radius-sm);
  background: transparent;
  color: var(--ll-color-text-tertiary);
}
.cb-gear:hover { background: var(--ll-color-surface-hover); color: var(--ll-color-brand); }

/* 扫描中反馈：小号 spinner 与图标旋转 */
.ll-spinner--xs { width: 12px; height: 12px; border-width: 1.5px; }
.spin { animation: ll-spin .8s linear infinite; }
</style>
