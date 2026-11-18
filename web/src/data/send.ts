// =====================================================================
// 「发送文件」主窗口的文案与空态常量
// ---------------------------------------------------------------------
// 所有列表/统计初值均为空：真机由 Tauri 采集与传输事件填充，
// 浏览器预览态保持空态（不看假数据，只看布局）。
// 文案逐字对齐定版设计稿；改文案时同步此文件。
// =====================================================================

export interface LockedFile {
  name: string
  size: string
  time: string
  ext: string
  tone: 'pdf' | 'video' | 'zip'
}

export interface LockedDevice {
  id: string
  name: string
  platform: string
  addr: string
  latency: string
  action: string
  primary: boolean
  warn?: boolean
  icon: string
}

export const SEND_LOCKED = {
  /** 中央区标题行 */
  title: '发送文件',
  subtitle: '先选文件，再选目标：同网段直连，跨网用序列号',
  clearLabel: '清空列表',

  /** 拖放区 */
  dropTitle: '将文件或文件夹拖放到此处',
  dropSub: '或点击左上角「选择文件」 · 支持断点续传与 100 GB 以上大文件',

  /** 待发送文件表：初值为空，用户选择文件后由真实队列替换 */
  files: [] as LockedFile[],
  fileFoot: '未选择文件',

  /** 选择目标设备卡 */
  target: {
    title: '选择目标设备',
    scan: '等待扫描',
    hint: '同网段 · 正在发现设备',
    devices: [] as LockedDevice[],
    crossTitle: '跨网 / 异地设备 · 需用凭证',
    crossDesc: '序列号自包含地址，经 IM 搬运；一个凭证仅可握手一次，有效期 15 分钟',
    generateLabel: '生成序列号',
    pasteLabel: '粘贴序列号连接',
  },

  /** 底部提示条 */
  hintTitle: '当前无进行中的传输',
  hintDesc: '选择目标设备后开始传输；进度、速率与剩余时间会在此实时显示',
  hintLink: '查看传输记录',

  /** 右栏：当前会话 */
  sessionRows: [
    { k: '连接状态', v: '未连接' },
    { k: '数据通道', v: '未建立' },
    { k: '待发送', v: '0 个文件' },
    { k: '通道加密', v: '关闭' },
  ],
  sessionButton: '连接到远程设备',

  /** 右栏：最近连接：初值为空 */
  recent: [] as { name: string; desc: string; tone: 'ok' | 'warn' }[],

  /** 右栏：凭证安全（定版 3 条） */
  security: [
    '序列号仅存于内存，关闭弹窗即销毁',
    '握手成功即作废，不可重复使用',
    '同网段无需序列号，避免多余暴露',
  ],

  /** 左轨本机信息卡：真机由 get_self_info 填充，未就绪显示占位符 */
  selfOs: '—',
  selfLan4: '—',
  selfPub6: '—',
  selfStun: '—',
  sendBadge: 0,

  /** 命令栏与状态栏 */
  openSaveDirLabel: '打开保存目录',
  rescanLabel: '重新扫描设备',
  statusText: '就绪 · 未选择文件',
}
