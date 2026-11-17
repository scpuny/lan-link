// =====================================================================
// LanLink 桥接层（前端 ↔ 后端）
// ---------------------------------------------------------------------
// 通信方式：Tauri `invoke` 桥接（进程内 Rust 调用），**不使用 HTTP**。
//  - Tauri 运行时（真机 / cargo tauri dev 加载 Vite dev server）：
//    直接调用 @tauri-apps/api/core 的 invoke → Rust #[tauri::command]。
//  - 浏览器预览态（npm run dev，无 Tauri 全局对象）：
//    回退到 bridge/mock.ts 的内存实现，便于「先看前端效果」。
//  - 真机里想强制看 mock 效果：URL 加 ?bridge=mock（用于 A/B 对比）。
//
// 上层 UI 只依赖本文件的 invoke()，无需感知底层是 Tauri 还是 Mock。
// =====================================================================
import { invoke as tauriInvoke } from '@tauri-apps/api/core'
import * as mock from './mock'
import type {
  DecodedInvite,
  GenerateResult,
  NearbyDevice,
  SelfInfo,
  SessionInfo,
} from '../types/invite'

export type InvokeArgs = Record<string, unknown>

/** 是否处于 Tauri 运行时（Tauri v2 注入 __TAURI_INTERNALS__） */
export function hasTauri(): boolean {
  if (typeof window === 'undefined') return false
  return '__TAURI_INTERNALS__' in window || '__TAURI__' in window
}

/** ?bridge=mock 可在真机里强制走内存 mock，便于对比真/Rust 输出 */
function forceMock(): boolean {
  if (typeof window === 'undefined') return false
  return new URLSearchParams(window.location.search).get('bridge') === 'mock'
}

/** 统一桥接入口：路由到 Tauri 或 Mock */
export async function invoke<T = unknown>(cmd: string, args?: InvokeArgs): Promise<T> {
  if (hasTauri() && !forceMock()) {
    // Tauri 态：进程内 IPC，直连 Rust command；command 不存在会抛错，不再静默降级
    return (await tauriInvoke<T>(cmd, args)) as T
  }
  // ---- Mock 分支（浏览器预览，或 ?bridge=mock） ----
  switch (cmd) {
    case 'generate_serial':
      return (await mock.mockGenerateSerial((args as any) || {})) as T
    case 'decode_serial':
      return (await mock.mockDecodeSerial((args as any) || {})) as T
    case 'connect':
      return (await mock.mockConnect((args as any) || {})) as T
    case 'generate_lan_code':
      return (await mock.mockGenerateLanCode()) as T
    case 'get_lan_code':
      return (await mock.mockGetLanCode()) as T
    case 'connect_lan_code':
      return (await mock.mockConnectLanCode((args as any) || {})) as T
    case 'scan_lan':
      return (await mock.mockScanLan()) as T
    case 'get_self_info':
      return (await mock.mockGetSelf()) as T
    case 'open_save_dir':
      return (await mock.mockOpenSaveDir()) as T
    case 'pick_files':
      return (await mock.mockPickFiles()) as T
    case 'pick_folder':
      return (await mock.mockPickFolder()) as T
    case 'read_clipboard':
      return (await mock.mockReadClipboard()) as T
    case 'get_save_dir':
      return (await mock.mockGetSaveDir()) as T
    case 'choose_save_dir':
      return (await mock.mockChooseSaveDir()) as T
    case 'set_discoverable':
      return (await mock.mockSetDiscoverable(!!(args as any)?.on)) as T
    case 'get_discoverable':
      return (await mock.mockGetDiscoverable()) as T
    case 'get_blacklist_count':
      return (await mock.mockGetBlacklistCount()) as T
    case 'clear_blacklist':
      return (await mock.mockClearBlacklist()) as T
    case 'cancel_handshake':
      return (await mock.mockCancelHandshake()) as T
    case 'probe_serial':
      return (await mock.mockProbeSerial()) as T
    case 'transfer_pause':
    case 'transfer_resume':
    case 'transfer_cancel':
    case 'transfer_pause_all':
      return (await mock.mockTransferCtl()) as T
    default:
      throw new Error(`[mock] unknown command: ${cmd}`)
  }
}

/** 当前桥接模式，便于 UI 角标提示 */
export function bridgeMode(): 'tauri' | 'mock' {
  return hasTauri() && !forceMock() ? 'tauri' : 'mock'
}

// 显式重导出常用返回类型，方便组件直接 import
export type {
  DecodedInvite,
  GenerateResult,
  NearbyDevice,
  SelfInfo,
  SessionInfo,
}
