// =====================================================================
// 平台识别（macOS / Windows / Linux）
// ---------------------------------------------------------------------
// 自定义窗体需要按平台走不同布局：
//  - macOS：保留系统「交通灯」按钮（titleBarStyle=Overlay），标题栏左侧留白；
//  - Windows / Linux：decorations=false，标题栏右侧自绘 最小化 / 最大化 / 关闭。
//
// 判断依据：Tauri 的 WKWebView / WebView2 都带标准 UA，浏览器预览态同样可用，
// 因此统一走 UA + userAgentData.platform，避免依赖 IPC 时序（init() 是异步的）。
// =====================================================================

export type Platform = 'macos' | 'windows' | 'linux' | 'unknown'

export function detectPlatform(): Platform {
  if (typeof navigator === 'undefined') return 'unknown'
  const ua = navigator.userAgent
  // 移动端不作为桌面窗体处理
  if (/iPhone|iPad|iPod|Android/i.test(ua)) return 'unknown'

  const nav = navigator as Navigator & { userAgentData?: { platform?: string } }
  const p = (nav.userAgentData?.platform || navigator.platform || '').toLowerCase()

  if (p.includes('mac') || /Mac OS X|Macintosh/i.test(ua)) return 'macos'
  if (p.includes('win') || /Windows/i.test(ua)) return 'windows'
  if (p.includes('linux') || p.includes('x11') || /Linux/i.test(ua)) return 'linux'
  return 'unknown'
}

export const PLATFORM: Platform = detectPlatform()
export const IS_MAC = PLATFORM === 'macos'
