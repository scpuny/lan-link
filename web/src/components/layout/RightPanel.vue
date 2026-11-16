<script setup lang="ts">
import { computed } from 'vue'
import Icon from '../common/Icon.vue'
import { useUi } from '../../composables/useUi'
import {
  DISCOVER_FIELDS, RECEIVE_PREFS,
  HISTORY_OVERVIEW, HISTORY_FAILURES,
} from '../../data/views'
import { SEND_LOCKED as L } from '../../data/send'

const { state, openDialog } = useUi()

const transferring = computed(() => state.centerState === 'transferring')
const withLan = computed(() => state.centerState === 'idle_with_lan')

const legacySecurity = [
  '序列号仅存于内存，关闭弹窗即销毁',
  '首次连接即可用，握手成功即作废',
  '高敏感文件建议勾选口令保护再生成',
]

const recent = computed(() =>
  withLan.value
    ? L.recent
    : [
        { name: '李工的 MacBook Pro', desc: '11-14 10:24 · IPv6 直连 · 已结束', tone: 'ok' },
        { name: 'OFFICE-PC-07', desc: '11-13 17:02 · 无法连接候选地址 E009', tone: 'warn' },
      ]
)

function togglePref(key: 'askBeforeReceive' | 'autoRename') {
  state[key] = !state[key]
}
</script>

<template>
  <aside class="panel">
    <!-- ==================== 发送视图右栏（原三态） ==================== -->
    <template v-if="state.mainView === 'send'">
      <div class="ll-card panel-card session-card" :class="{ 'session-card--live': transferring }">
        <div class="card-head">
          <span class="card-title"><Icon name="link" :size="14" class="head-ico" /> 当前会话</span>
          <span class="ll-dot" :class="transferring ? 'll-dot--success' : ''" />
        </div>
        <div class="ll-kv"><span class="ll-kv__k">连接状态</span><span class="ll-kv__v">{{ transferring ? '已建立' : '未连接' }}</span></div>
        <template v-if="transferring">
          <div class="ll-kv"><span class="ll-kv__k">连接时长</span><span class="ll-kv__v">00:01:42</span></div>
          <div class="ll-kv"><span class="ll-kv__k">累计传输</span><span class="ll-kv__v">535.3 MB</span></div>
          <div class="ll-kv"><span class="ll-kv__k">通道加密</span><span class="ll-kv__v">关闭（明文二进制 + CRC32）</span></div>
        </template>
        <template v-else-if="withLan">
          <div v-for="r in L.sessionRows.slice(1)" :key="r.k" class="ll-kv">
            <span class="ll-kv__k">{{ r.k }}</span><span class="ll-kv__v">{{ r.v }}</span>
          </div>
        </template>
        <template v-else>
          <div class="ll-kv"><span class="ll-kv__k">数据通道</span><span class="ll-kv__v">未建立</span></div>
          <div class="ll-kv"><span class="ll-kv__k">对端地址</span><span class="ll-kv__v">—</span></div>
          <div class="ll-kv"><span class="ll-kv__k">通道加密</span><span class="ll-kv__v">关闭（明文二进制 + CRC32）</span></div>
        </template>
        <button
          class="ll-btn ll-btn--block session-btn"
          :class="transferring ? 'll-btn--danger-outline' : 'll-btn--primary'"
          @click="transferring ? (state.centerState = 'idle_with_lan') : openDialog('connect')"
        >
          <Icon :name="transferring ? 'close' : 'link'" :size="13" />
          {{ transferring ? '断开连接' : L.sessionButton }}
        </button>
      </div>

      <template v-if="transferring">
        <div class="ll-card panel-card">
          <div class="card-head">
            <span class="card-title"><Icon name="activity" :size="14" class="head-ico" /> 传输层校验</span>
          </div>
          <div class="ll-kv"><span class="ll-kv__k">期望 seq</span><span class="ll-kv__v ll-mono">1343</span></div>
          <div class="ll-kv"><span class="ll-kv__k">重复丢弃</span><span class="ll-kv__v ll-mono">0</span></div>
          <div class="ll-kv"><span class="ll-kv__k">重传次数</span><span class="ll-kv__v ll-mono">2</span></div>
        </div>
        <div class="ll-card panel-card">
          <div class="card-head">
            <span class="card-title"><Icon name="shield" :size="14" class="head-ico head-ico--green" /> 凭证状态</span>
          </div>
          <ul class="quiet-list">
            <li>本次序列号已作废，不可再次握手</li>
            <li>断开连接后黑名单保留至进程退出</li>
            <li>需要更高保密性请下次勾选口令保护</li>
          </ul>
        </div>
      </template>

      <div v-if="!transferring" class="ll-card panel-card security-card">
        <div class="card-head">
          <span class="card-title"><Icon name="shield" :size="14" class="head-ico head-ico--green" /> 凭证安全</span>
        </div>
        <ul class="quiet-list">
          <li v-for="s in (withLan ? L.security : legacySecurity)" :key="s">{{ s }}</li>
        </ul>
      </div>

      <div v-if="!transferring" class="ll-card panel-card recent-card">
        <div class="card-head">
          <span class="card-title">最近连接</span>
          <span class="ll-link">全部</span>
        </div>
        <div v-for="r in recent" :key="r.name" class="recent-row">
          <span class="recent-ico" :class="r.tone === 'warn' ? 'warn' : ''">
            <Icon :name="r.tone === 'warn' ? 'alertTriangle' : 'laptop'" :size="15" />
          </span>
          <div class="ll-col recent-meta">
            <span class="recent-name ll-truncate">{{ r.name }}</span>
            <span class="ll-text-tertiary recent-desc" :class="{ warn: r.tone === 'warn' }">{{ r.desc }}</span>
          </div>
        </div>
      </div>
    </template>

    <!-- ==================== 接收视图右栏 ==================== -->
    <template v-else-if="state.mainView === 'receive'">
      <div class="ll-card panel-card discover-card">
        <div class="card-head">
          <span class="card-title"><Icon name="wifi" :size="14" class="head-ico--green" /> 本机可被发现</span>
          <span class="ll-dot ll-dot--success" />
        </div>
        <div v-for="f in DISCOVER_FIELDS" :key="f.k" class="ll-kv">
          <span class="ll-kv__k">{{ f.k }}</span>
          <span class="ll-kv__v" :class="{ 'tone-success': f.tone === 'success' }">{{ f.v }}</span>
        </div>
        <button class="ll-btn ll-btn--default ll-btn--block session-btn">
          <Icon name="copy" :size="13" /> 复制本机邀请信息
        </button>
      </div>

      <div class="ll-card panel-card">
        <div class="card-head">
          <span class="card-title"><Icon name="sliders" :size="14" class="head-ico" /> 接收偏好</span>
        </div>
        <div v-for="p in RECEIVE_PREFS" :key="p.key" class="pref-row">
          <span>{{ p.label }}</span>
          <button
            class="ll-switch"
            :data-on="state[p.key]"
            role="switch"
            :aria-checked="state[p.key]"
            @click="togglePref(p.key)"
          />
        </div>
      </div>

      <div class="ll-card panel-card">
        <div class="card-head">
          <span class="card-title"><Icon name="shield" :size="14" class="head-ico--green" /> 接收安全</span>
        </div>
        <ul class="quiet-list">
          <li>接收前确认可避免陌生设备直投</li>
          <li>跨网接收需生成序列号并告知对方</li>
        </ul>
      </div>
    </template>

    <!-- ==================== 传输记录视图右栏 ==================== -->
    <template v-else>
      <div class="ll-card panel-card">
        <div class="card-head">
          <span class="card-title"><Icon name="clock" :size="14" class="head-ico" /> 近 7 天</span>
          <span class="overview-range ll-text-tertiary">{{ HISTORY_OVERVIEW.range }}</span>
        </div>
        <div v-for="b in HISTORY_OVERVIEW.bars" :key="b.label" class="ov-row">
          <span class="ov-label">{{ b.label }}</span>
          <span class="ov-track"><i :class="`fill-${b.tone}`" :style="{ width: (b.value / b.max * 100) + '%' }" /></span>
          <span class="ov-count">{{ b.value }} 次</span>
        </div>
      </div>

      <div class="ll-card panel-card">
        <div class="card-head">
          <span class="card-title"><Icon name="alertCircle" :size="14" class="head-ico--orange" /> 失败原因</span>
        </div>
        <div v-for="f in HISTORY_FAILURES" :key="f.code" class="fail-row">
          <span class="fail-code">{{ f.code }}</span>
          <span class="fail-desc">{{ f.desc }}</span>
          <span class="fail-count">{{ f.count }}</span>
        </div>
      </div>

      <div class="ll-card panel-card">
        <div class="card-head">
          <span class="card-title"><Icon name="lock" :size="14" class="head-ico--green" /> 记录隐私</span>
        </div>
        <ul class="quiet-list">
          <li>仅存 session 前 8 位，不存完整序列号</li>
          <li>不记录候选 IP 详情，仅保留连接类型</li>
        </ul>
      </div>
    </template>
  </aside>
</template>

<style scoped>
.panel {
  width: var(--ll-panel-w);
  flex: none;
  background: var(--ll-color-bg-elevated);
  border-left: 1px solid var(--ll-color-border);
  padding: var(--ll-space-3);
  display: flex;
  flex-direction: column;
  gap: var(--ll-space-3);
  overflow-y: auto;
}
.panel > * { flex: none; }
.panel-card { padding: var(--ll-space-3); }
/* 发送视图原型顺序：当前会话 → 最近连接 → 凭证安全 */
.recent-card { order: 1; }
.security-card { order: 2; }
.card-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--ll-space-2); }
.card-title { display: inline-flex; align-items: center; gap: var(--ll-space-2); font-weight: var(--ll-fw-semibold); font-size: var(--ll-fs-md); }
.head-ico { color: var(--ll-color-brand); }
.head-ico--green { color: var(--ll-color-success); }
.head-ico--orange { color: var(--ll-color-warning); }
.ll-kv { font-size: var(--ll-fs-xs); }
.ll-kv__k { width: 72px; }
/* 定版右栏「当前会话」是普通白卡，仅传输中为绿描边 */
.session-card { border-color: var(--ll-color-border); }
.session-card--live { border-color: var(--ll-color-success); }
.session-btn { margin-top: var(--ll-space-3); }
.tone-success { color: var(--ll-color-success); }
.discover-card { border-color: var(--ll-color-success-border); }

.quiet-list {
  margin: 0;
  padding: 0 0 0 var(--ll-space-4);
  font-size: var(--ll-fs-xs);
  color: var(--ll-color-text-secondary);
  line-height: 1.9;
}
.quiet-list li::marker { color: var(--ll-color-text-quaternary); }

.quick-row {
  width: 100%;
  display: flex; align-items: center; gap: var(--ll-space-2);
  padding: var(--ll-space-2) 0;
  border: none; border-top: 1px solid var(--ll-color-divider);
  background: transparent; font-size: var(--ll-fs-sm);
  color: var(--ll-color-text-secondary); text-align: left;
}
.quick-row:first-of-type { border-top: none; }
.quick-row > span:nth-child(2) { flex: 1; }
.quick-row .ll-mono { font-size: var(--ll-fs-xs); }
.quick-row:hover { color: var(--ll-color-brand); }
.qr-ico { color: var(--ll-color-brand); }

.recent-row { display: flex; align-items: flex-start; gap: var(--ll-space-2); padding: var(--ll-space-2) 0; border-top: 1px solid var(--ll-color-divider); }
.recent-row:first-of-type { border-top: none; }
.recent-ico { color: var(--ll-color-brand); display: inline-flex; flex: none; margin-top: 2px; }
.recent-ico.warn { color: var(--ll-color-warning); }
.recent-meta { min-width: 0; }
.recent-name { font-size: var(--ll-fs-sm); font-weight: var(--ll-fw-medium); }
.recent-desc { font-size: 10px; }
.recent-desc.warn { color: var(--ll-color-warning); }

/* 接收偏好开关行 */
.pref-row {
  display: flex; align-items: center; justify-content: space-between;
  padding: var(--ll-space-2) 0; border-top: 1px solid var(--ll-color-divider);
  font-size: var(--ll-fs-sm); color: var(--ll-color-text-secondary);
}
.pref-row:first-of-type { border-top: none; }
.pref-row .ll-switch { cursor: pointer; }

/* 记录视图：概览进度条 */
.overview-range { font-size: var(--ll-fs-xs); }
.ov-row {
  display: flex; align-items: center; gap: var(--ll-space-2);
  padding: var(--ll-space-1) 0; font-size: var(--ll-fs-xs);
}
.ov-label { width: 30px; flex: none; color: var(--ll-color-text-secondary); }
.ov-track {
  flex: 1; height: 6px; border-radius: var(--ll-radius-full);
  background: var(--ll-color-surface-3); overflow: hidden;
}
.ov-track > i { display: block; height: 100%; border-radius: var(--ll-radius-full); }
.fill-brand { background: var(--ll-color-brand); }
.fill-success { background: var(--ll-color-success); }
.fill-danger { background: var(--ll-color-danger); }
.ov-count { width: 30px; flex: none; text-align: right; color: var(--ll-color-text-secondary); }

/* 记录视图：失败分布 */
.fail-row {
  display: flex; align-items: center; gap: var(--ll-space-2);
  padding: var(--ll-space-2) 0; border-top: 1px solid var(--ll-color-divider);
  font-size: var(--ll-fs-xs);
}
.fail-row:first-of-type { border-top: none; }
.fail-code {
  padding: 1px 5px; border-radius: var(--ll-radius-xs);
  background: var(--ll-color-danger-light); color: var(--ll-color-danger);
  font-size: 10px; flex: none;
}
.fail-desc { flex: 1; min-width: 0; color: var(--ll-color-text-secondary); }
.fail-count { color: var(--ll-color-text-tertiary); }
</style>
