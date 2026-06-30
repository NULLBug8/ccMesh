pub mod commands;
pub mod error;
pub mod models;
pub mod modules;
pub mod runtime;
pub mod state;
pub mod utils;

use std::path::PathBuf;
use std::sync::Arc;

use tracing_subscriber::prelude::*;

use crate::error::AppResult;

pub async fn run_server() -> AppResult<()> {
    init_logging();

    let data_dir = resolve_data_dir()?;
    let resource_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let app = runtime::AppHandle::new(data_dir.clone(), resource_dir);

    let db_file = utils::paths::db_path(&app)?;
    let pool = modules::storage::db::create_pool(&db_file)?;
    {
        let conn = pool.get()?;
        modules::storage::migration::run_migrations(&conn)?;
    }

    {
        let conn = pool.get()?;
        if let Ok(Some(level)) = modules::storage::config_repo::get_value(&conn, "logLevel") {
            modules::logs::set_level(&level);
        }
    }
    modules::logs::set_app_handle(app.clone());

    let device_id = {
        let conn = pool.get()?;
        modules::storage::device::get_or_create_device_id(&conn)?
    };
    tracing::info!(%device_id, "storage initialized");

    let stats =
        modules::stats::aggregator::StatsAggregator::new(pool.clone(), app.clone(), device_id.clone());
    let state = Arc::new(state::AppState::new(pool, device_id, stats));
    app.set_state(state.clone())?;

    let port = {
        let conn = state.db_pool.get()?;
        let cfg = modules::storage::config_repo::get_config(&conn)?;
        models::config::port_with_env_override(cfg.port)
    };

    let handle =
        modules::proxy::server::start_proxy(app.clone(), state.db_pool.clone(), port, state.stats.clone())
            .await?;
    *state.proxy.lock().unwrap() = Some(handle);
    tracing::info!(port, "ccMesh web server started");

    tokio::signal::ctrl_c().await.ok();
    if let Some(handle) = state.proxy.lock().unwrap().take() {
        handle.stop().await;
    }
    Ok(())
}

fn init_logging() {
    let console_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new(
                "info,log=warn,hyper=warn,reqwest=warn,h2=warn,rustls=warn,tokio=warn",
            )
        });
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(console_filter))
        .with(modules::logs::CaptureLayer)
        .try_init();
}

fn resolve_data_dir() -> AppResult<PathBuf> {
    let dir = std::env::var_os("CCMESH_DATA_DIR")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| {
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
                .join("ccmesh")
        });
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
