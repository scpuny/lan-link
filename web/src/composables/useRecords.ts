// =====================================================================
// LanLink 传输记录持久化（F24–F28）
// ---------------------------------------------------------------------
// 本地 localStorage 保存，保留 30 天自动清理；不上传云端、不存完整序列号/IP。
// 首次访问为空；记录只由真实传输完成事件（useUi → recordFromProgress）追加。
// 旧版本写入的原型种子记录没有 ts 字段，加载时自动剔除。
// =====================================================================
import { ref, computed } from 'vue'
import type { TransferProgress } from '../types/invite'
import { type HistoryRow } from '../data/views'

const KEY = 'll-history-v1'
const RETAIN_DAYS = 30

/** 自适应单位：5 KB 不能被按 MB 取整成 0 MB */
function fmtSize(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(2)} GB`
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(1)} MB`
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(0)} KB`
  return `${bytes} B`
}

function load(): HistoryRow[] {
  try {
    const raw = localStorage.getItem(KEY)
    if (!raw) return []
    const rows = JSON.parse(raw) as HistoryRow[]
    // 只保留带 ts 的真实记录：原型种子（h1..h5 / seed-*）无 ts，自动剔除；30 天过期一并清掉
    const cutoff = Date.now() - RETAIN_DAYS * 86400_000
    const kept = rows.filter((r) => typeof r.ts === 'number' && r.ts >= cutoff)
    let changed = kept.length !== rows.length
    for (const r of kept) {
      if (typeof r.bytes === 'number' && r.bytes > 0) {
        const s = fmtSize(r.bytes)
        if (r.size !== s) { r.size = s; changed = true }
      } else if (r.ts && r.size === '0 MB') {
        // 旧版按 MB 取整把小文件记成 0 MB，已无原始字节数可反推，退为保守表述
        r.size = '< 1 MB'
        changed = true
      }
    }
    if (changed) localStorage.setItem(KEY, JSON.stringify(kept))
    return kept
  } catch {
    return []
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
  const ok = ev.status === 'done'
  rows.value.unshift({
    id: ev.taskId,
    time: `${p(now.getMonth() + 1)}-${p(now.getDate())} ${p(now.getHours())}:${p(now.getMinutes())}`,
    direction: '发送',
    peer: '直连设备',
    linkType: 'P2P 直连',
    files: '1 个',
    size: fmtSize(ev.size),
    duration: '—',
    status: ok ? '成功' : (ev.error?.match(/E00[1-9]/)?.[0] || 'E009'),
    tone: ok ? 'success' : 'danger',
    ts: Date.now(),
    bytes: ev.size,
  })
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
