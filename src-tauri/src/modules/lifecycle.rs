use tauri::AppHandle;

#[cfg(not(target_os = "windows"))]
use tauri::Manager;

const TRAY_ID: &str = "main-tray";

/// 通过 `set_visible(false)` 移除托盘图标；macOS 走主线程代理，跨线程安全。
pub fn remove_tray_icon_before_exit(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Err(e) = tray.set_visible(false) {
            tracing::warn!("退出时移除托盘图标失败: {e}");
        } else {
            tracing::info!("已显式从系统托盘移除图标");
        }
    }
}

/// 主动释放 single-instance 锁，避免重启后新进程误连旧 listener。
pub fn destroy_single_instance_lock(app: &AppHandle) {
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    tauri_plugin_single_instance::destroy(app);
}

/// 移除托盘并释放单实例锁（Windows install 前、或配合 restart 使用）。
pub fn prepare_for_process_exit(app: &AppHandle) {
    remove_tray_icon_before_exit(app);
    destroy_single_instance_lock(app);
}

/// 清理托盘与单实例锁后直接 `process::restart`，不经过 `request_restart` 事件循环。
#[cfg(not(target_os = "windows"))]
pub fn restart_process(app: &AppHandle) -> ! {
    prepare_for_process_exit(app);
    tauri::process::restart(&app.env());
}
