<script setup lang="ts">
// 发送文件视图三态：
//   idle_with_lan —— 定版（main-ready-locked.png），应用默认态
//   idle_no_lan   —— 旧就绪态（main-ready.png），保留为无设备时的预览变体
//   transferring  —— 传输中（main-transferring.png）
import { computed, ref } from 'vue'
import Icon from '../../common/Icon.vue'
import EmptyState from '../../common/EmptyState.vue'
import { useUi } from '../../../composables/useUi'
import { SEND_LOCKED as L } from '../../../data/send'
import type { SendFile } from '../../../types/invite'

const {
  state, openDialog, pickFiles, pickFolder, pasteSend, addPathsToSend,
  removeSendFile, clearSendFiles, sendSummary, showToast,
  pauseTask, resumeTask, cancelTask,
} = useUi()

const withLan = computed(() => state.centerState === 'idle_with_lan')
const transferring = computed(() => state.centerState === 'transferring')
const dragOver = ref(false)

/** 字节 → 人类可读 */
function fmtSize(n: number): string {
  if (!n) return '—'
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`
}
/** 毫秒时间戳 → MM-DD HH:mm */
function fmtTime(ms: number): string {
  if (!ms) return '—'
  const d = new Date(ms)
  const p = (v: number) => String(v).padStart(2, '0')
  return `${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}
/** 扩展名 → 图标 tone */
function toneOf(ext: string): string {
  const e = ext.toUpperCase()
  if (e === 'PDF') return 'pdf'
  if (['MP4', 'MOV', 'AVI', 'MKV', 'WEBM', 'WMV'].includes(e)) return 'video'
  if (['ZIP', 'RAR', '7Z', 'TAR', 'GZ'].includes(e)) return 'zip'
  return 'file'
}
function extOf(f: SendFile): string {
  return f.ext || (f.name.includes('.') ? f.name.split('.').pop()!.toUpperCase() : 'TXT')
}

/**
 * 展示用的文件表数据：
 *  - 用户已选文件（sendFiles 非空）→ 真实队列；
 *  - 否则回退到定版原型数据，保证默认打开界面逐像素不变。
 */
interface DisplayFile { key: string; name: string; size: string; time: string; ext: string; tone: string; pathOrName: string }
const displayFiles = computed<DisplayFile[]>(() => {
  if (state.sendFiles.length) {
    return state.sendFiles.map((f) => ({
      key: f.path || `text:${f.name}`,
      name: f.name,
      size: fmtSize(f.size),
      time: fmtTime(f.modified),
      ext: extOf(f),
      tone: toneOf(f.ext),
      pathOrName: f.path || f.name,
    }))
  }
  return L.files.map((f, i) => ({ key: `proto-${i}`, ...f, pathOrName: `proto-${i}` }))
})

/** 汇总行：真实队列优先，否则用原型汇总文案 */
const fileFootText = computed(() =>
  state.sendFiles.length
    ? `已选 ${sendSummary.value.count} 个文件，共 ${fmtSize(sendSummary.value.bytes)}`
    : L.fileFoot,
)

/** 就绪态文件表（原型 main-ready.png，无 LAN 变体） */
const idleFiles = [
  { name: 'LanLink-设计规范-v1.2.pdf', size: '4.2 MB', time: '11-14 10:22', ext: 'PDF', tone: 'pdf' },
  { name: '会议录屏_1114_外网序列号评审.mp4', size: '862 MB', time: '11-14 09:58', ext: 'MP4', tone: 'video' },
  { name: 'lanlink-protocol-src.zip', size: '128 MB', time: '11-13 21:40', ext: 'ZIP', tone: 'zip' },
]

/**
 * 设备表展示数据：真机一旦发现设备用真实 nearby；否则回退定版原型数据，
 * 保证默认打开界面与原型逐像素一致。
 */
interface DisplayDevice {
  id: string; name: string; platform: string; addr: string
  latency: string; action: string; primary: boolean; warn: boolean; icon: string
}
const displayDevices = computed<DisplayDevice[]>(() => {
  if (state.nearby.length) {
    return state.nearby.map((d, i) => ({
      id: d.id,
      name: d.name,
      platform: d.platform || '局域网设备',
      addr: d.addr,
      latency: d.latency ? `${d.latency} ms` : '—',
      action: i === 0 ? '连接' : '发送',
      primary: i === 0,
      warn: false,
      icon: 'laptop',
    }))
  }
  return L.target.devices.map((d) => ({ ...d, warn: !!d.warn }))
})
const deviceHint = computed(() =>
  state.nearby.length
    ? `同网段 · mDNS 自动发现 ${state.nearby.length} 台`
    : L.target.hint,
)

/** F1 拖拽落区：从 DataTransfer 取本地路径；Tauri webview 的 File 对象带 path 属性 */
function onDrop(e: DragEvent) {
  dragOver.value = false
  const dt = e.dataTransfer
  if (!dt) return
  const paths: string[] = []
  for (const f of Array.from(dt.files)) {
    const p = (f as File & { path?: string }).path
    if (p) paths.push(p)
  }
  if (paths.length) addPathsToSend(paths)
  else showToast('未识别到本地文件路径，请使用「选择文件」按钮')
}
const activeTasks = computed(() => state.tasks.filter((t) => t.status === 'transferring'))
const queuedTasks = computed(() => state.tasks.filter((t) => t.status === 'queued'))
const doneCount = computed(() => state.tasks.filter((t) => t.status === 'done').length)

/** 近 60s 速率曲线：真实采样优先，否则用原型静态序列 */
const PROTO_RATE = [17, 18, 19, 18, 21, 22, 21, 24, 26, 25, 28, 27, 30, 32, 31, 34, 33, 36, 38, 37, 40, 39, 41, 42, 40, 43, 44, 42, 45, 46]
const rateSeries = computed(() => (state.rateHistory.length >= 3 ? state.rateHistory.slice(-30) : PROTO_RATE))
const rateLine = computed(() => {
  const s = rateSeries.value
  return s
    .map((v, i) => {
      const x = s.length > 1 ? (i / (s.length - 1)) * 600 : 0
      const y = 86 - (v / 50) * 72
      return `${x.toFixed(1)},${y.toFixed(1)}`
    })
    .join(' ')
})
const rateArea = computed(() => `M0,90 L${rateLine.value.split(' ').join(' L')} L600,90 Z`)
</script>

<template>
  <div class="view">
    <header class="center-head">
      <div class="ch-title">
        <h2>{{ transferring ? '传输任务' : '发送文件' }}</h2>
        <p class="ch-sub ll-text-tertiary">
          <template v-if="transferring">与「李工的 MacBook Pro」的直连通道已建立，分片按 seq 单调校验</template>
          <template v-else-if="withLan">{{ L.subtitle }}</template>
          <template v-else>选择文件后，通过外网序列号或局域网短码邀请对方建立直连通道</template>
        </p>
      </div>
      <div class="ch-actions" v-if="transferring">
        <span class="ll-tag ll-tag--brand">进行中 {{ activeTasks.length }}</span>
        <span class="ll-tag ll-tag--plain">排队 {{ queuedTasks.length }}</span>
        <span class="ll-tag ll-tag--success">已完成 {{ doneCount }}</span>
      </div>
      <div class="ch-actions" v-else>
        <button v-if="withLan" class="ll-btn ll-btn--default ll-btn--sm" @click="clearSendFiles">{{ L.clearLabel }}</button>
        <button v-else class="ll-btn ll-btn--default ll-btn--sm" @click="clearSendFiles">清空</button>
      </div>
    </header>

    <div class="center-scroll">
      <!-- ---------- 就绪态（无 LAN 设备） ---------- -->
      <template v-if="state.centerState === 'idle_no_lan'">
        <div
          class="dropzone"
          :class="{ 'dropzone--over': dragOver }"
          @dragover.prevent="dragOver = true"
          @dragleave.prevent="dragOver = false"
          @drop.prevent="onDrop"
          @click="pickFiles"
        >
          <span class="dz-icon"><Icon name="cloudUpload" :size="22" /></span>
          <span class="dz-title">将文件或文件夹拖放到此处</span>
          <span class="dz-sub ll-text-tertiary">或点击左上角「选择文件」· 支持断点续传与 100 GB 以上大文件</span>
        </div>

        <div class="ll-card file-card">
          <table class="ll-table file-table">
            <thead>
              <tr><th>文件名</th><th>大小</th><th>修改时间</th><th>状态</th><th></th></tr>
            </thead>
            <tbody>
              <tr v-for="f in displayFiles" :key="f.key">
                <td>
                  <div class="file-name">
                    <span class="file-ico" :class="`ico-${f.tone}`">{{ f.ext }}</span>
                    <span class="ll-truncate">{{ f.name }}</span>
                  </div>
                </td>
                <td>{{ f.size }}</td>
                <td>{{ f.time }}</td>
                <td><span class="ll-tag ll-tag--plain">待发送</span></td>
                <td><button class="row-x" title="移除" @click="removeSendFile(f.pathOrName)"><Icon name="close" :size="12" /></button></td>
              </tr>
              <tr v-if="!displayFiles.length">
                <td colspan="5" class="empty-row"><EmptyState description="暂无数据" /></td>
              </tr>
            </tbody>
          </table>
          <div class="file-foot">
            <span class="ll-text-tertiary">{{ fileFootText }}</span>
            <span class="ll-text-tertiary">剩余空间 218 GB</span>
          </div>
        </div>

        <div class="method-head">
          <span class="ll-section-title">选择连接方式</span>
          <span class="ll-text-tertiary method-hint">两种凭证互不通用，请勿混输</span>
        </div>
        <div class="method-cards">
          <div class="ll-card method-card method-card--brand">
            <div class="mc-head">
              <span class="mc-icon brand"><Icon name="globe" :size="15" /></span>
              <span class="mc-title">外网序列号</span>
              <span class="ll-tag ll-tag--brand">跨网直连</span>
            </div>
            <p class="mc-desc ll-text-secondary">自包含候选地址，经 IM 纯文本搬运，有效期 15 分钟</p>
            <button class="ll-btn ll-btn--primary ll-btn--block" @click="openDialog('generate')">
              <Icon name="link" :size="13" /> 生成外网序列号
            </button>
          </div>
          <div class="ll-card method-card">
            <div class="mc-head">
              <span class="mc-icon"><Icon name="lan" :size="15" /></span>
              <span class="mc-title">局域网短码</span>
              <span class="ll-tag ll-tag--plain">同网段</span>
            </div>
            <p class="mc-desc ll-text-secondary">6 位短码 + mDNS 发现，断网可用，程序运行期内有效</p>
            <button class="ll-btn ll-btn--default ll-btn--block" @click="openDialog('generateLan')">
              <Icon name="lan" :size="13" /> 生成局域网短码
            </button>
          </div>
        </div>

        <div class="idle-task-bar">
          <span class="itb-icon"><Icon name="clock" :size="15" /></span>
          <div class="ll-col">
            <span class="itb-title">当前无进行中的传输任务</span>
            <span class="ll-text-tertiary">对端通过序列号完成握手后，传输任务会在此实时显示速度、速率与剩余时间</span>
          </div>
          <span class="ll-link" @click="state.mainView = 'history'">查看传输记录 <Icon name="chevronRight" :size="11" /></span>
        </div>
      </template>

      <!-- ---------- 定版：就绪态（锁定版），应用默认 ---------- -->
      <template v-else-if="withLan">
        <div
          class="dropzone"
          :class="{ 'dropzone--over': dragOver }"
          @dragover.prevent="dragOver = true"
          @dragleave.prevent="dragOver = false"
          @drop.prevent="onDrop"
          @click="pickFiles"
        >
          <span class="dz-icon"><Icon name="cloudUpload" :size="22" /></span>
          <span class="dz-title">{{ L.dropTitle }}</span>
          <span class="dz-sub ll-text-tertiary">{{ L.dropSub }}</span>
        </div>

        <div class="ll-card file-card">
          <table class="ll-table file-table">
            <thead>
              <tr><th>文件名</th><th>大小</th><th>修改时间</th><th>状态</th><th></th></tr>
            </thead>
            <tbody>
              <tr v-for="f in displayFiles" :key="f.key">
                <td>
                  <div class="file-name">
                    <span class="file-ico" :class="`ico-${f.tone}`">{{ f.ext }}</span>
                    <span class="ll-truncate">{{ f.name }}</span>
                  </div>
                </td>
                <td>{{ f.size }}</td>
                <td>{{ f.time }}</td>
                <td><span class="ll-tag ll-tag--plain">待发送</span></td>
                <td><button class="row-x" title="移除" @click="removeSendFile(f.pathOrName)"><Icon name="close" :size="12" /></button></td>
              </tr>
              <tr v-if="!displayFiles.length">
                <td colspan="5" class="empty-row"><EmptyState description="暂无数据" /></td>
              </tr>
            </tbody>
          </table>
          <div class="file-foot locked-foot">
            <span class="ll-text-tertiary">{{ fileFootText }}</span>
          </div>
        </div>

        <!-- 选择目标设备：定版核心区块 -->
        <div class="ll-card target-card">
          <div class="tc-head">
            <span class="tc-title"><Icon name="compass" :size="14" /> {{ L.target.title }}</span>
            <span class="ll-text-tertiary">{{ L.target.scan }}</span>
          </div>
          <div class="tc-hint"><Icon name="wifi" :size="13" /> {{ deviceHint }}</div>
          <table class="ll-table device-table">
            <thead>
              <tr><th>设备</th><th>地址</th><th>延迟</th><th>操作</th></tr>
            </thead>
            <tbody>
              <tr v-for="d in displayDevices" :key="d.id">
                <td>
                  <div class="device-name">
                    <span class="device-ico"><Icon :name="d.icon" :size="15" /></span>
                    <div class="ll-col">
                      <span class="ll-truncate">{{ d.name }}</span>
                      <span class="ll-text-tertiary device-platform" :class="{ warn: d.warn }">{{ d.platform }}</span>
                    </div>
                  </div>
                </td>
                <td class="ll-mono">{{ d.addr }}</td>
                <td class="device-lat" :class="{ warn: d.warn }">{{ d.latency }}</td>
                <td>
                  <button
                    class="ll-btn ll-btn--sm"
                    :class="d.primary ? 'll-btn--primary' : 'll-btn--default'"
                    @click="openDialog(d.primary ? 'connect' : 'generate')"
                  >{{ d.action }}</button>
                </td>
              </tr>
              <tr v-if="!displayDevices.length">
                <td colspan="4" class="empty-row"><EmptyState description="暂无数据" /></td>
              </tr>
            </tbody>
          </table>
          <div class="tc-cross">
            <div class="tc-cross-title"><Icon name="globe" :size="13" /> {{ L.target.crossTitle }}</div>
            <div class="tc-cross-body">
              <span class="ll-text-secondary">{{ L.target.crossDesc }}</span>
              <button class="ll-btn ll-btn--primary ll-btn--sm" @click="openDialog('generate')">
                <Icon name="link" :size="12" /> {{ L.target.generateLabel }}
              </button>
              <button class="ll-btn ll-btn--default ll-btn--sm" @click="openDialog('connect')">
                <Icon name="clipboard" :size="12" /> {{ L.target.pasteLabel }}
              </button>
            </div>
          </div>
        </div>

        <div class="idle-task-bar">
          <span class="itb-icon"><Icon name="clock" :size="15" /></span>
          <div class="ll-col">
            <span class="itb-title">{{ L.hintTitle }}</span>
            <span class="ll-text-tertiary">{{ L.hintDesc }}</span>
          </div>
          <span class="ll-link" @click="state.mainView = 'history'">{{ L.hintLink }} <Icon name="chevronRight" :size="11" /></span>
        </div>
      </template>

      <!-- ---------- 传输中 ---------- -->
      <template v-else>
        <div class="ll-card channel-card">
          <div class="channel-head">
            <span class="ll-section-title"><Icon name="link" :size="14" class="ch-ico" /> 通道信息</span>
            <span class="ll-text-tertiary ll-mono">session 6f3a9c2e</span>
          </div>
          <div class="channel-grid">
            <div><span class="cg-k">对端设备</span><span class="cg-v">李工的 MacBook Pro</span></div>
            <div><span class="cg-k">连接地址</span><span class="cg-v ll-mono">[2408:8207:…:9c1d]:51820</span></div>
            <div><span class="cg-k">往返延迟</span><span class="cg-v">38 ms</span></div>
            <div><span class="cg-k">合计速率</span><span class="cg-v">41.6 MB/s</span></div>
          </div>
        </div>

        <div
          v-for="(t, i) in activeTasks"
          :key="t.id"
          class="ll-card task-card"
          :class="{ 'task-card--active': i === 0 }"
        >
          <div class="task-head">
            <span class="file-ico" :class="i === 0 ? 'ico-video' : 'ico-pdf'">{{ i === 0 ? 'MP4' : 'PDF' }}</span>
            <span class="task-name ll-truncate">{{ t.name }}</span>
            <span class="task-speed">{{ t.speedText || '—' }}</span>
            <button class="task-btn" :title="t.status === 'transferring' ? '暂停' : '继续'" @click="t.status === 'transferring' ? pauseTask(t.id) : resumeTask(t.id)">
              <Icon :name="t.status === 'transferring' ? 'pause' : 'play'" :size="12" />
            </button>
            <button class="task-btn" title="取消" @click="cancelTask(t.id)"><Icon name="close" :size="12" /></button>
          </div>
          <div class="task-detail ll-text-tertiary">{{ t.detail }}</div>
          <div class="ll-progress"><i :style="{ width: t.progress + '%' }" /></div>
          <div class="task-foot">
            <span class="ll-text-tertiary">{{ t.progress }}% {{ t.transferred }}</span>
            <span class="ll-text-tertiary">接收端已确认 seq {{ t.seq }}</span>
          </div>
        </div>

        <div class="ll-card rate-card">
          <div class="rate-head">
            <span class="ll-section-title"><Icon name="chart" :size="14" class="ch-ico" /> 近 60s 速率</span>
            <span class="ll-text-tertiary">峰值 48.7 MB/s · 平均 41.6 MB/s</span>
          </div>
          <svg class="rate-svg" viewBox="0 0 600 90" preserveAspectRatio="none">
            <defs>
              <linearGradient id="llRate" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0%" stop-color="var(--ll-color-brand)" stop-opacity="0.22" />
                <stop offset="100%" stop-color="var(--ll-color-brand)" stop-opacity="0" />
              </linearGradient>
            </defs>
            <path :d="rateArea" fill="url(#llRate)" />
            <polyline :points="rateLine" fill="none" stroke="var(--ll-color-brand)" stroke-width="2" />
          </svg>
          <div class="rate-axis ll-text-tertiary"><span>60s</span><span>30s</span><span>now</span></div>
        </div>

        <div class="queue-bar">
          <span class="ll-text-tertiary"><Icon name="clock" :size="13" class="q-ico" /> 排队中 {{ queuedTasks.length }} 项：{{ queuedTasks[0]?.name }} · 128 MB</span>
          <span class="ll-link">立即启动</span>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.view { flex: 1; display: flex; flex-direction: column; min-width: 0; min-height: 0; }
.center-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--ll-space-3);
  padding: 6px var(--ll-space-5) 2px;
}
.ch-title h2 { font-size: var(--ll-fs-xl); font-weight: var(--ll-fw-semibold); }
.ch-sub { font-size: var(--ll-fs-sm); margin-top: 2px; }
.ch-actions { display: flex; align-items: center; gap: var(--ll-space-2); flex: none; }
.center-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 0 var(--ll-space-5) var(--ll-space-3);
  display: flex;
  flex-direction: column;
  gap: 3px;
}
/* 防止 flex 子项被纵向压缩导致表格行被裁切；超出时由容器滚动 */
.center-scroll > * { flex: none; }
/* 行高比上一版抬高，压缩其它间距来补偿，保证 780px 内区块完整 */
.file-table td { padding: 6px var(--ll-space-3); }

/* 拖放区 */
.dropzone {
  border: 1px dashed var(--ll-color-border-strong);
  border-radius: var(--ll-radius-md);
  background: var(--ll-color-surface-2);
  padding: 10px;
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  transition: border-color .15s, background .15s;
}
.dropzone:hover { border-color: var(--ll-color-brand); }
.dropzone--over {
  border-color: var(--ll-color-brand);
  background: var(--ll-color-brand-light);
}
.dz-icon { color: var(--ll-color-brand); }
.dz-title { font-weight: var(--ll-fw-medium); font-size: var(--ll-fs-md); }
.dz-sub { font-size: var(--ll-fs-xs); }

/* 文件表 */
.file-card, .device-card { overflow: hidden; }
/* fixed 布局：列宽由表头一次定死，剩余空间全部给「文件名」。
   auto 布局下窄列会被按比例压到内容宽度以下，「4.2 MB」「11-14 10:22」就折成两行。
   宽度按最小窗口（1040px → 中央区约 500px，单元格左右内边距 12px）内容单行反推。 */
.file-table { table-layout: fixed; }
.file-table th:nth-child(1) { width: auto; }  /* 文件名：唯一弹性列，超长由 .ll-truncate 省略 */
.file-table th:nth-child(2) { width: 64px; }  /* 大小  4.2 MB */
.file-table th:nth-child(3) { width: 94px; }  /* 修改时间 11-14 10:22 */
.file-table th:nth-child(4) { width: 72px; }  /* 状态  待发送 */
.file-table th:nth-child(5) { width: 44px; }  /* 移除按钮 */
.file-table th { white-space: nowrap; }
.file-table td:nth-child(2),
.file-table td:nth-child(3),
.file-table td:nth-child(4),
.file-table td:nth-child(5) { white-space: nowrap; }
.file-table td:nth-child(1) { color: var(--ll-color-text); }
.empty-row { padding: 0; }
.file-name { display: flex; align-items: center; gap: var(--ll-space-2); min-width: 0; }
.file-ico {
  width: 20px;
  height: 20px;
  flex: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--ll-radius-xs);
  color: #fff;
  font-size: 8px;
  font-weight: var(--ll-fw-bold);
  letter-spacing: -0.3px;
}
.ico-pdf { background: var(--ll-color-danger); }
.ico-video { background: var(--ll-color-brand); color: var(--ll-color-text-inverse); }
.ico-zip { background: var(--ll-color-warning); }
.row-x {
  width: 20px; height: 20px; border: none; background: transparent;
  color: var(--ll-color-text-quaternary); border-radius: var(--ll-radius-xs);
  display: inline-flex; align-items: center; justify-content: center;
}
.row-x:hover { background: var(--ll-color-surface-3); color: var(--ll-color-danger); }
/* 表底汇总行：同样不画分割线，用浅灰底与数据行区分 */
.file-foot {
  display: flex; justify-content: space-between;
  padding: var(--ll-space-1) var(--ll-space-3);
  background: var(--ll-color-surface-2);
  font-size: var(--ll-fs-xs);
}

/* 连接方式双卡片 */
.method-head { display: flex; align-items: center; justify-content: space-between; margin-top: var(--ll-space-1); }
.method-hint { font-size: var(--ll-fs-xs); }
.method-cards { display: grid; grid-template-columns: 1fr 1fr; gap: var(--ll-space-3); }
.method-card { padding: var(--ll-space-3); display: flex; flex-direction: column; gap: var(--ll-space-2); }
.method-card--brand { border-color: var(--ll-color-brand); }
.mc-head { display: flex; align-items: center; gap: var(--ll-space-2); }
.mc-icon { color: var(--ll-color-text-tertiary); display: inline-flex; }
.mc-icon.brand { color: var(--ll-color-brand); }
.mc-title { font-weight: var(--ll-fw-semibold); font-size: var(--ll-fs-md); flex: 1; }
.mc-desc { font-size: var(--ll-fs-xs); line-height: var(--ll-lh-relaxed); min-height: 32px; }

.idle-task-bar {
  display: flex; align-items: center; gap: var(--ll-space-3);
  padding: var(--ll-space-1) var(--ll-space-3);
  border: 1px solid var(--ll-color-border);
  border-radius: var(--ll-radius-md);
  background: var(--ll-color-surface-2);
}
.itb-icon { color: var(--ll-color-text-tertiary); display: inline-flex; }
.itb-title { font-size: var(--ll-fs-md); font-weight: var(--ll-fw-medium); }
.idle-task-bar .ll-col { flex: 1; min-width: 0; font-size: var(--ll-fs-xs); }
.idle-task-bar .ll-link { display: inline-flex; align-items: center; gap: 2px; white-space: nowrap; }

/* 定版（main-ready-locked.png）：选择目标设备卡 */
.locked-foot { justify-content: flex-start; }
.target-card { padding: 0; overflow: hidden; }
.tc-head {
  display: flex; align-items: center; justify-content: space-between;
  padding: var(--ll-space-1) var(--ll-space-3);
  background: var(--ll-color-info-surface);
  border-bottom: 1px solid var(--ll-color-border);
}
.tc-title {
  display: inline-flex; align-items: center; gap: var(--ll-space-2);
  color: var(--ll-color-brand); font-weight: var(--ll-fw-semibold); font-size: var(--ll-fs-md);
}
.tc-head > span:last-child { font-size: var(--ll-fs-xs); }
/* 状态提示条：极浅绿底 + 细下边界，与下方灰色表头分成两层，
   避免「同网段」行和表头同为 #FAFBFC 而融成一块。
   这条线是卡片内区块边界，不是表格行分隔线。 */
.tc-hint {
  display: flex; align-items: center; gap: var(--ll-space-2);
  padding: 7px var(--ll-space-3);
  background: var(--ll-color-success-bg);
  /* 用绿色语义边框而非淡灰 divider：换到绿色系色板时仍能与灰色表头分层 */
  border-bottom: 1px solid var(--ll-color-success-border);
  color: var(--ll-color-success); font-size: var(--ll-fs-xs);
}
/* 设备表：同样 fixed 布局 + 窄列不换行，「设备」列吸收剩余宽度。
   地址列给 ellipsis，避免出现长 IPv6 时把延迟/操作列挤到折行。 */
.device-table { table-layout: fixed; }
.device-table th:nth-child(1) { width: auto; }
.device-table th:nth-child(2) { width: 116px; }
.device-table th:nth-child(3) { width: 60px; }
.device-table th:nth-child(4) { width: 84px; }
.device-table th { white-space: nowrap; }
.device-table td { padding: 6px var(--ll-space-3); }
.device-table td:nth-child(2) {
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.device-table td:nth-child(3),
.device-table td:nth-child(4) { white-space: nowrap; }
.device-name { display: flex; align-items: center; gap: var(--ll-space-2); min-width: 0; }
.device-ico { color: var(--ll-color-brand); display: inline-flex; flex: none; }
.device-platform { font-size: 10px; }
.device-platform.warn { color: var(--ll-color-warning); }
.device-lat { color: var(--ll-color-success); }
.device-lat.warn { color: var(--ll-color-warning); }
/* 跨网凭证条（定版原型 main-ready-locked.png）：
   第一行标题 = 浅灰分区条，第二行说明 + 按钮 = 纯白无底色；不画线，靠明度分层 */
.tc-cross { background: var(--ll-color-surface); }
.tc-cross-title {
  display: flex; align-items: center; gap: var(--ll-space-2);
  padding: 6px var(--ll-space-3);
  background: var(--ll-color-surface-2);
  color: var(--ll-color-text-secondary); font-size: var(--ll-fs-xs);
}
.tc-cross-body {
  display: flex; align-items: center; gap: var(--ll-space-2);
  padding: var(--ll-space-2) var(--ll-space-3);
  background: var(--ll-color-surface);
}
.tc-cross-body > span { flex: 1; min-width: 0; font-size: 10px; white-space: nowrap; }
.tc-cross-body .ll-btn { flex: none; white-space: nowrap; }

/* 传输中 */
.channel-card { padding: var(--ll-space-3); }
.channel-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--ll-space-3); }
.channel-head .ll-section-title { display: inline-flex; align-items: center; gap: var(--ll-space-2); }
.ch-ico { color: var(--ll-color-brand); }
.channel-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: var(--ll-space-3); }
.channel-grid > div { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.cg-k { font-size: var(--ll-fs-xs); color: var(--ll-color-text-tertiary); }
.cg-v { font-size: var(--ll-fs-sm); color: var(--ll-color-text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.task-card { padding: var(--ll-space-3); }
.task-card--active { border-color: var(--ll-color-brand); }
.task-head { display: flex; align-items: center; gap: var(--ll-space-2); }
.task-name { flex: 1; font-weight: var(--ll-fw-medium); font-size: var(--ll-fs-md); }
.task-speed { color: var(--ll-color-brand); font-weight: var(--ll-fw-semibold); font-size: var(--ll-fs-md); font-family: var(--ll-font-mono); }
.task-btn {
  width: 22px; height: 22px; flex: none; border: 1px solid var(--ll-color-border);
  background: var(--ll-color-surface); border-radius: var(--ll-radius-xs);
  color: var(--ll-color-text-tertiary);
  display: inline-flex; align-items: center; justify-content: center;
}
.task-btn:hover { color: var(--ll-color-brand); border-color: var(--ll-color-brand); }
.task-detail { font-size: var(--ll-fs-xs); margin: var(--ll-space-2) 0; }
.task-foot { display: flex; justify-content: space-between; gap: var(--ll-space-3); font-size: var(--ll-fs-xs); margin-top: var(--ll-space-2); }

.rate-card { padding: var(--ll-space-3); }
.rate-head { display: flex; align-items: center; justify-content: space-between; }
.rate-head .ll-section-title { display: inline-flex; align-items: center; gap: var(--ll-space-2); }
.rate-head > span:last-child { font-size: var(--ll-fs-xs); }
.rate-svg { width: 100%; height: 76px; display: block; margin-top: var(--ll-space-2); }
.rate-axis { display: flex; justify-content: space-between; font-size: 10px; }

.queue-bar {
  display: flex; align-items: center; justify-content: space-between;
  padding: var(--ll-space-3);
  border: 1px solid var(--ll-color-border);
  border-radius: var(--ll-radius-md);
  background: var(--ll-color-surface-2);
  font-size: var(--ll-fs-xs);
}
.queue-bar > span:first-child { display: inline-flex; align-items: center; gap: var(--ll-space-2); }
.q-ico { color: var(--ll-color-text-tertiary); }
</style>
