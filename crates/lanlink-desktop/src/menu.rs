//! macOS 原生菜单栏（NSMenu）
//!
//! 平台差异（这是本模块存在的唯一理由）：
//!  - macOS：菜单属于「应用」，固定渲染在**屏幕顶部**的系统菜单栏，窗口内不放菜单；
//!  - Windows / Linux：没有全局菜单栏，菜单画在窗口标题栏里（`web/…/AppMenu.vue`）。
//!
//! 因此 macOS 由 Rust 侧构建原生菜单，点击后通过 `ll-menu` 事件把动作回传前端，
//! 前端复用与窗口内菜单**完全相同**的 `onAction(key, value)` 分发逻辑，
//! 两个平台的业务行为保持一致，只是承载位置不同。
//!
//! 菜单项 id 约定：`key` 或 `key|value`（value 用于「外观」「视觉风格」二级菜单）。

use serde::Serialize;
use tauri::menu::{AboutMetadata, Menu, MenuItem, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::{AppHandle, Runtime};

/// 回传给前端的菜单动作，结构对齐 `TitleBar.onAction(p: { key, value? })`
#[derive(Debug, Clone, Serialize)]
pub struct MenuAction {
    pub key: String,
    pub value: Option<String>,
}

/// 前端监听的菜单事件名（与 `TitleBar.vue` 中的 `listen('ll-menu')` 一致）
pub const MENU_EVENT: &str = "ll-menu";

/// 构造带 id 与可选加速键的普通菜单项
fn item<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
    text: &str,
    accel: Option<&str>,
) -> tauri::Result<MenuItem<R>> {
    let mut b = MenuItemBuilder::with_id(id, text);
    if let Some(a) = accel {
        b = b.accelerator(a);
    }
    b.build(app)
}

/// 构建完整的 macOS 菜单栏。
///
/// 结构遵循 macOS HIG：第一个子菜单是「应用菜单」（关于 / 设置 / 服务 / 隐藏 / 退出），
/// 之后才是 文件 / 编辑 / 设备 / 视图 / 窗口 / 帮助。这里省掉「编辑」是因为当前
/// 应用没有富文本编辑场景，输入框的复制粘贴由系统默认实现兜底。
pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    // ---- 应用菜单（macOS 会用 Bundle 里的应用名替换这个标题）----
    let about = PredefinedMenuItem::about(
        app,
        Some("关于 LanLink"),
        Some(AboutMetadata {
            name: Some("LanLink".into()),
            version: Some(env!("CARGO_PKG_VERSION").into()),
            short_version: Some(env!("CARGO_PKG_VERSION").into()),
            comments: Some("局域网 / 跨网 P2P 文件传输".into()),
            ..Default::default()
        }),
    )?;
    let settings = item(app, "settings", "设置…", Some("Cmd+,"))?;

    let app_menu = SubmenuBuilder::new(app, "LanLink")
        .item(&about)
        .separator()
        .item(&settings)
        .separator()
        .services_with_text("服务")
        .separator()
        .hide_with_text("隐藏 LanLink")
        .hide_others_with_text("隐藏其他")
        .show_all_with_text("全部显示")
        .separator()
        .quit_with_text("退出 LanLink")
        .build()?;

    // ---- 文件 ----
    let file_menu = SubmenuBuilder::new(app, "文件")
        .item(&item(app, "open-file", "选择文件…", Some("Cmd+O"))?)
        .item(&item(app, "open-folder", "选择文件夹…", Some("Cmd+Shift+O"))?)
        .item(&item(
            app,
            "paste-send",
            "从剪贴板发送",
            Some("Cmd+Shift+V"),
        )?)
        .separator()
        .item(&item(app, "view-history", "传输记录", None)?)
        .item(&item(app, "open-dl", "打开下载目录", Some("Cmd+Shift+D"))?)
        .separator()
        .close_window_with_text("关闭窗口")
        .build()?;

    // ---- 设备 ----
    let device_menu = SubmenuBuilder::new(app, "设备")
        .item(&item(app, "scan", "扫描局域网设备", Some("Cmd+R"))?)
        .item(&item(app, "nearby", "附近设备", None)?)
        .separator()
        .item(&item(app, "gen", "生成外网序列号…", Some("Cmd+G"))?)
        .item(&item(app, "gen-lan", "生成局域网短码…", Some("Cmd+L"))?)
        .item(&item(app, "connect", "连接到远程设备…", Some("Cmd+K"))?)
        .separator()
        .item(&item(app, "netinfo", "本机网络信息", None)?)
        .item(&item(app, "portcheck", "端口与防火墙检查", None)?)
        .build()?;

    // ---- 视图（含 外观 / 视觉风格 二级菜单）----
    let theme_menu = SubmenuBuilder::new(app, "外观")
        .item(&item(app, "theme|system", "跟随系统", None)?)
        .item(&item(app, "theme|light", "浅色", None)?)
        .item(&item(app, "theme|dark", "深色", None)?)
        .build()?;

    // 顺序与设置页 3×2 卡片一致，最后补「原型基准」
    let mut palette = SubmenuBuilder::new(app, "视觉风格");
    for (id, label) in [
        ("graphite", "石墨"),
        ("aurora", "柔雾极光"),
        ("slate", "精炼"),
        ("carbon", "深邃"),
        ("nocturne", "柔和"),
        ("amber", "琥珀"),
    ] {
        palette = palette.item(&item(app, &format!("palette|{id}"), label, None)?);
    }
    let palette_menu = palette
        .separator()
        .item(&item(app, "palette|prototype", "原型基准", None)?)
        .build()?;

    let view_menu = SubmenuBuilder::new(app, "视图")
        .item(&item(app, "view-send", "发送文件", Some("Cmd+1"))?)
        .item(&item(app, "view-recv", "接收文件", Some("Cmd+2"))?)
        .item(&item(app, "view-history-2", "传输记录", Some("Cmd+3"))?)
        .separator()
        .item(&item(app, "toggle-rail", "显示侧栏", None)?)
        .item(&item(app, "toggle-rate", "显示速率曲线", None)?)
        .separator()
        .item(&theme_menu)
        .item(&palette_menu)
        .separator()
        .fullscreen_with_text("进入全屏")
        .build()?;

    // ---- 窗口 ----
    let window_menu = SubmenuBuilder::new(app, "窗口")
        .minimize_with_text("最小化")
        .maximize_with_text("缩放")
        .separator()
        .close_window_with_text("关闭窗口")
        .build()?;

    // ---- 帮助 ----
    let help_menu = SubmenuBuilder::new(app, "帮助")
        .item(&item(app, "help", "使用指南", None)?)
        .item(&item(app, "shortcuts", "快捷键一览", None)?)
        .item(&item(app, "errcodes", "错误码说明 E001–E009", None)?)
        .item(&item(app, "protocol", "协议与安全说明", None)?)
        .separator()
        .item(&item(app, "update", "检查更新", None)?)
        .build()?;

    Menu::with_items(
        app,
        &[
            &app_menu,
            &file_menu,
            &device_menu,
            &view_menu,
            &window_menu,
            &help_menu,
        ],
    )
}

/// 把 `key|value` 形式的菜单 id 拆成前端可直接分发的动作。
pub fn split_id(id: &str) -> MenuAction {
    match id.split_once('|') {
        Some((k, v)) => MenuAction {
            key: k.to_string(),
            value: Some(v.to_string()),
        },
        None => MenuAction {
            key: id.to_string(),
            value: None,
        },
    }
}
