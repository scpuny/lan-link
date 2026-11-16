// =====================================================================
// LanLink 全局 UI 状态（轻量 reactive 单例，替代 Pinia 以减少依赖）
// 组件通过 useUi() 共享：主视图路由、发送区状态、设备/会话/任务、
// 接收偏好、记录筛选、对话框可见性、轻量提示条。
// =====================================================================
import { reactive, ref, computed } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { invoke, bridgeMode, hasTauri } from '../bridge'
import { HISTORY_TOTAL } from '../data/views'
import { useRecords } from './useRecords'
import { SEND_LOCKED } from '../data/send'
import type {
  SelfInfo,
  NearbyDevice,
  SessionInfo,
  TransferTask,
  SendFile,
  TransferProgress,
} from '../types/invite'

export type MainView = 'send' | 'receive' | 'history'
export type CenterState = 'idle_no_lan' | 'idle_with_lan' | 'transferring'
export type DialogKey = 'connect' | 'generate' | 'generateLan' | 'waitConnect' | 'settings'
export type HistoryRange = 'today' | '7d' | 'all'
export type HistoryDirection = 'all' | 'send' | 'receive'
export type HistoryStatus = 'all' | 'failed'

interface UiState {
  /** 左轨导航驱动的三个主视图（新增原型：接收文件 / 传输记录） */
  mainView: MainView
  /** 仅 send 视图内部的三态 */
  centerState: CenterState
  bridgeMode: 'tauri' | 'mock'
  self: SelfInfo | null
  nearby: NearbyDevice[]
  session: SessionInfo | null
  tasks: TransferTask[]
  /** F22：近 60 次速率采样（MB/s），驱动速率曲线 */
  rateHistory: number[]
  /** F1–F3：待发送文件队列（真实文件选择 / 拖拽 / 剪贴板写入） */
  sendFiles: SendFile[]
  /** F4：生效的接收保存目录 */
  saveDir: string
  /** F17：会话黑名单条数（真实，来自后端内存） */
  blacklistCount: number
  dialogs: Record<DialogKey, boolean>

  /** 接收视图：命令栏「允许被同网段设备发现」 */
  discoverable: boolean
  /** 接收视图：右栏偏好开关 */
  askBeforeReceive: boolean
  autoRename: boolean
  /** 接收视图：正在接收卡暂停态 */
  receivePaused: boolean

  /** 记录视图：命令栏搜索 / 时间范围 / 方向 / 状态 / 分页 */
  historyQuery: string
  historyRange: HistoryRange
  historyDirection: HistoryDirection
  historyStatus: HistoryStatus
  historyPage: number

  // 预览用：连接对话框注入错误码，便于演示 E001–E009 全部状态
  demoError: string | null
}

const state = reactive<UiState>({
  mainView: 'send',
  // 定版（main-ready-locked.png）是打开应用后的默认发送页
  centerState: 'idle_with_lan',
  bridgeMode: bridgeMode(),
  self: null,
  nearby: [],
  session: null,
  // 原型 main-transferring.png 的传输任务数据
  tasks: [
    {
      id: 't1', name: '会议录屏_1114_外网序列号评审.mp4', size: 862_000_000,
      progress: 62, speed: 34_200_000, status: 'transferring',
      detail: '862 MB · 分片 4 MB · 发送中',
      transferred: '534.4 MB / 862 MB · 剩余约 9.6s',
      seq: 1342, speedText: '34.2 MB/s',
    },
    {
      id: 't2', name: 'LanLink-设计规范-v1.2.pdf', size: 4_200_000,
      progress: 21, speed: 7_400_000, status: 'transferring',
      detail: '4.2 MB · 分片 4 MB · 发送中',
      transferred: '0.9 MB / 4.2 MB · 剩余约 0.5s',
      seq: 127, speedText: '7.4 MB/s',
    },
    {
      id: 't3', name: 'lanlink-protocol-src.zip', size: 128_000_000,
      progress: 0, speed: 0, status: 'queued',
    },
  ],
  rateHistory: [],
  // 定版默认待发送 2 项（与原型一致）；用户选择文件后由真实队列替换
  sendFiles: [],
  saveDir: '',
  blacklistCount: 0,
  dialogs: { connect: false, generate: false, generateLan: false, waitConnect: false, settings: false },

  discoverable: true,
  askBeforeReceive: true,
  autoRename: true,
  receivePaused: false,

  historyQuery: '',
  historyRange: '7d',
  historyDirection: 'all',
  historyStatus: 'all',
  historyPage: 1,

  demoError: null,
})

/** 左轨角标：接收文件 1（进行中），传输记录 18（总条数） */
const receiveBadge = 1
const historyBadge = HISTORY_TOTAL

/** 轻量提示条：用于「打开保存目录」等一次性动作的反馈 */
const toast = ref<string | null>(null)
let toastTimer: ReturnType<typeof setTimeout> | null = null

/** 正在进行的动作 key 集合：供按钮显示 loading / 禁用态（如 'rescan'） */
const busy = ref<Record<string, boolean>>({})

function showToast(message: string, duration = 2600) {
  toast.value = message
  if (toastTimer) clearTimeout(toastTimer)
  toastTimer = setTimeout(() => { toast.value = null }, duration)
}

/** 打开接收文件保存目录：走 Tauri invoke 桥接（非 HTTP），预览态走 mock */
async function openSaveDir() {
  try {
    const r = await invoke<{ path: string }>('open_save_dir')
    showToast(`已打开保存目录 · ${r.path}`)
  } catch (e) {
    showToast(`打开保存目录失败 · ${e instanceof Error ? e.message : String(e)}`)
  }
}

// ---------------------------------------------------------------------------
// F1–F4：文件入口动作（真实 Tauri IPC / 浏览器 mock 双通）
// ---------------------------------------------------------------------------

/** 去重追加：以 path 为键；无 path 的剪贴板文本项以 name 为键 */
function appendSendFiles(incoming: SendFile[]): number {
  let added = 0
  const seen = new Set(state.sendFiles.map((f) => f.path || `text:${f.name}`))
  for (const f of incoming) {
    const key = f.path || `text:${f.name}`
    if (seen.has(key)) continue
    seen.add(key)
    state.sendFiles.push(f)
    added++
  }
  return added
}

/** F1：选择文件（多选） */
async function pickFiles() {
  try {
    const files = await invoke<SendFile[]>('pick_files')
    const added = appendSendFiles(files)
    if (added) showToast(`已添加 ${added} 个文件 · 待发送 ${state.sendFiles.length} 项`)
    return added
  } catch (e) {
    showToast(`选择文件失败 · ${e instanceof Error ? e.message : String(e)}`)
    return 0
  }
}

/** F1：选择文件夹（递归） */
async function pickFolder() {
  try {
    const files = await invoke<SendFile[]>('pick_folder')
    const added = appendSendFiles(files)
    if (added) showToast(`已从文件夹添加 ${added} 个文件`)
    else if (files.length === 0) showToast('已取消选择文件夹')
    return added
  } catch (e) {
    showToast(`选择文件夹失败 · ${e instanceof Error ? e.message : String(e)}`)
    return 0
  }
}

/** F1：从本地路径列表追加（拖拽落区 / 菜单 open-file 复用） */
function addPathsToSend(paths: string[]) {
  const files: SendFile[] = paths.map((p) => {
    const name = p.split(/[\\/]/).pop() || p
    const dot = name.lastIndexOf('.')
    return {
      path: p,
      name,
      size: 0,
      modified: Date.now(),
      ext: dot > 0 ? name.slice(dot + 1).toUpperCase() : '',
    }
  })
  const added = appendSendFiles(files)
  if (added) showToast(`已添加 ${added} 个文件 · 待发送 ${state.sendFiles.length} 项`)
  return added
}

/** F2：从剪贴板发送（文本或路径） */
async function pasteSend() {
  try {
    const text = (await invoke<string>('read_clipboard')).trim()
    if (!text) {
      showToast('剪贴板为空')
      return 0
    }
    // 逐行处理：本地已存在路径交给后端无法批量 stat，这里按文本项入队；
    // 真实路径场景由拖拽 / 选择文件覆盖，剪贴板主要承载跨应用文本内容。
    const lines = text.split(/\r?\n/).map((s) => s.trim()).filter(Boolean)
    const files: SendFile[] = lines.map((line) => ({
      path: '',
      name: line.length > 28 ? line.slice(0, 28) + '…' : line,
      size: new TextEncoder().encode(line).length,
      modified: Date.now(),
      ext: 'TXT',
    }))
    const added = appendSendFiles(files)
    if (added) showToast(`已从剪贴板添加 ${added} 项`)
    return added
  } catch (e) {
    showToast(`读取剪贴板失败 · ${e instanceof Error ? e.message : String(e)}`)
    return 0
  }
}

/** F3：移除单个待发送项 */
function removeSendFile(pathOrName: string) {
  const i = state.sendFiles.findIndex((f) => (f.path || f.name) === pathOrName)
  if (i >= 0) state.sendFiles.splice(i, 1)
}

/** F3：清空待发送列表 */
function clearSendFiles() {
  const n = state.sendFiles.length
  state.sendFiles.splice(0)
  if (n) showToast(`已清空 ${n} 个待发送文件`)
}

/** 待发送汇总：数量 + 总字节（供文件表底栏与状态栏复用） */
const sendSummary = computed(() => ({
  count: state.sendFiles.length,
  bytes: state.sendFiles.reduce((sum, f) => sum + f.size, 0),
}))

/** F4：加载生效保存目录 */
async function loadSaveDir() {
  try {
    const r = await invoke<{ path: string }>('get_save_dir')
    state.saveDir = r.path
  } catch {
    /* 预览态忽略 */
  }
}

/** F4：更改保存目录（原生目录选择 + 持久化） */
async function chooseSaveDir() {
  try {
    const r = await invoke<{ path: string }>('choose_save_dir')
    state.saveDir = r.path
    showToast(`保存目录已更新 · ${r.path}`)
  } catch (e) {
    showToast(`更改保存目录失败 · ${e instanceof Error ? e.message : String(e)}`)
  }
}

/** F17：加载黑名单条数 */
async function loadBlacklistCount() {
  try {
    state.blacklistCount = await invoke<number>('get_blacklist_count')
  } catch {
    /* 预览态忽略 */
  }
}

/** F17：清空黑名单 */
async function clearBlacklist() {
  try {
    state.blacklistCount = await invoke<number>('clear_blacklist')
    showToast('已清空会话黑名单')
  } catch (e) {
    showToast(`清空失败 · ${e instanceof Error ? e.message : String(e)}`)
  }
}

// ---------------------------------------------------------------------------
// F18–F23：传输引擎控制（进度由后端 ll-transfer-progress 事件推送）
// ---------------------------------------------------------------------------

/** 把后端进度事件合并进响应式任务表；返回匹配到的任务 */
function applyTransferProgress(ev: TransferProgress) {
  const fmtSpeed = (b: number) => (b >= 1024 * 1024 ? `${(b / 1024 / 1024).toFixed(1)} MB/s` : `${(b / 1024).toFixed(0)} KB/s`)
  const fmtBytes = (b: number) => (b >= 1024 * 1024 ? `${(b / 1024 / 1024).toFixed(1)} MB` : `${(b / 1024).toFixed(0)} KB`)
  let task = state.tasks.find((t) => t.id === ev.taskId)
  if (!task) {
    task = {
      id: ev.taskId, name: ev.name, size: ev.size, progress: 0, speed: 0,
      status: 'transferring', detail: `${fmtBytes(ev.size)} · 分片 4 MB · 发送中`,
    }
    state.tasks.push(task)
  }
  task.progress = ev.progress
  task.speed = ev.speed
  task.seq = ev.seq
  task.speedText = ev.status === 'transferring' ? fmtSpeed(ev.speed) : '—'
  task.transferred = `${fmtBytes(ev.transferred)} / ${fmtBytes(ev.size)}`
  if (ev.status === 'done') {
    task.status = 'done'
    task.progress = 100
    task.detail = `${fmtBytes(ev.size)} · 已完成 · Blake3 校验通过`
  } else if (ev.status === 'cancelled') {
    task.status = 'queued'
    task.detail = '已取消'
  } else if (ev.status === 'error') {
    task.status = 'queued'
    task.detail = `失败 · ${ev.error || '未知错误'}`
  } else {
    task.status = 'transferring'
  }
  if (ev.status === 'transferring') {
    state.rateHistory.push(ev.speed / 1024 / 1024)
    if (state.rateHistory.length > 30) state.rateHistory.shift()
  }
  // F24：终态写入本机传输记录（每个任务只记一次）
  if ((ev.status === 'done' || ev.status === 'error') && !recordedTasks.has(ev.taskId)) {
    recordedTasks.add(ev.taskId)
    useRecords().recordFromProgress(ev)
  }
}
const recordedTasks = new Set<string>()

/** 注册传输进度事件监听（真机态），浏览器预览态跳过 */
async function initTransferEvents() {
  if (!hasTauri()) return
  try {
    await listen<TransferProgress>('ll-transfer-progress', (e) => applyTransferProgress(e.payload))
  } catch {
    /* 事件通道不可用时忽略 */
  }
}

async function transferCtl(cmd: 'pause' | 'resume' | 'cancel', id: string) {
  try {
    await invoke(`transfer_${cmd}`, { id })
  } catch (e) {
    showToast(`操作失败 · ${e instanceof Error ? e.message : String(e)}`)
  }
}
async function pauseTask(id: string) { await transferCtl('pause', id) }
async function resumeTask(id: string) { await transferCtl('resume', id) }
async function cancelTask(id: string) { await transferCtl('cancel', id) }
async function pauseAllTasks() {
  try { await invoke('transfer_pause_all') } catch { /* 预览态忽略 */ }
}

/** F10：切换本机可发现状态（本地 state + 后端广播开关） */
async function setDiscoverable(on: boolean) {
  state.discoverable = on
  try {
    await invoke('set_discoverable', { on })
  } catch {
    /* 预览态忽略 */
  }
}

/** F11：重新扫描设备，刷新设备表（带 loading 态，按钮显示 spinner 并禁用） */
async function rescanLan() {
  if (busy.value.rescan) return
  busy.value.rescan = true
  try {
    state.nearby = await invoke<NearbyDevice[]>('scan_lan')
    showToast(`扫描完成 · 发现 ${state.nearby.length} 台设备`)
  } catch (e) {
    showToast(`扫描失败 · ${e instanceof Error ? e.message : String(e)}`)
  } finally {
    busy.value.rescan = false
  }
}

/** 派生：状态栏主文案，按主视图分派 */
const statusText = computed(() => {
  if (state.mainView === 'receive') {
    return state.receivePaused
      ? '接收已暂停 · 1 个进行 · 1 个排队'
      : '接收中 · 1 个进行 · 1 个排队 · 34.2 MB/s'
  }
  if (state.mainView === 'history') {
    return `传输记录 · 共 ${HISTORY_TOTAL} 条 · 失败 2`
  }
  if (state.centerState === 'transferring') {
    const n = state.tasks.filter((t) => t.status === 'transferring').length
    return `传输中 · ${n} 个任务 · 41.6 MB/s`
  }
  if (state.centerState === 'idle_with_lan') {
    // 用户已选文件时反映真实数量；否则保持定版原型文案
    if (state.sendFiles.length) {
      return `就绪 · 待发送 ${state.sendFiles.length} 个文件 · 已发现 ${state.nearby.length} 台设备`
    }
    return SEND_LOCKED.statusText
  }
  return '就绪 · 无活动会话'
})

/** 浏览器预览态：?view=receive|history&send=transferring|idle_with_lan 直达指定状态，便于视觉回归 */
function applyPreviewParams() {
  if (state.bridgeMode !== 'mock' || typeof window === 'undefined') return
  const p = new URLSearchParams(window.location.search)
  const v = p.get('view')
  if (v === 'receive' || v === 'history' || v === 'send') state.mainView = v
  const s = p.get('send')
  if (s === 'transferring' || s === 'idle_with_lan' || s === 'idle_no_lan') state.centerState = s
}

async function init() {
  try {
    state.self = await invoke<SelfInfo>('get_self_info')
    state.nearby = await invoke<NearbyDevice[]>('scan_lan')
    await loadSaveDir()
    await loadBlacklistCount()
    // 默认即「定版就绪态」（main-ready-locked.png：选择目标设备卡）；
    // main-ready.png 的双连接卡片作为无设备变体，可通过 ?send=idle_no_lan 预览。
  } catch (e) {
    // 预览态忽略
  }
  await initTransferEvents()
  applyPreviewParams()
}

function navigate(view: MainView) {
  state.mainView = view
}

function openDialog(k: DialogKey) {
  state.dialogs[k] = true
}
function closeDialog(k: DialogKey) {
  state.dialogs[k] = false
}

export function useUi() {
  return {
    state,
    statusText,
    toast,
    busy,
    receiveBadge,
    historyBadge,
    init,
    navigate,
    openDialog,
    closeDialog,
    openSaveDir,
    showToast,
    invoke,
    // F1–F4
    pickFiles,
    pickFolder,
    pasteSend,
    addPathsToSend,
    removeSendFile,
    clearSendFiles,
    sendSummary,
    loadSaveDir,
    chooseSaveDir,
    setDiscoverable,
    rescanLan,
    loadBlacklistCount,
    clearBlacklist,
    applyTransferProgress,
    pauseTask,
    resumeTask,
    cancelTask,
    pauseAllTasks,
  }
}
