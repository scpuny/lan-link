// =====================================================================
// LanLink 传输记录持久化（F24–F28）
// ---------------------------------------------------------------------
// 本地 localStorage 保存，保留 30 天自动清理；不上传云端、不存完整序列号/IP。
// 首次访问用原型数据（补足到 18 条支撑 3 页分页）播种；真实传输完成事件
// 由 useUi 调用 recordFromProgress() 追加。
// =====================================================================
import { ref, computed } from 'vue'
import type { TransferProgress } from '../types/invite'
import { HISTORY_ROWS, type HistoryRow } from '../data/views'

const KEY = 'll-history-v1'
const RETAIN_DAYS = 30

function seedRows(): HistoryRow[] {
  const base = HISTORY_ROWS.map((r) => ({ ...r }))
  // 补足到 18 条：复用原型行，改 id / 时间前缀，保证分页有 3 页数据
  const peers = ['李工的 MacBook Pro', 'OFFICE-PC-07', 'zhang-mac', 'chen 的 Android', '设计部-NAS']
  const links = ['P2P 直连', '同网段', '序列号', '局域网短码']
  const days = ['11-12', '11-11', '11-10', '11-09', '11-08', '11-07']
  let n = base.length
  while (n < 18) {
    const d = days[(n - base.length) % days.length]
    const tm = `${String(9 + (n % 9)).padStart(2, '0')}:${String((n * 7) % 60).padStart(2, '0')}`
    const ok = n % 7 !== 0
    base.push({
      id: `seed-${n}`,
      time: `${d} ${tm}`,
      direction: n % 2 ? '接收' : '发送',
      peer: peers[n % peers.length],
      linkType: links[n % links.length],
      files: `${1 + (n % 4)} 个`,
      size: `${20 + n * 7} MB`,
      duration: ok ? `${3 + (n % 20)} s` : '—',
      status: ok ? '成功' : n % 3 === 0 ? 'E006' : 'E009',
      tone: ok ? 'success' : n % 3 === 0 ? 'danger' : 'warning',
    })
    n++
  }
  return base
}

function load(): HistoryRow[] {
  try {
    const raw = localStorage.getItem(KEY)
    if (!raw) {
      const seeded = seedRows()
      localStorage.setItem(KEY, JSON.stringify(seeded))
      return seeded
    }
    const rows = JSON.parse(raw) as HistoryRow[]
    // F24：30 天清理（原型种子时间固定为 11 月，测试期不过滤；真实记录带 ts 才按时间清）
    const cutoff = Date.now() - RETAIN_DAYS * 86400_000
    const kept = rows.filter((r) => {
      const meta = (r as any).ts as number | undefined
      return !meta || meta >= cutoff
    })
    if (kept.length !== rows.length) localStorage.setItem(KEY, JSON.stringify(kept))
    return kept
  } catch {
    return seedRows()
  }
}

const rows = ref<HistoryRow[]>(load())

function persist() {
  try { localStorage.setItem(KEY, JSON.stringify(rows.value)) } catch { /* 隐私模式忽略 */ }
}

/** 由传输完成事件追加一条记录（F24） */
function recordFromProgress(ev: TransferProgress) {
  if (ev.status !== 'done' && ev.status !== 'error') return
  const now = new Date()
  const p = (v: number) => String(v).padStart(2, '0')
  const mb = (ev.size / 1024 / 1024).toFixed(0)
  const ok = ev.status === 'done'
  rows.value.unshift({
    id: ev.taskId,
    time: `${p(now.getMonth() + 1)}-${p(now.getDate())} ${p(now.getHours())}:${p(now.getMinutes())}`,
    direction: '发送',
    peer: '直连设备',
    linkType: 'P2P 直连',
    files: '1 个',
    size: `${mb} MB`,
    duration: '—',
    status: ok ? '成功' : (ev.error?.match(/E00[1-9]/)?.[0] || 'E009'),
    tone: ok ? 'success' : 'danger',
    ts: Date.now(),
  } as HistoryRow)
  persist()
}

/** F28：清空全部记录 */
function clearAll() {
  rows.value.splice(0)
  persist()
}

/** F26：把给定行导出为 CSV 并触发下载 */
function exportCsv(list: HistoryRow[]) {
  const head = ['时间', '方向', '对端设备', '链路', '文件', '大小', '耗时', '状态']
  const esc = (v: string) => `"${String(v).replace(/"/g, '""')}"`
  const body = list.map((r) =>
    [r.time, r.direction, r.peer, r.linkType, r.files, r.size, r.duration, r.status].map(esc).join(',')
  )
  const csv = '\uFEFF' + [head.map(esc).join(','), ...body].join('\r\n')
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `LanLink-传输记录-${new Date().toISOString().slice(0, 10)}.csv`
  a.click()
  URL.revokeObjectURL(url)
}

const total = computed(() => rows.value.length)

export function useRecords() {
  return { rows, total, recordFromProgress, clearAll, exportCsv }
}
