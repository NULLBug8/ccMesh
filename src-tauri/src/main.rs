// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Linux：在 WebKitGTK 初始化前设置环境变量，规避部分发行版/GPU 下的渲染与
    // 输入失效。必须早于 tauri::Builder（即 run()）执行，否则 WebView 已初始化无效。
    // 参考 cc-switch；Tauri #9394。
    #[cfg(target_os = "linux")]
    {
        // DMA-BUF 渲染器在某些环境（如 Nvidia、虚拟机）导致白屏/黑屏。
        if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        // 禁用合成模式，规避 `visible:false → show()` 路径下 GTK surface 与
        // WebKitWebView 的 input region 协商失败：整窗 UI 点击无响应、必须
        // 最大化-还原才能恢复（本次修复的主症状）。
        if std::env::var("WEBKIT_DISABLE_COMPOSITING_MODE").is_err() {
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        }
    }

    ccmesh_lib::run()
}
