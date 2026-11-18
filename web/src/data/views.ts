// =====================================================================
// 「接收文件」「传输记录」两个主窗口视图的类型、文案与空态常量
// ---------------------------------------------------------------------
// 所有数据初值均为空：真机由 Tauri 采集与传输事件填充，
// 浏览器预览态保持空态（不看假数据，只看布局）。
// 文案逐字对齐定版设计稿；改文案时同步此文件。
// =====================================================================

export interface ReceiveItem {
  id: string
  name: string
  size: string
  note: string
  ext: string
  tone: 'pdf' | 'video' | 'zip' | 'img'
}

export interface HistoryRow {
  id: string
  time: string
  direction: '发送' | '接收'
  peer: string
  linkType: string
  files: string
  size: string
  duration: string
  status: string
  tone: 'success' | 'warning' | 'danger'
  /** 真实传输记录才有：完成时间戳与原始字节数，用于重新格式化大小 */
  ts?: number
  bytes?: number
}

/** 接收视图：当前接收通道（无会话时为空） */
export const RECEIVE_CHANNEL = {
  peerName: '—',
  title: '等待接收任务',
  addr: '—',
  speed: '—',
  saveDir: '',
  subtitle: '同网段设备可直连；跨网设备需提供序列号或口令',
}

/** 接收视图：正在接收的单据 */
export const RECEIVE_ACTIVE = {
  name: '',
  detail: '',
  progress: 0,
  percentText: '0%',
  transferred: '',
}

/** 接收视图：等待队列 / 本次会话已完成 */
export const RECEIVE_QUEUE: ReceiveItem[] = []
export const RECEIVE_DONE: ReceiveItem[] = []

export const RECEIVE_HINT =
  '接收端只接受 seq 大于上次确认值的分片，重复分片自动丢弃；断开后可从断点续传'

/** 接收视图右栏：本机可被发现（真机值由 state.self 覆盖） */
export const DISCOVER_FIELDS = [
  { k: '设备名', v: '—' },
  { k: '局域网', v: '—' },
  { k: '监听端口', v: '42210' },
  { k: '发现服务', v: 'UDP 42211 · 等待状态', tone: 'muted' as const },
]

/** 接收视图右栏：偏好开关初值 */
export const RECEIVE_PREFS = [
  { key: 'askBeforeReceive' as const, label: '接收前先询问', on: true },
  { key: 'autoRename' as const, label: '同名文件自动重命名', on: true },
]

/** 传输记录：顶部四张统计卡（无记录时全 0） */
export const HISTORY_STATS = [
  { key: 'total', label: '总传输', value: '0', sub: '次会话', tone: 'plain' as const },
  { key: 'success', label: '成功', value: '0', sub: '成功率 0%', tone: 'success' as const },
  { key: 'failed', label: '失败', value: '0', sub: '含错误码', tone: 'danger' as const },
  { key: 'traffic', label: '累计流量', value: '0 B', sub: '发送 0 / 接收 0', tone: 'plain' as const },
]

/** 传输记录：8 列表格数据，初值为空 */
export const HISTORY_ROWS: HistoryRow[] = []
export const HISTORY_TOTAL = 0
export const HISTORY_HINT =
  '记录仅存本机，30 天自动清理；失败记录保留错误码与诊断快照，便于复现'

/** 记录视图右栏：近 7 天概览 */
export const HISTORY_OVERVIEW = {
  range: '—',
  bars: [
    { label: '发送', value: 0, max: 1, tone: 'brand' as const },
    { label: '接收', value: 0, max: 1, tone: 'success' as const },
    { label: '失败', value: 0, max: 1, tone: 'danger' as const },
  ],
}

/** 记录视图右栏：失败原因分布 */
export const HISTORY_FAILURES: { code: string; desc: string; count: number }[] = []

/** 记录视图左栏：记录策略 */
export const HISTORY_POLICY = [
  { k: '本地保存', v: '保留 30 天' },
  { k: '当前条数', v: '0 条' },
  { k: '不上传云端', v: '' },
]
