<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import Modal from '../common/Modal.vue'
import Icon from '../common/Icon.vue'
import { useTheme, type ThemePref } from '../../composables/useTheme'
import { useUi } from '../../composables/useUi'
import { VISUAL_STYLES, DEFAULT_PALETTE, type PaletteMeta } from '../../data/themes'

const { state, chooseSaveDir, clearBlacklist, showToast, openSaveDir } = useUi()
const { pref, setTheme, palette, setPalette } = useTheme()
const emit = defineEmits<{ (e: 'close'): void }>()

const checkingUpdate = ref(false)
async function checkUpdate() {
  if (checkingUpdate.value) return
  checkingUpdate.value = true
  // TODO-F33：接真实更新检查（需打包签名 + 发布渠道）；当前延迟模拟
  await new Promise((r) => setTimeout(r, 700))
  checkingUpdate.value = false
  showToast('已是最新版本 · v0.9.4')
}

type TabKey = 'general' | 'appearance' | 'transfer' | 'security' | 'about'
const tab = ref<TabKey>('general')
const nav: { key: TabKey; label: string; icon: string }[] = [
  { key: 'general', label: '通用', icon: 'settings' },
  { key: 'appearance', label: '外观', icon: 'eye' },
  { key: 'transfer', label: '传输', icon: 'sliders' },
  { key: 'security', label: '安全与协议', icon: 'shield' },
  { key: 'about', label: '关于', icon: 'info' },
]
const themeOpts: { label: string; v: ThemePref }[] = [
  { label: '跟随系统', v: 'system' }, { label: '浅色', v: 'light' }, { label: '深色', v: 'dark' },
]

// 通用
const autoStart = ref(true)
const minToTray = ref(true)
const lang = ref('简体中文')
const openFolder = ref(false)
const desktopNotify = ref(true)
const soundNotify = ref(false)
const dlDir = ref('D:\\LanLink\\Downloads')

// 传输
const concurrency = ref(2)
const resume = ref(true)
const limitMode = ref<'off' | 'custom'>('custom')
const limitVal = ref(20)
const blakeVerify = ref(true)
const autoFirewall = ref(true)

// 安全
const ttl = ref<900 | 300>(900)
const defaultLan = ref(false)
const defaultPwd = ref(false)

const footerText = computed(() => {
  switch (tab.value) {
    case 'general': return '通用设置不影响安全与协议参数'
    case 'appearance': return '视觉风格只改变中性色与强调色，状态色保持固定'
    case 'transfer': return '更改监听端口后需重启监听服务'
    case 'security': return '配置只作用于本进程，不写入注册表以外的持久存储'
    default: return '日志仅记录 session 前 8 位 + ts + error_code'
  }
})
function close() { emit('close') }

// Aurora 的强调色是极光渐变，其余风格为纯色块
function swatchStyle(s: PaletteMeta) {
  return s.accent2
    ? { background: `linear-gradient(135deg, ${s.swatch[2]} 0%, ${s.accent2} 100%)` }
    : { background: s.swatch[2] }
}
function styleName(s: PaletteMeta) {
  const i = s.name.indexOf(' ')
  return { en: i < 0 ? s.name : s.name.slice(0, i), zh: i < 0 ? '' : s.name.slice(i + 1) }
}

// F29：除外观外的设置持久化（外观由 useTheme 单独写 localStorage）
const SETTINGS_KEY = 'll-settings-v1'
onMounted(() => {
  try {
    const s = JSON.parse(localStorage.getItem(SETTINGS_KEY) || '{}')
    if (typeof s.autoStart === 'boolean') autoStart.value = s.autoStart
    if (typeof s.minToTray === 'boolean') minToTray.value = s.minToTray
    if (s.lang) lang.value = s.lang
    if (typeof s.openFolder === 'boolean') openFolder.value = s.openFolder
    if (typeof s.desktopNotify === 'boolean') desktopNotify.value = s.desktopNotify
    if (typeof s.soundNotify === 'boolean') soundNotify.value = s.soundNotify
    if (s.dlDir) dlDir.value = s.dlDir
    if (s.concurrency) concurrency.value = s.concurrency
    if (typeof s.resume === 'boolean') resume.value = s.resume
    if (s.limitMode) limitMode.value = s.limitMode
    if (s.limitVal) limitVal.value = s.limitVal
    if (typeof s.blakeVerify === 'boolean') blakeVerify.value = s.blakeVerify
    if (typeof s.autoFirewall === 'boolean') autoFirewall.value = s.autoFirewall
    if (s.ttl) ttl.value = s.ttl
    if (typeof s.defaultLan === 'boolean') defaultLan.value = s.defaultLan
    if (typeof s.defaultPwd === 'boolean') defaultPwd.value = s.defaultPwd
  } catch { /* 忽略损坏的配置 */ }
})
/** F32：复制脱敏诊断信息（不含完整序列号 / IP） */
async function copyDiagnostics() {
  const s = state.self
  const text = [
    'LanLink 诊断信息',
    `版本: v0.9.4 (20261114) · 协议 v1`,
    `系统: ${s?.osUser || '未知'}`,
    `网络能力: IPv6=${s?.netCapabilities.ipv6Global ? '是' : '否'} STUN=${s?.netCapabilities.stun ? '是' : '否'} mDNS=${s?.netCapabilities.mdns ? '是' : '否'}`,
    `黑名单: ${state.blacklistCount} 条 · 日志红线: session前8位 + ts + error_code`,
  ].join('\n')
  try {
    await navigator.clipboard.writeText(text)
    showToast('诊断信息已复制（已脱敏）')
  } catch {
    showToast('复制失败：剪贴板不可用')
  }
}

function saveSettings() {
  localStorage.setItem(SETTINGS_KEY, JSON.stringify({
    autoStart: autoStart.value, minToTray: minToTray.value, lang: lang.value,
    openFolder: openFolder.value, desktopNotify: desktopNotify.value, soundNotify: soundNotify.value,
    dlDir: dlDir.value, concurrency: concurrency.value, resume: resume.value,
    limitMode: limitMode.value, limitVal: limitVal.value, blakeVerify: blakeVerify.value,
    autoFirewall: autoFirewall.value, ttl: ttl.value, defaultLan: defaultLan.value,
    defaultPwd: defaultPwd.value,
  }))
}
</script>

<template>
  <Modal :title="tab === 'about' ? '关于 LanLink' : '设置'" :icon="tab === 'about' ? 'info' : 'settings'" :width="720" @close="close">
    <template #header-extra>
      <button v-if="tab === 'about'" class="ll-btn ll-btn--default ll-btn--sm set-head-btn" :disabled="checkingUpdate" @click="checkUpdate">
        <span v-if="checkingUpdate" class="ll-spinner ll-spinner--xs" />
        <Icon v-else name="update" :size="12" /> {{ checkingUpdate ? '检查中…' : '检查更新' }}
      </button>
    </template>

    <div class="set-layout">
      <!-- 左侧导航 -->
      <nav class="set-nav">
        <button v-for="n in nav" :key="n.key" :class="{ active: tab === n.key }" @click="tab = n.key">
          <Icon :name="n.icon" :size="14" /> {{ n.label }}
        </button>
      </nav>

      <!-- 右侧内容 -->
      <div class="set-content">
        <!-- ===== 通用 ===== -->
        <template v-if="tab === 'general'">
          <section class="ll-card set-group">
            <h4 class="set-gtitle"><Icon name="settings" :size="13" /> 启动</h4>
            <div class="set-row">
              <div class="set-label"><span>开机自动启动</span><small class="ll-text-tertiary">随 Windows 登录启动 LanLink</small></div>
              <button class="ll-switch" :data-on="autoStart" @click="autoStart = !autoStart" />
            </div>
            <div class="set-row">
              <div class="set-label"><span>启动时最小化到托盘</span><small class="ll-text-tertiary">不弹主窗口，仅保留托盘图标与监听</small></div>
              <button class="ll-switch" :data-on="minToTray" @click="minToTray = !minToTray" />
            </div>
            <div class="set-row">
              <div class="set-label"><span>界面语言</span></div>
              <select v-model="lang" class="set-select"><option>简体中文</option><option>English</option></select>
            </div>
          </section>

          <section class="ll-card set-group">
            <h4 class="set-gtitle"><Icon name="folder" :size="13" /> 文件与通知</h4>
            <div class="set-row">
              <div class="set-label"><span>接收文件保存目录</span><small class="ll-mono ll-text-tertiary">{{ state.saveDir || dlDir }}</small></div>
              <button class="ll-btn ll-btn--default ll-btn--sm" @click="chooseSaveDir">更改</button>
            </div>
            <div class="set-row"><div class="set-label"><span>传输完成后打开所在文件夹</span></div><button class="ll-switch" :data-on="openFolder" @click="openFolder = !openFolder" /></div>
            <div class="set-row"><div class="set-label"><span>收到连接请求时弹桌面通知</span></div><button class="ll-switch" :data-on="desktopNotify" @click="desktopNotify = !desktopNotify" /></div>
            <div class="set-row"><div class="set-label"><span>传输完成声音提示</span></div><button class="ll-switch" :data-on="soundNotify" @click="soundNotify = !soundNotify" /></div>
          </section>
        </template>

        <!-- ===== 外观：主题 + 视觉风格 ===== -->
        <template v-else-if="tab === 'appearance'">
          <section class="ll-card set-group">
            <h4 class="set-gtitle"><Icon name="eye" :size="13" /> 主题</h4>
            <div class="set-row">
              <div class="set-label"><span>明暗模式</span><small class="ll-text-tertiary">可跟随系统，或与视觉风格自由组合</small></div>
              <div class="set-seg">
                <button v-for="o in themeOpts" :key="o.v" :data-active="pref === o.v" @click="setTheme(o.v)">{{ o.label }}</button>
              </div>
            </div>
          </section>

          <section class="ll-card set-group">
            <div class="vs-head">
              <span class="vs-heading"><Icon name="eye" :size="13" /> 视觉风格</span>
              <button
                class="vs-reset" :data-active="palette === DEFAULT_PALETTE"
                title="回到与 Calicat 定版原型逐像素一致的色板"
                @click="setPalette(DEFAULT_PALETTE)"
              >
                <Icon name="refresh" :size="11" /> 原型基准
              </button>
            </div>
            <div class="vs-grid">
              <button
                v-for="s in VISUAL_STYLES" :key="s.id"
                class="vs-card" :data-active="palette === s.id"
                :title="`${s.name}（${s.origin}）`"
                @click="setPalette(s.id)"
              >
                <span v-if="palette === s.id" class="vs-check"><Icon name="check" :size="11" :width="2.4" /></span>
                <span class="vs-title"><b>{{ styleName(s).en }}</b> {{ styleName(s).zh }}</span>
                <span class="vs-tag">{{ s.tag }}</span>
                <span class="vs-swatches">
                  <i :style="{ background: s.swatch[0] }"></i>
                  <i :style="{ background: s.swatch[1] }"></i>
                  <i :style="swatchStyle(s)"></i>
                </span>
                <span class="vs-desc">{{ s.desc }}</span>
              </button>
            </div>
            <p class="vs-note ll-text-tertiary">明暗模式与视觉风格相互独立；成功 / 警示 / 危险 / 信息等状态色为保留色，不随风格改变。</p>
          </section>
        </template>

        <!-- ===== 传输 ===== -->
        <template v-else-if="tab === 'transfer'">
          <section class="ll-card set-group">
            <h4 class="set-gtitle"><Icon name="sliders" :size="13" /> 传输调度</h4>
            <div class="set-row"><div class="set-label"><span>分片大小</span></div><b>4 MB <small class="ll-text-tertiary">（协议固定，仅调度可调）</small></b></div>
            <div class="set-row">
              <div class="set-label"><span>并发传输任务数</span></div>
              <div class="set-seg"><button v-for="n in [1,2,3,4]" :key="n" :data-active="concurrency === n" @click="concurrency = n">{{ n }}</button></div>
            </div>
            <p class="set-group-note ll-text-tertiary">分片与 seq 规则由协议固定，此处仅调整调度参数</p>
          </section>

          <section class="ll-card set-group">
            <h4 class="set-gtitle"><Icon name="activity" :size="13" /> 速度与控制</h4>
            <div class="set-row"><div class="set-label"><span>启用断点续传</span></div><button class="ll-switch" :data-on="resume" @click="resume = !resume" /></div>
            <div class="set-row">
              <div class="set-label"><span>限速模式</span></div>
              <div class="set-seg"><button :data-active="limitMode === 'off'" @click="limitMode = 'off'">不限速</button><button :data-active="limitMode === 'custom'" @click="limitMode = 'custom'">自定义限速</button></div>
            </div>
            <div v-if="limitMode === 'custom'" class="set-row"><div class="set-label"><span>限速值</span></div><div class="set-num"><input v-model="limitVal" class="ll-input" /><span class="unit">MB/s</span></div></div>
            <div class="set-row"><div class="set-label"><span>传输完成后 Blake3 校验文件</span></div><button class="ll-switch" :data-on="blakeVerify" @click="blakeVerify = !blakeVerify" /></div>
            <p class="set-group-note ll-text-tertiary">校验失败的分片会自动重传，不覆盖已确认的分片</p>
          </section>

          <section class="ll-card set-group">
            <h4 class="set-gtitle"><Icon name="network" :size="13" /> 网络与端口</h4>
            <div class="set-row"><div class="set-label"><span>监听端口</span></div><b class="ll-mono">42210</b></div>
            <div class="set-row"><div class="set-label"><span>自动添加防火墙入站规则</span></div><button class="ll-switch" :data-on="autoFirewall" @click="autoFirewall = !autoFirewall" /></div>
          </section>
        </template>

        <!-- ===== 安全与协议 ===== -->
        <template v-else-if="tab === 'security'">
          <section class="ll-card set-group">
            <h4 class="set-gtitle"><Icon name="link" :size="13" /> 外网序列号</h4>
            <div class="set-row">
              <div class="set-label"><span>外网序列号有效期 TTL</span></div>
              <div class="set-seg"><button :data-active="ttl === 900" @click="ttl = 900">900s（15分钟）</button><button :data-active="ttl === 300" @click="ttl = 300">300s（5分钟·高安全）</button></div>
            </div>
            <p class="set-group-note ll-text-tertiary">过期凭证无法握手，已建立的通道不受影响</p>
            <div class="set-row"><div class="set-label"><span>生成时默认包含内网地址</span></div><button class="ll-switch" :data-on="defaultLan" @click="defaultLan = !defaultLan" /></div>
            <div class="set-row"><div class="set-label"><span>生成时默认勾选口令保护</span></div><button class="ll-switch" :data-on="defaultPwd" @click="defaultPwd = !defaultPwd" /></div>
            <div class="set-fixed">口令算法固定 PBKDF2-HMAC-SHA256(10000) + AES-GCM-256，不可更改</div>
          </section>

          <section class="ll-card set-group">
            <h4 class="set-gtitle"><Icon name="shield" :size="13" /> 会话与日志</h4>
            <div class="set-row"><div class="set-label"><span>会话黑名单</span><small class="ll-text-tertiary">当前 {{ state.blacklistCount }} 条 · 仅内存，退出清空</small></div><button class="ll-btn ll-btn--default ll-btn--sm" @click="clearBlacklist">清空</button></div>
            <div class="set-info"><Icon name="info" :size="13" /><span>仅记录 session 前 8 位 + ts + error_code；严禁打印完整序列号与 IP；口令 / 流密钥用完即 zeroize。此项不可关闭。</span></div>
            <div class="set-row"><div class="set-label"><span>协议</span></div><b class="ll-mono">v1</b></div>
            <p class="set-group-note ll-text-tertiary">flags bit2–bit7 未启用（中继 / 传输加密 / 权限 / 版本号）</p>
          </section>
        </template>

        <!-- ===== 关于 ===== -->
        <template v-else>
          <section class="ll-card set-version">
            <span class="sv-logo"><Icon name="link" :size="20" :width="2.2" /></span>
            <div class="ll-col">
              <b>LanLink v0.9.4</b>
              <span class="ll-text-tertiary">build 20261114 · 协议 v1</span>
              <span class="ll-tag ll-tag--success"><Icon name="check" :size="11" /> 已是最新</span>
            </div>
            <button class="ll-btn ll-btn--default ll-btn--sm" :disabled="checkingUpdate" @click="checkUpdate">
              <span v-if="checkingUpdate" class="ll-spinner ll-spinner--xs" />
              <Icon v-else name="update" :size="12" /> {{ checkingUpdate ? '检查中…' : '检查更新' }}
            </button>
          </section>

          <section class="ll-card set-group">
            <h4 class="set-gtitle"><Icon name="monitor" :size="13" /> 运行环境</h4>
            <div class="set-row"><div class="set-label"><span>操作系统</span></div><b>Windows 11 · 当前用户 chen</b></div>
            <div class="set-row"><div class="set-label"><span>网络能力</span></div><b class="ll-text-secondary">IPv6 可用 · STUN 已获取 · IPv4 疑似 CGNAT</b></div>
            <div class="set-row"><div class="set-label"><span>监听端口</span></div><b class="ll-mono">TCP 42210</b></div>
            <div class="set-row"><div class="set-label"><span>诊断与日志</span></div><div class="ll-row ll-gap-2"><button class="ll-btn ll-btn--default ll-btn--sm" @click="copyDiagnostics">复制诊断信息</button><button class="ll-btn ll-btn--default ll-btn--sm" @click="openSaveDir">打开日志目录</button></div></div>
          </section>

          <section class="ll-card set-group">
            <h4 class="set-gtitle"><Icon name="package" :size="13" /> 协议依赖（MIT / Apache-2.0）</h4>
            <div class="dep-list">
              <div><b class="ll-mono">crc32fast</b><span class="ll-text-tertiary">完整性 · 明文模式校验包体</span></div>
              <div><b class="ll-mono">lz4_flex</b><span class="ll-text-tertiary">自适应压缩 · 仅在变小时采用</span></div>
              <div><b class="ll-mono">aes-gcm</b><span class="ll-text-tertiary">可选口令加密 + Tag 完整性</span></div>
              <div><b class="ll-mono">pbkdf2</b><span class="ll-text-tertiary">口令派生 10000 次 · HMAC-SHA256</span></div>
              <div><b class="ll-mono">blake3</b><span class="ll-text-tertiary">握手摘要 + 整文件校验</span></div>
            </div>
          </section>

          <div class="set-info"><Icon name="shield" :size="13" /><span>隐私声明：无服务器架构：不收集设备指纹、不上传任何地址或文件</span></div>
        </template>
      </div>
    </div>

    <template #footer>
      <div class="set-foot">
        <span class="ll-text-tertiary"><Icon name="info" :size="11" /> {{ footerText }}</span>
        <div class="ll-row ll-gap-2">
          <button v-if="tab !== 'about'" class="ll-btn ll-btn--default" @click="close">取消</button>
          <button class="ll-btn ll-btn--primary" @click="tab !== 'about' && saveSettings(); close()">{{ tab === 'about' ? '关闭' : '保存设置' }}</button>
        </div>
      </div>
    </template>
  </Modal>
</template>

<style scoped>
/* 固定尺寸：宽 720（Modal）+ 内容区高 440，切换选项卡不再改变窗体大小 */
.set-layout { display: flex; gap: var(--ll-space-4); height: 440px; min-height: 0; }
/* 与标题栏关闭按钮同高，避免「关于」页因 header-extra 把窗体撑高 2px */
.set-head-btn { height: 24px; }
.set-nav { width: 132px; flex: none; display: flex; flex-direction: column; gap: var(--ll-space-1); }
.set-nav button {
  display: flex; align-items: center; gap: var(--ll-space-2);
  height: 32px; padding: 0 var(--ll-space-3);
  border: none; background: transparent; border-radius: var(--ll-radius-sm);
  color: var(--ll-color-nav-text); font-size: var(--ll-fs-sm); font-weight: var(--ll-fw-nav); text-align: left;
}
.set-nav button:hover { background: var(--ll-color-surface-hover); }
.set-nav button.active { background: var(--ll-color-surface-active); color: var(--ll-color-brand); font-weight: var(--ll-fw-medium); }
.set-content { flex: 1; min-width: 0; height: 440px; max-height: none; overflow-y: auto; display: flex; flex-direction: column; gap: var(--ll-space-2); padding-right: var(--ll-space-1); }

.set-group { padding: var(--ll-space-2) var(--ll-space-3); }
.set-gtitle { display: flex; align-items: center; gap: var(--ll-space-2); font-size: var(--ll-fs-sm); font-weight: var(--ll-fw-semibold); color: var(--ll-color-text-secondary); margin-bottom: var(--ll-space-2); }
.set-gtitle svg { color: var(--ll-color-text-tertiary); }
.set-row { display: flex; align-items: center; justify-content: space-between; gap: var(--ll-space-3); padding: 5px 0; border-top: 1px solid var(--ll-color-divider); font-size: var(--ll-fs-sm); }
.set-row:first-of-type { border-top: none; }
.set-label { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.set-label small { font-size: 10px; }
.set-row > b { font-weight: var(--ll-fw-medium); font-size: var(--ll-fs-xs); white-space: nowrap; }
.set-row > b small { font-weight: var(--ll-fw-regular); }
.set-group-note { font-size: 10px; margin-top: var(--ll-space-2); }
.set-seg { display: inline-flex; gap: var(--ll-space-1); flex: none; }
.set-seg button { height: 26px; padding: 0 var(--ll-space-2); font-size: var(--ll-fs-xs); border: 1px solid var(--ll-color-border-strong); border-radius: var(--ll-radius-sm); background: var(--ll-color-surface); color: var(--ll-color-text-secondary); }
.set-seg button[data-active="true"] { border-color: var(--ll-color-brand); color: var(--ll-color-brand); background: var(--ll-color-brand-light); font-weight: var(--ll-fw-medium); }

/* ---- 视觉风格（Base Styles）3×2 卡片网格，对齐 Reasonix Studio 设置页 ---- */
.vs-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: var(--ll-space-3); }
.vs-heading { display: inline-flex; align-items: center; gap: var(--ll-space-2); font-size: var(--ll-fs-sm); font-weight: var(--ll-fw-semibold); color: var(--ll-color-text); }
.vs-heading svg { color: var(--ll-color-text-tertiary); }
.vs-reset {
  display: inline-flex; align-items: center; gap: 4px;
  height: 24px; padding: 0 9px; border: 1px solid var(--ll-color-border-strong);
  border-radius: var(--ll-radius-sm); background: var(--ll-color-surface);
  color: var(--ll-color-text-tertiary); font-size: var(--ll-fs-xs);
}
.vs-reset:hover { border-color: var(--ll-color-brand); color: var(--ll-color-brand); }
.vs-reset[data-active="true"] { border-color: var(--ll-color-brand); color: var(--ll-color-brand); background: var(--ll-color-brand-light); }
.vs-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: var(--ll-space-2); }
.vs-card {
  position: relative; display: flex; flex-direction: column; align-items: flex-start;
  min-height: 130px; padding: 9px 11px 8px;
  border: 1px solid var(--ll-color-border); border-radius: var(--ll-radius-md);
  background: var(--ll-color-surface-2); color: var(--ll-color-text);
  text-align: left; transition: border-color .15s, box-shadow .15s, transform .15s;
}
.vs-card:hover { border-color: var(--ll-color-brand); transform: translateY(-1px); }
.vs-card[data-active="true"] {
  border-color: var(--ll-color-brand);
  box-shadow: 0 0 0 1px var(--ll-color-brand), 0 4px 14px var(--ll-color-brand-bg);
}
.vs-check {
  position: absolute; top: 10px; right: 10px;
  width: 18px; height: 18px; border-radius: 50%;
  display: inline-flex; align-items: center; justify-content: center;
  background: var(--ll-color-brand); color: var(--ll-color-text-inverse);
}
.vs-title { font-size: var(--ll-fs-sm); line-height: 1.3; padding-right: 22px; }
.vs-title b { font-weight: var(--ll-fw-semibold); }
.vs-tag {
  margin-top: 5px; padding: 1px 8px; border-radius: 999px;
  border: 1px solid var(--ll-color-border-strong);
  background: var(--ll-color-surface-3);
  color: var(--ll-color-text-secondary); font-size: 10px; line-height: 1.5;
}
.vs-swatches { display: flex; gap: 6px; margin: 8px 0 7px; }
.vs-swatches i { width: 28px; height: 24px; border-radius: 6px; border: 1px solid rgba(255,255,255,.14); flex: none; }
.vs-desc { font-size: 10px; line-height: 1.5; color: var(--ll-color-text-secondary); }
.vs-note { margin-top: var(--ll-space-2); font-size: 10px; line-height: var(--ll-lh-relaxed); }
.set-select { height: 28px; min-width: 130px; border: 1px solid var(--ll-color-border-strong); border-radius: var(--ll-radius-sm); background: var(--ll-color-surface); color: var(--ll-color-text); padding: 0 8px; font-size: var(--ll-fs-xs); }
.set-num { display: flex; align-items: center; gap: var(--ll-space-2); }
.set-num .ll-input { width: 80px; height: 28px; }
.unit { font-size: var(--ll-fs-xs); color: var(--ll-color-text-tertiary); }
.set-fixed, .set-info { margin-top: var(--ll-space-2); padding: var(--ll-space-2) var(--ll-space-3); font-size: var(--ll-fs-xs); line-height: var(--ll-lh-relaxed); border-radius: var(--ll-radius-sm); }
.set-fixed { background: var(--ll-color-surface-3); color: var(--ll-color-text-tertiary); }
.set-info { display: flex; align-items: flex-start; gap: var(--ll-space-2); background: var(--ll-color-info-surface); border: 1px solid var(--ll-color-info-border); color: var(--ll-color-text-secondary); }
.set-info svg { flex: none; color: var(--ll-color-brand); margin-top: 1px; }

.set-version { display: flex; align-items: center; gap: var(--ll-space-3); padding: var(--ll-space-3); }
.sv-logo { width: 36px; height: 36px; flex: none; display: inline-flex; align-items: center; justify-content: center; border-radius: var(--ll-radius-md); background: var(--ll-color-brand); color: var(--ll-color-text-inverse); }
.set-version .ll-col { flex: 1; gap: 2px; }
.set-version b { font-size: var(--ll-fs-md); }
.set-version .ll-text-tertiary { font-size: var(--ll-fs-xs); }
.set-version .ll-tag { align-self: flex-start; }
.dep-list { display: flex; flex-direction: column; }
.dep-list > div { display: flex; align-items: baseline; gap: var(--ll-space-3); padding: var(--ll-space-2) 0; border-top: 1px solid var(--ll-color-divider); }
.dep-list > div:first-child { border-top: none; }
.dep-list b { width: 88px; flex: none; font-size: var(--ll-fs-xs); color: var(--ll-color-brand); }
.dep-list span { font-size: var(--ll-fs-xs); }

.set-foot { display: flex; align-items: center; justify-content: space-between; gap: var(--ll-space-3); width: 100%; }
.set-foot > span { display: inline-flex; align-items: center; gap: var(--ll-space-1); font-size: var(--ll-fs-xs); }
.ll-spinner--xs { width: 11px; height: 11px; border-width: 1.5px; }
</style>
