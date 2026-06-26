mod commands;
mod error;
#[cfg(target_os = "linux")]
mod linux_fix;
mod models;
mod modules;
mod state;
mod utils;

use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 控制台 fmt 层（RUST_LOG 可覆盖）+ 捕获层（动态级别 + log-line 事件推送）
    use tracing_subscriber::prelude::*;
    // 默认 info，并压制第三方框架噪音：tao/wry 等经 `log` crate 桥接 → log=warn；HTTP 栈降到 warn。
    // 设 RUST_LOG 可覆盖（例：RUST_LOG=ccmesh=debug,log=warn 只看本项目 debug）。
    let console_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new(
                "info,log=warn,hyper=warn,reqwest=warn,h2=warn,rustls=warn,tao=warn,wry=warn",
            )
        });
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(console_filter))
        .with(modules::logs::CaptureLayer)
        .init();

    let mut builder = tauri::Builder::default();

    // 应用单例：必须最先注册。二次启动（含点击桌面快捷方式）回调到已运行实例，
    // 唤起并聚焦已有主窗口，避免多开造成端口冲突。Windows/macOS/Linux 通用。
    #[cfg(desktop)]
    {
        if std::env::var_os("CCMESH_ALLOW_MULTI_INSTANCE").is_none() {
            builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                    // Linux：show() 后补一次窗口交互重激活，修复 WebKitGTK 整窗点击无响应。
                    #[cfg(target_os = "linux")]
                    linux_fix::nudge_main_window(w.clone());
                }
            }));
        }
    }

    builder
        // 开机自启（自启动）：仅注册插件，启停由前端按设置开关调用 enable/disable。
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();

            // 数据目录 → 连接池 → 幂等迁移
            let db_file = utils::paths::db_path(&handle)?;
            let pool = modules::storage::db::create_pool(&db_file)?;
            {
                let conn = pool.get()?;
                modules::storage::migration::run_migrations(&conn)?;
            }

            // 日志级别（从配置恢复）+ 实时推送接线
            {
                let conn = pool.get()?;
                if let Ok(Some(level)) = modules::storage::config_repo::get_value(&conn, "logLevel")
                {
                    modules::logs::set_level(&level);
                }
            }
            modules::logs::set_app_handle(handle.clone());

            // 设备唯一 ID
            let device_id = {
                let conn = pool.get()?;
                modules::storage::device::get_or_create_device_id(&conn)?
            };
            tracing::info!(%device_id, "存储与设备标识初始化完成");

            // 统计聚合器（内存累加 + 2s 防抖落库 + 零延迟事件）
            let stats = modules::stats::aggregator::StatsAggregator::new(
                pool.clone(),
                handle.clone(),
                device_id.clone(),
            );

            // 注入全局状态，保存 AppHandle 供事件推送
            let app_state = state::AppState::new(pool, device_id, stats);
            let _ = app_state.app_handle.set(handle);
            app.manage(app_state);

            // 系统托盘
            if let Err(e) = modules::tray::build_tray(app.handle()) {
                tracing::warn!("托盘构建失败: {e}");
            }

            // 窗口关闭行为（quit 退出 / minimize 隐藏托盘 / ask 前端询问）
            if let Some(window) = app.get_webview_window("main") {
                let close_handle = window.app_handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        let behavior = {
                            let state = close_handle.state::<state::AppState>();
                            state
                                .db_pool
                                .get()
                                .ok()
                                .and_then(|c| {
                                    modules::storage::config_repo::get_value(
                                        &c,
                                        "closeWindowBehavior",
                                    )
                                    .ok()
                                    .flatten()
                                })
                                .unwrap_or_else(|| "ask".to_string())
                        };
                        match behavior.as_str() {
                            "quit" => close_handle.exit(0),
                            "minimize" => {
                                api.prevent_close();
                                if let Some(w) = close_handle.get_webview_window("main") {
                                    let _ = w.hide();
                                }
                            }
                            _ => {
                                api.prevent_close();
                                let _ = close_handle.emit("close-requested", ());
                            }
                        }
                    }
                });
            }

            // 自动运行（默认开）：应用打开即拉起代理服务，覆盖静默启动等无 UI 交互场景。
            // 放后端 setup 而非前端，确保不展示窗口时也能自动运行。
            let auto_run = app
                .state::<state::AppState>()
                .db_pool
                .get()
                .ok()
                .and_then(|c| modules::storage::config_repo::get_config(&c).ok())
                .map(|cfg| cfg.auto_run)
                .unwrap_or_else(|| models::config::AppConfig::default().auto_run);
            let auto_run = models::config::auto_run_with_env_override(auto_run);
            if auto_run {
                let run_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let state = run_handle.state::<state::AppState>();
                    // 已运行则跳过（避免重复绑定端口）；锁守卫在语句结束即释放，不跨 await。
                    if state.proxy.lock().unwrap().is_some() {
                        return;
                    }
                    let port = state
                        .db_pool
                        .get()
                        .ok()
                        .and_then(|c| modules::storage::config_repo::get_config(&c).ok())
                        .map(|cfg| cfg.port)
                        .unwrap_or_else(|| models::config::AppConfig::default().port);
                    let port = models::config::port_with_env_override(port);
                    match modules::proxy::server::start_proxy(
                        run_handle.clone(),
                        state.db_pool.clone(),
                        port,
                        state.stats.clone(),
                    )
                    .await
                    {
                        Ok(handle) => {
                            *state.proxy.lock().unwrap() = Some(handle);
                            let status = commands::proxy::build_status(&state);
                            let _ = run_handle.emit(commands::proxy::PROXY_STATUS_EVENT, status);
                            tracing::info!("自动运行：代理已启动");
                        }
                        Err(e) => tracing::warn!("自动运行：代理启动失败: {e}"),
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health::get_health,
            commands::health::get_endpoint_health,
            commands::proxy::start_proxy,
            commands::proxy::stop_proxy,
            commands::proxy::get_proxy_status,
            commands::proxy::switch_endpoint,
            commands::stats::get_stats,
            commands::stats::get_request_logs,
            commands::stats::get_stats_history,
            commands::stats::delete_daily_stat,
            commands::stats::delete_stats_by_date,
            commands::usage::sync_session_usage,
            commands::usage::get_usage_summary,
            commands::usage::get_usage_by_model,
            commands::usage::get_usage_by_day,
            commands::usage::get_usage_by_day_model,
            commands::backup::export_config,
            commands::backup::import_config,
            commands::config::get_config,
            commands::config::get_all_config,
            commands::config::set_config,
            commands::rules::get_rules_config,
            commands::rules::set_rules_config,
            commands::rules::reset_rules_config,
            commands::endpoint::list_endpoints,
            commands::endpoint::create_endpoint,
            commands::endpoint::update_endpoint,
            commands::endpoint::delete_endpoint,
            commands::endpoint::reorder_endpoints,
            commands::endpoint::clone_endpoint,
            commands::endpoint::test_endpoint,
            commands::endpoint::query_endpoint_balance,
            commands::endpoint::probe_endpoint_balance_templates,
            commands::endpoint::generate_balance_template_with_ai,
            commands::endpoint::test_proxy,
            commands::models::get_models,
            commands::models::fetch_endpoint_models,
            commands::tokens::count_tokens,
            commands::logs::get_recent_logs,
            commands::logs::set_log_level,
            commands::webdav::test_webdav,
            commands::webdav::webdav_backup,
            commands::webdav::webdav_restore,
            commands::webdav::webdav_list_backups,
            commands::webdav::webdav_delete_backup,
            commands::window::set_language,
            commands::window::apply_close_action,
            commands::window::hide_to_tray,
            commands::window::notify_window_shown,
            commands::update::check_for_updates,
            commands::update::download_and_install,
            commands::update::get_update_settings,
            commands::update::set_update_settings,
            commands::update::skip_version,
            commands::tool_config::list_profile_channels,
            commands::tool_config::get_profile_channel,
            commands::tool_config::save_profile_channel,
            commands::tool_config::delete_profile_channel,
            commands::tool_config::extract_source_record,
            commands::tool_config::apply_profile_config,
            commands::tool_config::preview_claude_settings,
            commands::tool_config::parse_claude_fields,
            commands::tool_config::preview_codex_config,
            commands::tool_config::parse_codex_fields
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app_handle, _event| {
            // macOS：窗口最小化/隐藏后点击 Dock 图标会触发 Reopen，
            // 默认不会恢复窗口，这里手动显示并取消最小化。
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                if let Some(w) = _app_handle.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
            }
        });
}
