<script setup lang="ts">
// 接收文件视图：对齐 docs/prototypes/main-receiving.png
// 无真实接收任务时只显示待机通道条，不渲染任何空壳任务卡。
import { computed, ref } from 'vue'
import Icon from '../../common/Icon.vue'
import EmptyState from '../../common/EmptyState.vue'
import { useUi } from '../../../composables/useUi'
import {
  RECEIVE_CHANNEL, RECEIVE_QUEUE, RECEIVE_DONE, RECEIVE_HINT,
} from '../../../data/views'

const { state, chooseSaveDir, showToast, openSaveDir, confirm } = useUi()
const connected = ref(true)

/** 从文件名取扩展名做图标角标（大写，最多 4 字符） */
function extOf(name: string): string {
  const ext = name.split('.').pop() || ''
  return ext.slice(0, 4).toUpperCase()
}

/** 正在接收：由后端进度事件驱动（state.tasks 中第一个 transferring 任务） */
const activeTask = computed(() => state.tasks.find((t) => t.status === 'transferring') || null)
const hasTask = computed(() => !!activeTask.value)

function onPause() {
  state.receivePaused = !state.receivePaused
  showToast(state.receivePaused ? '已暂停接收' : '已继续接收')
}

/** 危险操作：断开接收通道（二次确认） */
async function toggleConnected() {
  if (connected.value) {
    const ok = await confirm({
      title: '断开接收通道',
      message: '断开后本机将不再被同网段设备发现和连接，正在进行的接收会中断。',
      confirmText: '断开',
      tone: 'warning',
    })
    if (!ok) return
    connected.value = false
    showToast('已断开接收通道')
  } else {
    connected.value = true
    showToast('已恢复接收，等待对端连接')
  }
}

/** 危险操作：取消正在接收的文件（二次确认） */
async function cancelReceive() {
  if (!activeTask.value) return
  const ok = await confirm({
    title: '取消本次接收',
    message: '将中断当前文件的接收，已收到的数据不会保留，需对方重新发送。',
    confirmText: '取消接收',
  })
  if (!ok) return
  showToast('已取消本次接收')
}

/** 危险操作：从队列移除接收任务（二次确认） */
async function cancelQueued() {
  const ok = await confirm({
    title: '移除排队任务',
    message: '确定从接收队列中移除该任务？',
    confirmText: '移除',
  })
  if (!ok) return
  showToast('已从队列移除')
}
</script>

<template>
  <div class="view">
    <header class="center-head">
      <div class="ch-title">
        <h2>接收文件</h2>
        <p class="ch-sub ll-text-tertiary">{{ RECEIVE_CHANNEL.subtitle }}</p>
      </div>
      <div class="save-dir">
        <Icon name="folder" :size="14" />
        <span class="ll-mono save-path">{{ state.saveDir || RECEIVE_CHANNEL.saveDir }}</span>
        <button class="ll-btn ll-btn--default ll-btn--sm" @click="chooseSaveDir">更改</button>
      </div>
    </header>

    <div class="center-scroll">
      <!-- 接收通道条：有待机/接收中/断开三态；无速率时用中性的“待连接”，不显示刺眼的空绿条 -->
      <div class="channel-bar" :class="{
        offline: !connected,
        idle: connected && !hasTask,
      }">
        <span class="cb-ico"><Icon :name="connected ? 'download' : 'close'" :size="16" /></span>
        <div class="cb-text">
          <b>{{ !connected ? '已断开接收' : (hasTask ? RECEIVE_CHANNEL.title : '等待对端连接') }}</b>
          <span>{{ hasTask ? RECEIVE_CHANNEL.addr : '同网段设备可通过短码或序列号向本机发送' }}</span>
        </div>
        <span v-if="hasTask" class="cb-speed">{{ activeTask?.speedText || '—' }}</span>
        <button v-if="!hasTask && connected" class="ll-btn ll-btn--default ll-btn--sm" @click="toggleConnected">允许接收</button>
        <button v-else class="ll-btn ll-btn--default ll-btn--sm" @click="toggleConnected">{{ connected ? '断开' : '重新允许接收' }}</button>
      </div>

      <!-- 正在接收卡：仅真实任务存在时渲染 -->
      <div v-if="activeTask" class="ll-card active-card">
        <div class="ac-head">
          <span class="file-ico ico-video">{{ extOf(activeTask.name) }}</span>
          <div class="ll-col ac-meta">
            <span class="ac-name">{{ activeTask.name }}</span>
            <span class="ll-text-tertiary ac-detail">{{ activeTask.detail }}</span>
          </div>
          <button class="ac-btn" :title="state.receivePaused ? '继续接收' : '暂停接收'" @click="onPause">
            <Icon :name="state.receivePaused ? 'play' : 'pause'" :size="12" />
          </button>
          <button class="ac-btn" title="取消接收" @click="cancelReceive"><Icon name="close" :size="12" /></button>
        </div>
        <div class="ll-progress"><i :style="{ width: activeTask.progress + '%' }" /></div>
        <div class="ac-foot">
          <span>{{ activeTask.progress }}%</span>
          <span class="ll-text-tertiary">{{ activeTask.transferred }}</span>
        </div>
      </div>

      <!-- 等待接收列表：仅在有排队任务时渲染 -->
      <div v-if="RECEIVE_QUEUE.length" class="ll-card list-card">
        <div class="lc-head">
          <Icon name="clock" :size="12" />
          <span>队列中 {{ RECEIVE_QUEUE.length }} 个文件</span>
        </div>
        <div v-for="f in RECEIVE_QUEUE" :key="f.id" class="lc-row">
          <span class="file-ico" :class="`ico-${f.tone}`">{{ f.ext }}</span>
          <span class="lc-name ll-truncate">{{ f.name }}</span>
          <span class="lc-size">{{ f.size }}</span>
          <span class="lc-note ll-text-tertiary">{{ f.note }}</span>
          <button class="lc-x" title="取消" @click="cancelQueued"><Icon name="close" :size="11" /></button>
        </div>
      </div>

      <!-- 最近完成列表：仅在有已完成任务时渲染 -->
      <div v-if="RECEIVE_DONE.length" class="ll-card list-card">
        <div class="lc-head">
          <Icon name="checkCircle" :size="12" />
          <span>本次会话已完成 {{ RECEIVE_DONE.length }} 个</span>
        </div>
        <div v-for="f in RECEIVE_DONE" :key="f.id" class="lc-row">
          <span class="file-ico" :class="`ico-${f.tone}`">{{ f.ext }}</span>
          <span class="lc-name ll-truncate">{{ f.name }}</span>
          <span class="lc-size">{{ f.size }}</span>
          <span class="lc-done"><Icon name="check" :size="11" /> {{ f.note }}</span>
          <button class="lc-open" title="打开所在目录" @click="openSaveDir"><Icon name="folder" :size="13" /></button>
        </div>
      </div>

      <!-- 无任何接收活动时给一个干净的空态，替代原来的多个空壳卡片 -->
      <EmptyState
        v-if="!hasTask && !RECEIVE_QUEUE.length && !RECEIVE_DONE.length && connected"
        description="暂无接收任务"
        size="md"
      >
        <template #footer>
          <span class="empty-hint">对端连接成功后，文件会实时出现在这里</span>
        </template>
      </EmptyState>

      <!-- 底部提示条 -->
      <div class="hint-bar">
        <Icon name="info" :size="13" />
        <span>{{ RECEIVE_HINT }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.view { flex: 1; display: flex; flex-direction: column; min-width: 0; min-height: 0; }
.center-head {
  display: flex; align-items: flex-start; justify-content: space-between;
  gap: var(--ll-space-3); padding: var(--ll-space-4) var(--ll-space-5) var(--ll-space-3);
}
.ch-title h2 { font-size: var(--ll-fs-xl); font-weight: var(--ll-fw-semibold); }
.ch-sub { font-size: var(--ll-fs-sm); margin-top: 2px; }
.save-dir {
  display: inline-flex; align-items: center; gap: var(--ll-space-2);
  color: var(--ll-color-text-tertiary); flex: none;
}
.save-path { font-size: var(--ll-fs-xs); color: var(--ll-color-text-secondary); }
.center-scroll {
  flex: 1; overflow-y: auto;
  padding: 0 var(--ll-space-5) var(--ll-space-5);
  display: flex; flex-direction: column; gap: var(--ll-space-2);
}
.center-scroll > * { flex: none; }

/* 绿色接收通道条 */
.channel-bar {
  display: flex; align-items: center; gap: var(--ll-space-3);
  padding: var(--ll-space-3);
  border: 1px solid var(--ll-color-success-border);
  border-radius: var(--ll-radius-md);
  background: var(--ll-color-success-light);
}
.channel-bar.offline,
.channel-bar.idle {
  border-color: var(--ll-color-border);
  background: var(--ll-color-surface-3);
}
.cb-ico { color: var(--ll-color-success); display: inline-flex; flex: none; }
.channel-bar.offline .cb-ico,
.channel-bar.idle .cb-ico { color: var(--ll-color-text-tertiary); }
.cb-text { display: flex; flex-direction: column; gap: 2px; flex: 1; min-width: 0; }
.cb-text b { font-size: var(--ll-fs-md); color: var(--ll-color-success); font-weight: var(--ll-fw-semibold); }
.channel-bar.offline .cb-text b,
.channel-bar.idle .cb-text b { color: var(--ll-color-text-secondary); }
.empty-hint { font-size: var(--ll-fs-xs); color: var(--ll-color-text-quaternary); }
.cb-text > span { font-size: var(--ll-fs-xs); color: var(--ll-color-text-secondary); }
.cb-speed {
  font-family: var(--ll-font-mono); font-weight: var(--ll-fw-semibold);
  font-size: var(--ll-fs-md); color: var(--ll-color-success); white-space: nowrap;
}

/* 正在接收卡 */
.active-card { padding: var(--ll-space-3); border-color: var(--ll-color-brand); }
.ac-head { display: flex; align-items: center; gap: var(--ll-space-2); }
.file-ico {
  width: 20px; height: 20px; flex: none;
  display: inline-flex; align-items: center; justify-content: center;
  border-radius: var(--ll-radius-xs); color: #fff;
  font-size: 8px; font-weight: var(--ll-fw-bold); letter-spacing: -0.3px;
}
.ico-pdf { background: var(--ll-color-danger); }
.ico-video { background: var(--ll-color-brand); color: var(--ll-color-text-inverse); }
.ico-zip { background: var(--ll-color-warning); }
.ico-img { background: var(--ll-color-info); }
.ac-meta { flex: 1; min-width: 0; gap: 2px; }
.ac-name { font-size: var(--ll-fs-md); font-weight: var(--ll-fw-medium); }
.ac-detail { font-size: var(--ll-fs-xs); }
.ac-btn {
  width: 22px; height: 22px; flex: none; border: 1px solid var(--ll-color-border);
  background: var(--ll-color-surface); border-radius: var(--ll-radius-xs);
  color: var(--ll-color-text-tertiary);
  display: inline-flex; align-items: center; justify-content: center;
}
.ac-btn:hover { color: var(--ll-color-brand); border-color: var(--ll-color-brand); }
.active-card .ll-progress { margin-top: var(--ll-space-3); }
.active-card .ll-progress > i { background: var(--ll-color-brand); }
.ac-foot {
  display: flex; justify-content: space-between; gap: var(--ll-space-3);
  margin-top: var(--ll-space-2); font-size: var(--ll-fs-xs);
  color: var(--ll-color-text-secondary);
}

/* 队列 / 已完成列表：与全局表格一致，行间无线、表头浅灰底、行高更高 */
.list-card { overflow: hidden; }
.lc-head {
  display: flex; align-items: center; gap: var(--ll-space-2);
  padding: 9px var(--ll-space-3);
  background: var(--ll-color-surface-2);
  border: none;
  font-size: var(--ll-fs-xs); color: var(--ll-color-text-secondary);
  font-weight: var(--ll-fw-medium);
}
.lc-row {
  display: flex; align-items: center; gap: var(--ll-space-2);
  padding: 10px var(--ll-space-3);
  border: none;
  font-size: var(--ll-fs-sm);
  transition: background-color .15s ease;
}
/* div 模拟的列表行与全局 .ll-table 数据行保持同一 hover 反馈 */
.lc-row:hover { background: var(--ll-color-row-hover); }
.lc-name { flex: 1; min-width: 0; color: var(--ll-color-text); }
.lc-size { width: 68px; flex: none; text-align: right; color: var(--ll-color-text-secondary); }
.lc-note { width: 96px; flex: none; text-align: right; font-size: var(--ll-fs-xs); }
.lc-done {
  width: 96px; flex: none; display: inline-flex; align-items: center;
  justify-content: flex-end; gap: 3px;
  color: var(--ll-color-success); font-size: var(--ll-fs-xs);
}
.lc-x, .lc-open {
  width: 20px; height: 20px; flex: none; border: none; background: transparent;
  border-radius: var(--ll-radius-xs); display: inline-flex;
  align-items: center; justify-content: center;
  color: var(--ll-color-text-tertiary);
}
.lc-x:hover { background: var(--ll-color-surface-3); color: var(--ll-color-danger); }
.lc-open { color: var(--ll-color-brand); }
.lc-open:hover { background: var(--ll-color-brand-light); }

/* 底部提示条 */
.hint-bar {
  display: flex; align-items: center; gap: var(--ll-space-2);
  padding: var(--ll-space-3);
  border: 1px solid var(--ll-color-border);
  border-radius: var(--ll-radius-md);
  background: var(--ll-color-surface-2);
  color: var(--ll-color-text-tertiary); font-size: var(--ll-fs-xs);
}
</style>
