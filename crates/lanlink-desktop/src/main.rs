#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// 虚拟机 / 无 GPU 环境下的 WebKitGTK 兼容兜底。
///
/// WebKitGTK 默认走 DMABUF + 合成器（GPU 加速）。在 VirtualBox / VMware / QEMU
/// 这类没有可用硬件 OpenGL 的环境里，合成器会持续等待 GPU 完成信号，表现为：
/// 窗口能画出来，但整机 UI 逐渐冻结、CPU 占用接近 0、连 SSH 都不再响应。
///
/// 必须在 WebKit 初始化之前设置环境变量，所以放在 main 的最前面。
#[cfg(target_os = "linux")]
fn tune_webkit_for_virtual_machine() {
    let product = std::fs::read_to_string("/sys/class/dmi/id/product_name")
        .unwrap_or_default()
        .to_lowercase();
    let sys_vendor = std::fs::read_to_string("/sys/class/dmi/id/sys_vendor")
        .unwrap_or_default()
        .to_lowercase();
    let probe = format!("{product} {sys_vendor}");

    let is_vm = [
        "virtualbox",
        "innotek",
        "vmware",
        "qemu",
        "kvm",
        "hyper-v",
        "parallels",
    ]
    .iter()
    .any(|k| probe.contains(k));

    if !is_vm {
        return;
    }

    // 用户显式设置过的项不覆盖，保留手动调优的空间。
    for (key, val) in [
        ("WEBKIT_DISABLE_COMPOSITING_MODE", "1"),
        ("WEBKIT_DISABLE_DMABUF_RENDERER", "1"),
        ("LIBGL_ALWAYS_SOFTWARE", "1"),
    ] {
        if std::env::var_os(key).is_none() {
            std::env::set_var(key, val);
        }
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    tune_webkit_for_virtual_machine();

    lanlink_desktop::run()
}
