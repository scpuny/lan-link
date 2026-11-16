// 主题组合式：两个相互独立的维度（参考 Reasonix Studio 主题状态模型）
//   1. 明暗 pref    → <html data-theme="light|dark">，可为 system
//   2. 视觉风格 palette → <html data-palette="graphite|aurora|slate|carbon|nocturne|amber">
//      prototype 为原型基准（不写属性，直接用 tokens.css 默认值）
// 只切换语义色变量，不触发组件重渲染；两者各自持久化。
import { ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { DEFAULT_PALETTE, isPaletteId, type PaletteId } from '../data/themes'

export type ThemePref = 'light' | 'dark' | 'system'

const STORAGE_KEY = 'll-theme'
const PALETTE_KEY = 'll-palette'

function systemTheme(): 'light' | 'dark' {
  if (typeof window === 'undefined' || !window.matchMedia) return 'light'
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

function isTauri(): boolean {
  if (typeof window === 'undefined') return false
  return '__TAURI_INTERNALS__' in window || '__TAURI__' in window
}

/**
 * 同步 macOS 原生窗口外观。
 *
 * NSWindow 的边框/阴影取 effectiveAppearance：当系统处于深色、而应用被手动切成浅色时，
 * 窗口边缘会保留 1px 深色描边（用户反馈的「主窗体黑色边框」，实测边缘像素 rgb(1,0,3)）。
 * 把窗口主题设成与应用一致即可消除。浏览器预览态没有原生窗口，直接跳过。
 */
function syncWindowTheme(resolved: 'light' | 'dark') {
  if (!isTauri()) return
  try {
    getCurrentWindow()
      .setTheme(resolved)
      .catch(() => {
        /* 缺少 core:window:allow-set-theme 权限时忽略，不影响页面主题 */
      })
  } catch {
    /* 非 Tauri 运行时 */
  }
}

function applyToDom(pref: ThemePref) {
  const resolved = pref === 'system' ? systemTheme() : pref
  document.documentElement.setAttribute('data-theme', resolved)
  syncWindowTheme(resolved)
}

function applyPalette(id: PaletteId) {
  const root = document.documentElement
  // 原型基准色板无需属性：tokens.css 的 :root / [data-theme] 已定义
  if (id === DEFAULT_PALETTE) root.removeAttribute('data-palette')
  else root.setAttribute('data-palette', id)
}

/** 浏览器预览态可用 ?palette=xxx 直达某个视觉风格（Tauri 真机 URL 无此参数） */
function initialPalette(): PaletteId {
  const fromUrl = new URLSearchParams(window.location.search).get('palette')
  if (isPaletteId(fromUrl)) return fromUrl
  const saved = localStorage.getItem(PALETTE_KEY)
  if (isPaletteId(saved)) return saved
  // 早期版本的 studio / clear-blue / tea-green 已废弃，清掉残留避免反复回退
  if (saved) localStorage.removeItem(PALETTE_KEY)
  return DEFAULT_PALETTE
}

/** 浏览器预览态可用 ?theme=light|dark|system 直达某个明暗（Tauri 真机 URL 一般不带） */
function initialTheme(): ThemePref {
  const fromUrl = new URLSearchParams(window.location.search).get('theme')
  if (fromUrl === 'light' || fromUrl === 'dark' || fromUrl === 'system') return fromUrl
  const saved = localStorage.getItem(STORAGE_KEY)
  if (saved === 'light' || saved === 'dark' || saved === 'system') return saved
  return 'system'
}

const pref = ref<ThemePref>(initialTheme())
const palette = ref<PaletteId>(initialPalette())

// 初始化应用
applyToDom(pref.value)
applyPalette(palette.value)

// 跟随系统切换
if (typeof window !== 'undefined' && window.matchMedia) {
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    if (pref.value === 'system') applyToDom('system')
  })
}

export function useTheme() {
  function setTheme(p: ThemePref) {
    pref.value = p
    localStorage.setItem(STORAGE_KEY, p)
    applyToDom(p)
  }

  function setPalette(id: PaletteId) {
    palette.value = id
    localStorage.setItem(PALETTE_KEY, id)
    applyPalette(id)
  }

  function resetPalette() { setPalette(DEFAULT_PALETTE) }

  return { pref, setTheme, systemTheme, applyToDom, palette, setPalette, resetPalette }
}
