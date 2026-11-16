// 视觉风格（Visual Styles / Base Styles）元数据
//
// 参考来源：
//   1) Reasonix Studio「视觉风格」设置页（6 套卡片：Graphite / Aurora / Slate /
//      Carbon / Nocturne / Amber），名称、标签、描述、强调色逐字/逐像素对齐；
//   2) scpuny/frontend/packages/theme-ironlink/src/palettes.ts —— 6 套基础风格的
//      light/dark 中性色板（bg / bgSoft / bgElev / border / fg），CSS 落地见
//      styles/palettes.css。
//
// 设计：明暗模式（data-theme）与视觉风格（data-palette）相互独立；
// 语义色（成功/警示/危险/信息）为 RESERVED，不随风格变化。

export type PaletteId =
  | 'prototype'
  | 'graphite'
  | 'aurora'
  | 'slate'
  | 'carbon'
  | 'nocturne'
  | 'amber'

export interface PaletteMeta {
  id: PaletteId
  /** 卡片标题：英文 + 中文 */
  name: string
  /** 菜单里的短名 */
  short: string
  /** 卡片上的气质标签 */
  tag: string
  /** 卡片描述（逐字来自参考设计） */
  desc: string
  /** 预览色块：[最深底色, 次级底色, 强调色] */
  swatch: [string, string, string]
  /** 渐变强调色的第二色（Aurora 专用） */
  accent2?: string
  /** 是否为基础原型基准（不在 6 卡网格里展示） */
  baseline?: boolean
  origin: string
}

export const DEFAULT_PALETTE: PaletteId = 'prototype'

export const PALETTES: PaletteMeta[] = [
  {
    id: 'prototype',
    name: '原型基准',
    short: '原型基准',
    tag: '定版',
    desc: '与 Calicat 定版原型逐像素对齐的 Arco 蓝色板',
    swatch: ['#eef0f3', '#f7f8fa', '#165dff'],
    baseline: true,
    origin: 'Calicat 原型',
  },
  {
    id: 'graphite',
    name: 'Graphite 石墨',
    short: '石墨',
    tag: '利落',
    desc: '纸面白配石墨文字与橙色强调，利落、克制，贴近编辑器工作台。',
    swatch: ['#0C0D10', '#15161A', '#FF7B45'],
    origin: 'baseStyle: graphite',
  },
  {
    id: 'aurora',
    name: 'Aurora 柔雾极光',
    short: '柔雾极光',
    tag: '温润',
    desc: '柔紫底色融合极光蓝绿，半透明面板与弹性圆角，更轻盈、有呼吸感。',
    swatch: ['#0E0D18', '#1D1B34', '#9E7BFF'],
    accent2: '#8A99F7',
    origin: 'baseStyle: aurora',
  },
  {
    id: 'slate',
    name: 'Slate 精炼',
    short: '精炼',
    tag: '原生',
    desc: '冷灰工作台配品牌蓝，发丝边框清晰，适合高密度扫描与专业操作。',
    swatch: ['#0D0F12', '#1B1F25', '#478AF6'],
    origin: 'baseStyle: slate',
  },
  {
    id: 'carbon',
    name: 'Carbon 深邃',
    short: '深邃',
    tag: '高级',
    desc: '暖炭黑与米灰表面配青绿强调，质感更厚、对比更足，适合长时间专注。',
    swatch: ['#0E0D0C', '#1E1C1A', '#28D4D1'],
    origin: 'baseStyle: carbon',
  },
  {
    id: 'nocturne',
    name: 'Nocturne 柔和',
    short: '柔和',
    tag: '呼吸',
    desc: '柔紫夜色和云白表面配大圆角留白，阅读更安静，节奏更舒缓。',
    swatch: ['#101019', '#20212F', '#8A8BF9'],
    origin: 'baseStyle: nocturne',
  },
  {
    id: 'amber',
    name: 'Amber 琥珀',
    short: '琥珀',
    tag: '暖阳',
    desc: '暖橙强调色，明亮亲和（含深色变体）。',
    swatch: ['#0F0C08', '#201A12', '#D76B38'],
    origin: 'baseStyle: amber',
  },
]

/** 6 张卡片（不含原型基准） */
export const VISUAL_STYLES = PALETTES.filter((p) => !p.baseline)

export function isPaletteId(v: string | null): v is PaletteId {
  return !!v && PALETTES.some((p) => p.id === v)
}
