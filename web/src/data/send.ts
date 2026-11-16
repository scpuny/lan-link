// =====================================================================
// 「发送文件」定版主窗口的原型数据
// ---------------------------------------------------------------------
// 来源：docs/prototypes/main-ready-locked.png
//       Calicat 页面「LanLink 主窗口 · 就绪态（锁定版）」
// 用户确认此版为发送页定版（2026-11-15），默认发送视图按此渲染。
// 文案逐字对齐原型；改文案时同步此文件，避免运行态数据覆盖设计稿。
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

  /** 待发送文件表（定版为 2 行） */
  files: [
    { name: 'LanLink-设计规范-v1.2.pdf', size: '4.2 MB', time: '11-14 10:22', ext: 'PDF', tone: 'pdf' },
    { name: '会议录屏_1114_外网序列号评审.mp4', size: '862 MB', time: '11-14 09:58', ext: 'MP4', tone: 'video' },
  ] as LockedFile[],
  fileFoot: '已选 2 个文件，共 866.2 MB · 剩余空间 218 GB',

  /** 选择目标设备卡 */
  target: {
    title: '选择目标设备',
    scan: '最近扫描 2s 前',
    hint: '同网段 · mDNS 自动发现 3 台',
    devices: [
      { id: 'd1', name: '李工的 MacBook Pro', platform: 'macOS · 可接收', addr: '192.168.1.36', latency: '2 ms', action: '连接', primary: true, icon: 'laptop' },
      { id: 'd2', name: 'OFFICE-PC-07', platform: 'Windows 11 · 可接收', addr: '192.168.1.58', latency: '11 ms', action: '发送', primary: false, icon: 'monitor' },
      { id: 'd3', name: 'chen 的 Android', platform: 'Android · 需对方确认', addr: '192.168.1.72', latency: '24 ms', action: '发送', primary: false, warn: true, icon: 'phone' },
    ] as LockedDevice[],
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
    { k: '待发送', v: '2 个文件 · 866.2 MB' },
    { k: '通道加密', v: '关闭' },
  ],
  sessionButton: '连接到远程设备',

  /** 右栏：最近连接 */
  recent: [
    { name: '李工的 MacBook Pro', desc: '11-14 10:24 · 同网段直连', tone: 'ok' },
    { name: 'OFFICE-PC-07', desc: '11-13 17:02 · 序列号 E009', tone: 'warn' },
  ],

  /** 右栏：凭证安全（定版 3 条） */
  security: [
    '序列号仅存于内存，关闭弹窗即销毁',
    '握手成功即作废，不可重复使用',
    '同网段无需序列号，避免多余暴露',
  ],

  /** 左轨本机信息卡（定版无「默认 / 未打包」小标签） */
  selfOs: 'Windows 11 · chen',
  selfLan4: '192.168.1.24',
  selfPub6: '2408:8207:…9c1d',
  selfStun: '121.36.80.24',
  sendBadge: 2,

  /** 命令栏与状态栏 */
  /** 定版就绪态左上角第二按钮（用户要求由「接收文件」改为「打开保存目录」） */
  openSaveDirLabel: '打开保存目录',
  rescanLabel: '重新扫描设备',
  onlineText: 'mDNS 广播中 · 已发现 3 台',
  statusText: '就绪 · 待发送 2 个文件 · 已发现 3 台设备',
}
