<script setup lang="ts">
// 传输记录视图：对齐 docs/prototypes/main-history.png
import { computed, ref } from 'vue'
import Icon from '../../common/Icon.vue'
import EmptyState from '../../common/EmptyState.vue'
import { useUi } from '../../../composables/useUi'
import { useRecords } from '../../../composables/useRecords'
import { HISTORY_STATS, HISTORY_HINT, type HistoryRow } from '../../../data/views'

const { state } = useUi()
const { rows: allRows, total, clearAll, exportCsv } = useRecords()
const PAGE_SIZE = 6
const DAY_MS = 86_400_000
const diagRow = ref<HistoryRow | null>(null)

/** 把一条记录解析成「当天 00:00」的时间戳；真实记录优先用 ts，种子用 time 字符串 */
function dayIndex(r: HistoryRow): number | null {
  const ts = (r as HistoryRow & { ts?: number }).ts
  if (typeof ts === 'number') {
    const d = new Date(ts)
    return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime()
  }
  const m = r.time.match(/^(?:(\d{4})-)?(\d{2})-(\d{2})/)
  if (!m) return null
  const year = m[1] ? Number(m[1]) : new Date().getFullYear()
  return new Date(year, Number(m[2]) - 1, Number(m[3])).getTime()
}

/** 以数据集中最新一天为基准：演示种子仍是 11-14，真实传输后自动变成今天 */
const latestDay = computed(() => {
  let max = 0
  for (const r of allRows.value) {
    const d = dayIndex(r)
    if (d !== null && d > max) max = d
  }
  return max
})

/** F25：时间范围 / 方向 / 状态 / 搜索全量筛选（作用于真实持久化数据） */
const filtered = computed(() => {
  const q = state.historyQuery.trim().toLowerCase()
  const base = latestDay.value
  return allRows.value.filter((r) => {
    if (state.historyRange !== 'all') {
      const d = dayIndex(r)
      if (d === null) return false
      if (state.historyRange === 'today' && d !== base) return false
      if (state.historyRange === '7d' && (d > base || d < base - 6 * DAY_MS)) return false
    }
    if (state.historyDirection === 'send' && r.direction !== '发送') return false
    if (state.historyDirection === 'receive' && r.direction !== '接收') return false
    if (state.historyStatus === 'failed' && r.tone === 'success') return false
    if (q && !`${r.peer} ${r.linkType} ${r.status} ${r.files}`.toLowerCase().includes(q)) return false
    return true
  })
})
const pageCount = computed(() => Math.max(1, Math.ceil(filtered.value.length / PAGE_SIZE)))
/** 防御：筛选后页数变少时钳住页码，避免切到近 7 天仍是旧页而显示空表 */
const safePage = computed(() => Math.min(Math.max(1, state.historyPage), pageCount.value))
const rows = computed(() =>
  filtered.value.slice((safePage.value - 1) * PAGE_SIZE, safePage.value * PAGE_SIZE),
)

/** F27：由记录生成脱敏诊断快照行 */
function diagRows(r: HistoryRow): { k: string; v: string; tone?: string }[] {
  const code = r.status.match(/E00[1-9]/)?.[0]
  const sid = `ll-${r.id.replace(/[^a-z0-9]/gi, '').slice(0, 8).padEnd(8, '0')}`
  return [
    { k: '会话短号', v: sid },
    { k: '时间', v: r.time },
    { k: '方向 / 链路', v: `${r.direction} · ${r.linkType}` },
    { k: '错误码', v: code || '—', tone: r.tone },
    { k: '状态', v: r.status, tone: r.tone },
    { k: '文件 / 大小', v: `${r.files} · ${r.size}` },
  ]
}

/** F24：统计卡派生自真实记录（无记录时用原型常数） */
const stats = computed(() => {
  if (!allRows.value.length) return HISTORY_STATS
  const totalN = allRows.value.length
  const fail = allRows.value.filter((r) => r.tone !== 'success').length
  const succ = totalN - fail
  return [
    { key: 'total', label: '总传输', value: String(totalN), sub: '次会话', tone: 'plain' as const },
    { key: 'success', label: '成功', value: String(succ), sub: `成功率 ${((succ / totalN) * 100).toFixed(1)}%`, tone: 'success' as const },
    { key: 'failed', label: '失败', value: String(fail), sub: '含错误码', tone: 'danger' as const },
    HISTORY_STATS[3],
  ]
})

const ranges = [
  { key: 'today', label: '今天' },
  { key: '7d', label: '近 7 天' },
  { key: 'all', label: '全部' },
]
const directions = [
  { key: 'all', label: '全部方向' },
  { key: 'send', label: '发送' },
  { key: 'receive', label: '接收' },
]
const statuses = [
  { key: 'all', label: '全部状态' },
  { key: 'failed', label: '仅失败' },
]

const resultText = computed(() =>
  filtered.value.length === total.value
    ? `共 ${total.value} 条 · 已按时间倒序`
    : `筛选到 ${filtered.value.length} 条 · 共 ${total.value} 条`
)
const footText = computed(() => {
  if (!filtered.value.length) return `显示 0 条，共 ${total.value} 条`
  const start = (safePage.value - 1) * PAGE_SIZE + 1
  const end = Math.min(safePage.value * PAGE_SIZE, filtered.value.length)
  return `显示 ${start}-${end} 条，共 ${filtered.value.length} 条`
})
</script>

<template>
  <div class="view">
    <header class="center-head">
      <div class="ch-title">
        <h2>传输记录</h2>
        <p class="ch-sub ll-text-tertiary">本机流水，含失败原因与错误码，便于排查</p>
      </div>
      <div class="range-group">
        <button
          v-for="r in ranges"
          :key="r.key"
          class="range-btn"
          :class="{ active: state.historyRange === r.key }"
          @click="state.historyRange = r.key as any; state.historyPage = 1"
        >{{ r.label }}</button>
      </div>
    </header>

    <div class="center-scroll">
      <!-- 四张统计卡 -->
      <div class="stat-grid">
        <div v-for="s in stats" :key="s.key" class="ll-card stat-card">
          <span class="stat-label">{{ s.label }}</span>
          <span class="stat-value" :class="`tone-${s.tone}`">{{ s.value }}</span>
          <span class="stat-sub" :class="`tone-${s.tone}`">{{ s.sub }}</span>
        </div>
      </div>

      <!-- 筛选行 -->
      <div class="filter-bar">
        <Icon name="sliders" :size="13" class="fl-ico" />
        <div class="filter-group">
          <button
            v-for="d in directions"
            :key="d.key"
            :class="{ active: state.historyDirection === d.key }"
            @click="state.historyDirection = d.key as any; state.historyPage = 1"
          >{{ d.label }}</button>
        </div>
        <div class="filter-group status-group">
          <button
            v-for="s in statuses"
            :key="s.key"
            :class="{ active: state.historyStatus === s.key }"
            @click="state.historyStatus = s.key as any; state.historyPage = 1"
          >{{ s.label }}</button>
        </div>
        <span class="fl-right ll-text-tertiary">{{ resultText }}</span>
      </div>

      <!-- 记录表 -->
      <div class="ll-card table-card">
        <table class="ll-table history-table">
          <thead>
            <tr>
              <th>时间</th><th>方向</th><th>对端设备</th><th>文件</th>
              <th>大小</th><th>耗时</th><th>状态</th><th>操作</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="r in rows" :key="r.id">
              <td class="ll-mono cell-time">{{ r.time }}</td>
              <td><span class="dir-tag" :class="r.direction === '发送' ? 'send' : 'recv'">{{ r.direction }}</span></td>
              <td>
                <div class="peer">
                  <span class="peer-name ll-truncate">{{ r.peer }}</span>
                  <span class="ll-text-tertiary peer-link">· {{ r.linkType }}</span>
                </div>
              </td>
              <td>{{ r.files }}</td>
              <td>{{ r.size }}</td>
              <td>{{ r.duration }}</td>
              <td><span class="status-tag" :class="`tone-${r.tone}`">{{ r.status }}</span></td>
              <td><button class="row-open" title="查看诊断详情" @click="diagRow = r"><Icon name="file" :size="13" /></button></td>
            </tr>
            <tr v-if="!rows.length">
              <td colspan="8" class="empty-row">
                <EmptyState description="暂无数据" />
              </td>
            </tr>
          </tbody>
        </table>
        <div class="table-foot">
          <span class="ll-text-tertiary">{{ footText }}</span>
          <div class="pager">
            <button class="page-arrow" :disabled="state.historyPage === 1" @click="state.historyPage = Math.max(1, state.historyPage - 1)">
              <Icon name="chevronLeft" :size="12" />
            </button>
            <button
              v-for="p in pageCount"
              :key="p"
              class="page-num"
              :class="{ active: state.historyPage === p }"
              @click="state.historyPage = p"
            >{{ p }}</button>
            <button class="page-arrow" :disabled="state.historyPage === pageCount" @click="state.historyPage = Math.min(pageCount, state.historyPage + 1)">
              <Icon name="chevronRight" :size="12" />
            </button>
          </div>
        </div>
      </div>

      <!-- 底部提示条 -->
      <div class="hint-bar">
        <Icon name="info" :size="13" />
        <span>{{ HISTORY_HINT }}</span>
      </div>
    </div>

    <!-- F27：诊断详情面板 -->
    <div v-if="diagRow" class="diag-mask" @click.self="diagRow = null">
      <div class="diag-panel">
        <div class="diag-head">
          <span class="diag-title"><Icon name="file" :size="14" /> 诊断详情 · {{ diagRow.status }}</span>
          <button class="row-open" title="关闭" @click="diagRow = null"><Icon name="close" :size="13" /></button>
        </div>
        <div class="diag-body">
          <div v-for="r in diagRows(diagRow)" :key="r.k" class="diag-line">
            <span class="diag-k">{{ r.k }}</span>
            <span class="diag-v ll-mono" :class="r.tone ? `tone-${r.tone}` : ''">{{ r.v }}</span>
          </div>
        </div>
        <p class="diag-note ll-text-tertiary">脱敏快照：仅记录 session 前 8 位 + 时间戳 + 错误码，不含完整序列号或 IP。</p>
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
.center-scroll {
  flex: 1; overflow-y: auto;
  padding: 0 var(--ll-space-5) var(--ll-space-5);
  display: flex; flex-direction: column; gap: var(--ll-space-2);
}
.center-scroll > * { flex: none; }

/* 时间范围分段 */
.range-group { display: inline-flex; flex: none; }
.range-btn {
  height: 26px; padding: 0 var(--ll-space-3);
  /* 四边都保留 1px 边框，用负 margin 让相邻边叠成一条；选中项才能有完整右边框 */
  border: 1px solid var(--ll-color-border);
  margin-left: -1px;
  background: var(--ll-color-surface);
  color: var(--ll-color-text-secondary);
  font-size: var(--ll-fs-sm);
}
.range-btn:first-child { margin-left: 0; border-radius: var(--ll-radius-sm) 0 0 var(--ll-radius-sm); }
.range-btn:last-child { border-radius: 0 var(--ll-radius-sm) var(--ll-radius-sm) 0; }
.range-btn.active {
  position: relative; z-index: 1; /* 蓝框压在相邻灰框之上，四边完整可见 */
  color: var(--ll-color-brand);
  background: var(--ll-color-brand-light);
  border-color: var(--ll-color-brand);
  font-weight: var(--ll-fw-medium);
}

/* 统计卡 */
.stat-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: var(--ll-space-2); }
.stat-card { padding: var(--ll-space-2) var(--ll-space-3); display: flex; flex-direction: column; gap: 2px; }
.stat-label { font-size: var(--ll-fs-xs); color: var(--ll-color-text-tertiary); }
.stat-value { font-size: var(--ll-fs-2xl); font-weight: var(--ll-fw-semibold); line-height: 1.2; }
.stat-sub { font-size: 10px; color: var(--ll-color-text-tertiary); }
.tone-plain { color: var(--ll-color-text); }
.tone-success { color: var(--ll-color-success); }
.tone-danger { color: var(--ll-color-danger); }
.tone-warning { color: var(--ll-color-warning); }

/* 筛选行 */
.filter-bar {
  display: flex; align-items: center; gap: var(--ll-space-2);
  padding: var(--ll-space-2) var(--ll-space-3);
  border: 1px solid var(--ll-color-border);
  border-radius: var(--ll-radius-md);
  background: var(--ll-color-surface-2);
}
.fl-ico { color: var(--ll-color-text-tertiary); flex: none; }
.filter-group { display: inline-flex; gap: 2px; }
.filter-group button {
  height: 22px; padding: 0 var(--ll-space-2);
  border: none; border-radius: var(--ll-radius-xs);
  background: transparent; color: var(--ll-color-text-secondary);
  font-size: var(--ll-fs-xs);
}
.filter-group button:hover { background: var(--ll-color-surface-3); }
.filter-group button.active { background: var(--ll-color-brand); color: var(--ll-color-text-inverse); font-weight: var(--ll-fw-medium); }
/* 原型状态组默认「全部状态」为浅底，不用实心蓝 */
.status-group button.active { background: var(--ll-color-brand-light); color: var(--ll-color-brand); }
.fl-right { margin-left: auto; font-size: var(--ll-fs-xs); white-space: nowrap; }

/* 记录表 */
.table-card { overflow: hidden; }
/* fixed 布局：列宽由表头一次定死，剩余空间全部给「对端设备」列。
   auto 布局下窄列会被按比例压到内容宽度以下，导致「2 个 / 866 MB / 26 s」折行。 */
.history-table { table-layout: fixed; }
.history-table th, .history-table td { padding: 10px var(--ll-space-2); }
/* 列宽按最小窗口（1040px → 中央区约 500px）实测内容反推，
   保证「2 个」「866 MB」「26 s」「E009」都能单行放下 */
.history-table th:nth-child(1) { width: 94px; }
.history-table th:nth-child(2) { width: 50px; }
.history-table th:nth-child(3) { width: auto; }   /* 唯一弹性列，内容过长由 .ll-truncate 省略 */
.history-table th:nth-child(4) { width: 42px; }
.history-table th:nth-child(5) { width: 58px; }
.history-table th:nth-child(6) { width: 44px; }
.history-table th:nth-child(7) { width: 54px; }
.history-table th:nth-child(8) { width: 40px; }
/* 窄列一律不换行（表头与数字列）；「对端设备」列内部靠 .ll-truncate 截断 */
.history-table th { white-space: nowrap; }
.history-table td:nth-child(4),
.history-table td:nth-child(5),
.history-table td:nth-child(6),
.history-table td:nth-child(7),
.history-table td:nth-child(8) { white-space: nowrap; }
.cell-time { font-size: var(--ll-fs-xs); white-space: nowrap; }
.dir-tag, .status-tag {
  display: inline-flex; align-items: center; justify-content: center;
  height: 18px; padding: 0 6px; border-radius: var(--ll-radius-xs);
  font-size: 10px; white-space: nowrap;
}
.dir-tag.send { background: var(--ll-color-brand-light); color: var(--ll-color-brand); }
.dir-tag.recv { background: var(--ll-color-info-surface); color: var(--ll-color-brand); }
.status-tag.tone-success { background: var(--ll-color-success-light); color: var(--ll-color-success); }
.status-tag.tone-warning { background: var(--ll-color-warning-light); color: var(--ll-color-warning); }
.status-tag.tone-danger { background: var(--ll-color-danger-light); color: var(--ll-color-danger); }
.peer { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
.peer-name { color: var(--ll-color-text); font-size: var(--ll-fs-xs); }
.peer-link { font-size: 10px; }
.row-open {
  width: 22px; height: 20px; border: none; background: transparent;
  color: var(--ll-color-brand); border-radius: var(--ll-radius-xs);
  display: inline-flex; align-items: center; justify-content: center;
}
.row-open:hover { background: var(--ll-color-brand-light); }
.empty-row { padding: 0; }

/* 表底分页：不画分割线，用浅灰底与数据行区分 */
.table-foot {
  display: flex; align-items: center; justify-content: space-between;
  padding: var(--ll-space-2) var(--ll-space-3);
  background: var(--ll-color-surface-2);
  font-size: var(--ll-fs-xs);
}
.pager { display: inline-flex; align-items: center; gap: 3px; }
.page-arrow, .page-num {
  width: 22px; height: 20px; border: 1px solid var(--ll-color-border);
  background: var(--ll-color-surface); border-radius: var(--ll-radius-xs);
  color: var(--ll-color-text-secondary); font-size: 10px;
  display: inline-flex; align-items: center; justify-content: center;
}
.page-arrow:disabled { opacity: 0.45; }
.page-num.active { background: var(--ll-color-brand); border-color: var(--ll-color-brand); color: var(--ll-color-text-inverse); }

/* 底部提示条 */
.hint-bar {
  display: flex; align-items: center; gap: var(--ll-space-2);
  padding: var(--ll-space-3);
  border: 1px solid var(--ll-color-border);
  border-radius: var(--ll-radius-md);
  background: var(--ll-color-surface-2);
  color: var(--ll-color-text-tertiary); font-size: var(--ll-fs-xs);
}

/* F27：诊断详情浮层 */
.diag-mask {
  position: fixed; inset: 0; z-index: 60;
  display: flex; align-items: center; justify-content: center;
  background: rgba(0,0,0,.32);
}
.diag-panel {
  width: 420px; max-width: 90%;
  padding: var(--ll-space-4);
  background: var(--ll-color-surface);
  border: 1px solid var(--ll-color-border);
  border-radius: var(--ll-radius-md);
  box-shadow: 0 12px 40px rgba(0,0,0,.2);
}
.diag-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--ll-space-3); }
.diag-title { display: inline-flex; align-items: center; gap: var(--ll-space-2); font-weight: var(--ll-fw-semibold); font-size: var(--ll-fs-md); }
.diag-body { border: 1px solid var(--ll-color-border); border-radius: var(--ll-radius-sm); overflow: hidden; }
.diag-line { display: flex; justify-content: space-between; gap: var(--ll-space-3); padding: 7px var(--ll-space-3); border-bottom: 1px solid var(--ll-color-divider); font-size: var(--ll-fs-xs); }
.diag-line:last-child { border-bottom: none; }
.diag-k { color: var(--ll-color-text-tertiary); }
.diag-v { color: var(--ll-color-text); }
.diag-note { margin-top: var(--ll-space-3); font-size: 10px; line-height: var(--ll-lh-relaxed); }
</style>
