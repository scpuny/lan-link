<script setup lang="ts">
import { computed } from 'vue'
import Icon from '../common/Icon.vue'
import { useUi } from '../../composables/useUi'
import { HISTORY_POLICY } from '../../data/views'
import { SEND_LOCKED as L } from '../../data/send'

const { state, openDialog, navigate, receiveBadge, historyBadge } = useUi()

interface Cap { label: string; value: string; tone: 'ok' | 'warn' | 'danger' | 'muted'; icon: string }

/** 发送页定版（main-ready-locked.png）：本机卡无小标签、发送项角标 2 */
const locked = computed(() => state.mainView === 'send' && state.centerState === 'idle_with_lan')

/** 四项导航：角标随激活视图变化（原型：激活项蓝色徽标，其余灰色） */
const nav = computed(() => [
  { key: 'send', label: '发送文件', icon: 'upload', badge: locked.value ? L.sendBadge : 0 },
  {
    key: 'receive', label: '接收文件', icon: 'download',
    badge: locked.value || state.mainView === 'history' ? 0 : receiveBadge,
  },
  { key: 'history', label: '传输记录', icon: 'history', badge: historyBadge },
  { key: 'settings', label: '设置', icon: 'settings', badge: 0 },
])

function onNav(key: string) {
  if (key === 'settings') openDialog('settings')
  else navigate(key as 'send' | 'receive' | 'history')
}

/** 发送视图底部卡：随三态切换内容 */
const capTitle = computed(() => {
  if (state.centerState === 'transferring') return '当前通道能力'
  if (state.centerState === 'idle_with_lan') return '发现能力'
  return '网络能力检测'
})
const caps = computed<Cap[]>(() => {
  if (state.centerState === 'transferring') {
    return [
      { label: 'P2P 直连', value: '已建立', tone: 'ok', icon: 'checkCircle' },
      { label: '通道加密', value: '关闭', tone: 'danger', icon: 'alertCircle' },
      { label: '分片 seq 校验', value: '单调递增', tone: 'ok', icon: 'checkCircle' },
      { label: '会话黑名单', value: '1 条', tone: 'muted', icon: 'list' },
    ]
  }
  if (state.centerState === 'idle_with_lan') {
    return [
      { label: 'mDNS 广播', value: '运行中', tone: 'ok', icon: 'checkCircle' },
      { label: '子网探测', value: '42210', tone: 'muted', icon: 'network' },
      { label: '跨网序列号', value: '可用', tone: 'ok', icon: 'checkCircle' },
    ]
  }
  const s = state.self?.netCapabilities
  return [
    { label: 'mDNS 广播', value: s?.mdns ? '运行中' : '未启动', tone: s?.mdns ? 'ok' : 'warn', icon: 'checkCircle' },
    { label: 'STUN 端点', value: s?.stun ? '已获取' : '未获取', tone: s?.stun ? 'ok' : 'warn', icon: 'checkCircle' },
    { label: 'IPv6 全球单播', value: s?.ipv6Global ? '可用' : '不可用', tone: s?.ipv6Global ? 'ok' : 'warn', icon: 'checkCircle' },
    { label: 'IPv4 直连性', value: s?.ipv4Direct ? '可用' : '疑似 CGNAT', tone: s?.ipv4Direct ? 'ok' : 'warn', icon: 'alertTriangle' },
  ]
})

/** 接收视图底部卡：可接收状态 */
const receiveCaps: Cap[] = [
  { label: 'mDNS 可发现', value: '开启', tone: 'ok', icon: 'checkCircle' },
  { label: '防火墙放行', value: '已添加', tone: 'ok', icon: 'checkCircle' },
  { label: '剩余空间', value: '218 GB', tone: 'muted', icon: 'hardDrive' },
]
</script>

<template>
  <aside class="rail">
    <!-- 本机信息卡（三视图共用） -->
    <div class="ll-card self-card">
      <div class="self-head">
        <span class="self-avatar"><Icon name="monitor" :size="16" /></span>
        <div class="ll-col self-meta">
          <span class="self-name ll-truncate">{{ state.self?.deviceName || 'DESKTOP-LL-01' }}</span>
          <span class="self-os ll-text-tertiary ll-truncate">{{ locked ? L.selfOs : (state.self?.osUser || 'Windows 11 · 当前用户 chen') }}</span>
        </div>
      </div>
      <div class="self-addr">
        <div class="addr-row">
          <span class="addr-k">Lan4</span>
          <span class="addr-v ll-mono ll-truncate">{{ locked ? L.selfLan4 : (state.self?.lan4 || '192.168.1.24') }}</span>
          <span v-if="!locked" class="addr-tag" title="未打包进凭证">—</span>
        </div>
        <div class="addr-row">
          <span class="addr-k">Pub6</span>
          <span class="addr-v ll-mono ll-truncate">{{ locked ? L.selfPub6 : (state.self?.pub6 || '2408:8207:3d4f:…:9c1d') }}</span>
          <span v-if="!locked" class="addr-tag ok">默认</span>
        </div>
        <div class="addr-row">
          <span class="addr-k">STUN</span>
          <span class="addr-v ll-mono ll-truncate">{{ locked ? L.selfStun : (state.self?.stun || '121.36.80.24:51820') }}</span>
          <span v-if="!locked" class="addr-tag">默认</span>
        </div>
      </div>
    </div>

    <!-- 主视图导航 -->
    <nav class="rail-nav">
      <button
        v-for="n in nav"
        :key="n.key"
        class="nav-item"
        :class="{ active: n.key !== 'settings' && state.mainView === n.key }"
        @click="onNav(n.key)"
      >
        <Icon :name="n.icon" :size="15" />
        <span class="nav-label">{{ n.label }}</span>
        <span
          v-if="n.badge"
          class="ll-badge"
          :class="{ 'll-badge--plain': state.mainView !== n.key }"
        >{{ n.badge }}</span>
      </button>
    </nav>

    <!-- 发送视图：网络能力 / 发现能力 / 当前通道能力 -->
    <!-- 三视图的卡片一律贴左轨底部（margin-top:auto），切换视图时不跳动 -->
    <div v-if="state.mainView === 'send'" class="ll-card cap-card">
      <div class="cap-title">{{ capTitle }}</div>
      <div v-for="c in caps" :key="c.label" class="cap-row">
        <span class="cap-icon" :class="`tone-${c.tone}`"><Icon :name="c.icon" :size="12" /></span>
        <span class="cap-label">{{ c.label }}</span>
        <span class="cap-state" :class="`tone-${c.tone}`">{{ c.value }}</span>
      </div>
    </div>

    <!-- 接收视图：可接收状态 -->
    <div v-else-if="state.mainView === 'receive'" class="ll-card cap-card">
      <div class="cap-title">可接收状态</div>
      <div v-for="c in receiveCaps" :key="c.label" class="cap-row">
        <span class="cap-icon" :class="`tone-${c.tone}`"><Icon :name="c.icon" :size="12" /></span>
        <span class="cap-label">{{ c.label }}</span>
        <span class="cap-state" :class="`tone-${c.tone}`">{{ c.value }}</span>
      </div>
    </div>

    <!-- 记录视图：记录策略 -->
    <div v-else class="ll-card cap-card">
      <div class="cap-title">记录策略</div>
      <div v-for="p in HISTORY_POLICY" :key="p.k" class="cap-row">
        <span class="cap-icon tone-muted"><Icon name="list" :size="12" /></span>
        <span class="cap-label">{{ p.k }}</span>
        <span v-if="p.v" class="cap-state tone-muted">{{ p.v }}</span>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.rail {
  width: var(--ll-rail-w);
  flex: none;
  background: var(--ll-color-bg-elevated);
  border-right: 1px solid var(--ll-color-border);
  display: flex;
  flex-direction: column;
  gap: var(--ll-space-3);
  padding: var(--ll-space-3);
  overflow-y: auto;
}
.self-card { padding: var(--ll-space-3); }
.self-head { display: flex; align-items: center; gap: var(--ll-space-2); }
.self-avatar {
  width: 28px;
  height: 28px;
  border-radius: var(--ll-radius-sm);
  background: var(--ll-color-brand-light);
  color: var(--ll-color-brand);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
}
.self-meta { min-width: 0; }
.self-name { font-weight: var(--ll-fw-semibold); font-size: var(--ll-fs-md); }
.self-os { font-size: var(--ll-fs-xs); }
.self-addr {
  margin-top: var(--ll-space-3);
  padding-top: var(--ll-space-2);
  border-top: 1px solid var(--ll-color-divider);
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.addr-row { display: flex; align-items: center; gap: var(--ll-space-1); font-size: var(--ll-fs-xs); }
.addr-k { color: var(--ll-color-text-tertiary); width: 32px; flex: none; }
.addr-v { color: var(--ll-color-text-secondary); flex: 1; }
.addr-tag { flex: none; font-size: 10px; color: var(--ll-color-text-quaternary); }
.addr-tag.ok { color: var(--ll-color-success); }

.rail-nav { display: flex; flex-direction: column; gap: 2px; }
.nav-item {
  display: flex;
  align-items: center;
  gap: var(--ll-space-3);
  height: 32px;
  padding: 0 var(--ll-space-3);
  border: none;
  background: transparent;
  border-radius: var(--ll-radius-sm);
  color: var(--ll-color-nav-text);
  font-size: var(--ll-fs-md);
  font-weight: var(--ll-fw-nav);
  text-align: left;
  transition: all var(--ll-duration-fast);
}
.nav-item:hover { background: var(--ll-color-surface-hover); color: var(--ll-color-text); }
.nav-item.active {
  background: var(--ll-color-surface-active);
  color: var(--ll-color-brand);
  font-weight: var(--ll-fw-medium);
}
.nav-label { flex: 1; }

/* 底部能力卡：三视图统一贴底，不随导航高度变化 */
.cap-card { padding: var(--ll-space-3); margin-top: auto; }
.cap-title {
  font-size: var(--ll-fs-xs);
  color: var(--ll-color-text-tertiary);
  margin-bottom: var(--ll-space-2);
}
.cap-row { display: flex; align-items: center; gap: var(--ll-space-1); padding: 4px 0; font-size: var(--ll-fs-xs); }
.cap-icon { display: inline-flex; flex: none; }
.cap-label { flex: 1; color: var(--ll-color-text-secondary); white-space: nowrap; }
.cap-state { font-size: 10px; white-space: nowrap; }
.tone-ok { color: var(--ll-color-success); }
.tone-warn { color: var(--ll-color-warning); }
.tone-danger { color: var(--ll-color-danger); }
.tone-muted { color: var(--ll-color-text-tertiary); }
</style>
