// =====================================================================
// 「接收文件」「传输记录」两个主窗口视图的原型数据
// ---------------------------------------------------------------------
// 来源：calicat 新增原型
//   docs/prototypes/main-receiving.png  · LanLink 主窗口 · 接收文件（接收中）
//   docs/prototypes/main-history.png    · LanLink 主窗口 · 传输记录
// 文案逐字对齐原型；改文案时同步此文件，避免运行态覆盖设计稿。
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
}

/** 接收视图：当前接收通道 */
export const RECEIVE_CHANNEL = {
  peerName: '李工的 MacBook Pro',
  title: '正在接收来自『李工的 MacBook Pro』的文件',
  addr: '[2408:8207:…:9c1d]:51820 · 通道加密关闭 · 往返 38ms',
  speed: '34.2 MB/s',
  saveDir: 'D:\\LanLink\\Downloads',
  subtitle: '同网段设备可直连；跨网设备需提供序列号或口令',
}

/** 接收视图：正在接收的单据 */
export const RECEIVE_ACTIVE = {
  name: '会议录屏_1114_外网序列号评审.mp4',
  detail: '862 MB · 分片 4 MB · 接收中',
  progress: 62,
  percentText: '62%',
  transferred: '534.4 MB / 862 MB · 剩余约 9.6s',
}

/** 接收视图：等待队列 */
export const RECEIVE_QUEUE: ReceiveItem[] = [
  {
    id: 'q1',
    name: 'LanLink-设计规范-v1.2.pdf',
    size: '4.2 MB',
    note: '等待前序完成',
    ext: 'PDF',
    tone: 'pdf',
  },
]

/** 接收视图：本次会话已完成 */
export const RECEIVE_DONE: ReceiveItem[] = [
  {
    id: 'd1',
    name: 'lanlink-protocol-src.zip',
    size: '128 MB',
    note: '已完成 · 10:21',
    ext: 'ZIP',
    tone: 'zip',
  },
  {
    id: 'd2',
    name: 'UI-截图-全部.png',
    size: '18.6 MB',
    note: '已完成 · 10:19',
    ext: 'PNG',
    tone: 'img',
  },
]

export const RECEIVE_HINT =
  '接收端只接受 seq 大于上次确认值的分片，重复分片自动丢弃；断开后可从断点续传'

/** 接收视图右栏：本机可被发现 */
export const DISCOVER_FIELDS = [
  { k: '设备名', v: 'DESKTOP-LL-01' },
  { k: '局域网', v: '192.168.1.24' },
  { k: '监听端口', v: '42210' },
  { k: 'mDNS', v: '广播中 (_lanlink_tcp)', tone: 'success' as const },
]

/** 接收视图右栏：偏好开关初值 */
export const RECEIVE_PREFS = [
  { key: 'askBeforeReceive' as const, label: '接收前先询问', on: true },
  { key: 'autoRename' as const, label: '同名文件自动重命名', on: true },
]

/** 传输记录：顶部四张统计卡 */
export const HISTORY_STATS = [
  { key: 'total', label: '总传输', value: '18', sub: '次会话', tone: 'plain' as const },
  { key: 'success', label: '成功', value: '16', sub: '成功率 88.9%', tone: 'success' as const },
  { key: 'failed', label: '失败', value: '2', sub: 'E009 ×1 · E006 ×1', tone: 'danger' as const },
  { key: 'traffic', label: '累计流量', value: '4.8 GB', sub: '发送 3.1 / 接收 1.7', tone: 'plain' as const },
]

/** 传输记录：8 列表格数据（原型 5 行） */
export const HISTORY_ROWS: HistoryRow[] = [
  {
    id: 'h1', time: '11-14 10:24', direction: '发送',
    peer: '李工的 MacBook Pro', linkType: 'P2P 直连',
    files: '2 个', size: '866 MB', duration: '26 s', status: '成功', tone: 'success',
  },
  {
    id: 'h2', time: '11-14 10:21', direction: '接收',
    peer: '李工的 MacBook Pro', linkType: '同网段',
    files: '2 个', size: '147 MB', duration: '6 s', status: '成功', tone: 'success',
  },
  {
    id: 'h3', time: '11-13 17:02', direction: '发送',
    peer: 'OFFICE-PC-07', linkType: '序列号',
    files: '1 个', size: '128 MB', duration: '—', status: 'E009', tone: 'warning',
  },
  {
    id: 'h4', time: '11-13 16:48', direction: '发送',
    peer: 'zhang-mac', linkType: '序列号',
    files: '3 个', size: '1.2 GB', duration: '—', status: 'E006', tone: 'danger',
  },
  {
    id: 'h5', time: '11-13 15:20', direction: '接收',
    peer: 'chen 的 Android', linkType: '同网段',
    files: '2 个', size: '64 MB', duration: '4 s', status: '成功', tone: 'success',
  },
]

export const HISTORY_TOTAL = 18
export const HISTORY_HINT =
  '记录仅存本机，30 天自动清理；失败记录保留错误码与诊断快照，便于复现'

/** 记录视图右栏：近 7 天概览 */
export const HISTORY_OVERVIEW = {
  range: '11-08 至 11-14',
  bars: [
    { label: '发送', value: 12, max: 12, tone: 'brand' as const },
    { label: '接收', value: 6, max: 12, tone: 'success' as const },
    { label: '失败', value: 2, max: 12, tone: 'danger' as const },
  ],
}

/** 记录视图右栏：失败原因分布 */
export const HISTORY_FAILURES = [
  { code: 'E009', desc: '凭证合法但地址不通', count: 1 },
  { code: 'E006', desc: '质询超时', count: 1 },
]

/** 记录视图左栏：记录策略 */
export const HISTORY_POLICY = [
  { k: '本地保存', v: '保留 30 天' },
  { k: '当前条数', v: '18 条' },
  { k: '不上传云端', v: '' },
]
