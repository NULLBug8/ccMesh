use std::collections::HashMap;
use std::convert::Infallible;

use axum::extract::State;
use axum::http::{HeaderValue, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::commands::{
    backup, config, endpoint, health, logs, models, proxy, rules, stats, tokens, tool_config,
    update, usage, webdav, window,
};
use crate::error::{AppError, AppResult};
use crate::models::backup::ImportSummary;
use crate::models::config::WebDavConfig;
use crate::models::endpoint::{CreateEndpointRequest, UpdateEndpointRequest};
use crate::models::rules::RulesConfig;
use crate::models::tool_config::{ClaudeOperationFields, CodexOperationFields, SaveChannelRequest};
use crate::state::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvokeRequest {
    pub command: String,
    #[serde(default)]
    pub args: Value,
}

#[derive(Serialize)]
pub struct InvokeResponse {
    pub data: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestLogsArgs {
    start_ms: Option<i64>,
    end_ms: Option<i64>,
    endpoint: Option<String>,
    page: i64,
    page_size: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatsHistoryArgs {
    page: i64,
    page_size: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteDailyStatArgs {
    endpoint_name: String,
    date: String,
}

#[derive(Deserialize)]
struct DeleteStatsByDateArgs {
    date: String,
}

#[derive(Deserialize)]
struct SetLogLevelArgs {
    level: String,
}

#[derive(Deserialize)]
struct TestProxyArgs {
    url: String,
}

#[derive(Deserialize)]
struct SwitchEndpointArgs {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetConfigArgs {
    patch: HashMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateEndpointArgs {
    req: CreateEndpointRequest,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateEndpointArgs {
    id: i64,
    req: UpdateEndpointRequest,
}

#[derive(Deserialize)]
struct IdArg {
    id: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReorderArgs {
    ordered_ids: Vec<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestEndpointArgs {
    id: i64,
    model: Option<String>,
    mode: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestAllEndpointsArgs {
    mode: Option<String>,
}

#[derive(Deserialize)]
struct QueryEndpointBalanceArgs {
    id: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestEndpointBalanceQueryArgs {
    id: i64,
    balance_query: crate::models::endpoint::BalanceQueryConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProbeEndpointBalanceTemplatesArgs {
    id: i64,
    custom_path: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerateBalanceTemplateWithAiArgs {
    id: i64,
    ai_model: String,
    samples: Vec<endpoint::BalanceTemplateAiSample>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FetchEndpointModelsArgs {
    api_url: String,
    api_key: String,
    transformer: String,
    use_proxy: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetRulesArgs {
    config: RulesConfig,
}

#[derive(Deserialize)]
struct ForceRefreshArgs {
    force_refresh: Option<bool>,
    #[serde(rename = "forceRefresh")]
    force_refresh_camel: Option<bool>,
}

#[derive(Deserialize)]
struct CountTokensArgs {
    request: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageFilterArgs {
    start: Option<String>,
    end: Option<String>,
    #[serde(alias = "app_type")]
    app_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestWebDavArgs {
    config: WebDavConfig,
}

#[derive(Deserialize)]
struct VersionArg {
    version: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateSettingsArgs {
    #[serde(alias = "auto_check")]
    auto_check: Option<bool>,
    #[serde(alias = "check_interval")]
    check_interval: Option<i64>,
}

#[derive(Deserialize)]
struct ExportConfigArgs {
    path: String,
}

#[derive(Deserialize)]
struct ImportConfigArgs {
    path: String,
    strategy: String,
}

#[derive(Deserialize)]
struct WebDavRestoreArgs {
    filename: String,
    strategy: Option<String>,
}

#[derive(Deserialize)]
struct FilenameArg {
    filename: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolConfigAppTypeArg {
    app_type: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolConfigGetArg {
    app_type: String,
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolConfigSaveArg {
    app_type: String,
    req: SaveChannelRequest,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolConfigDeleteArg {
    app_type: String,
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ToolConfigApplyArg {
    app_type: String,
    snapshot: Value,
}

#[derive(Deserialize)]
struct PreviewClaudeArgs {
    base: Value,
    fields: ClaudeOperationFields,
}

#[derive(Deserialize)]
struct ParseClaudeArgs {
    snapshot: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreviewCodexArgs {
    #[serde(alias = "config_toml")]
    config_toml: String,
    fields: CodexOperationFields,
    #[serde(alias = "goal_mode")]
    goal_mode: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParseCodexArgs {
    auth: Value,
    #[serde(alias = "config_toml")]
    config_toml: String,
}

#[derive(Deserialize)]
struct WindowActionArg {
    action: String,
}

fn to_json<T: Serialize>(value: T) -> AppResult<Value> {
    serde_json::to_value(value).map_err(AppError::from)
}

fn parse_args<T: for<'de> Deserialize<'de>>(value: Value) -> AppResult<T> {
    serde_json::from_value(value)
        .map_err(|error| AppError::InvalidArgument(format!("参数无效: {error}")))
}

fn json_error(status: StatusCode, error: AppError) -> Response {
    (status, Json(json!({ "error": error.to_string() }))).into_response()
}

fn app_state(app: &AppHandle) -> tauri::State<'_, AppState> {
    app.state::<AppState>()
}

pub async fn invoke_http(
    State(proxy_state): State<std::sync::Arc<crate::modules::proxy::forward::ProxyState>>,
    Json(body): Json<InvokeRequest>,
) -> Response {
    let app = proxy_state.app.clone();
    let state = app_state(&app);

    let result: AppResult<Value> = async {
        match body.command.as_str() {
            "get_health" => to_json(health::get_health(state.clone())?),
            "get_endpoint_health" => to_json(health::get_endpoint_health(state.clone())?),
            "start_proxy" => to_json(proxy::start_proxy(app.clone(), state.clone()).await?),
            "stop_proxy" => to_json(proxy::stop_proxy(app.clone(), state.clone()).await?),
            "get_proxy_status" => to_json(proxy::get_proxy_status(state.clone())?),
            "switch_endpoint" => {
                let args: SwitchEndpointArgs = parse_args(body.args)?;
                to_json(proxy::switch_endpoint(
                    app.clone(),
                    state.clone(),
                    args.name,
                )?)
            }
            "get_stats" => to_json(stats::get_stats(state.clone())?),
            "get_request_logs" => {
                let args: RequestLogsArgs = parse_args(body.args)?;
                to_json(stats::get_request_logs(
                    state.clone(),
                    args.start_ms,
                    args.end_ms,
                    args.endpoint,
                    args.page,
                    args.page_size,
                )?)
            }
            "get_stats_history" => {
                let args: StatsHistoryArgs = parse_args(body.args)?;
                to_json(stats::get_stats_history(
                    state.clone(),
                    args.page,
                    args.page_size,
                )?)
            }
            "delete_daily_stat" => {
                let args: DeleteDailyStatArgs = parse_args(body.args)?;
                to_json(stats::delete_daily_stat(
                    state.clone(),
                    args.endpoint_name,
                    args.date,
                )?)
            }
            "delete_stats_by_date" => {
                let args: DeleteStatsByDateArgs = parse_args(body.args)?;
                to_json(stats::delete_stats_by_date(state.clone(), args.date)?)
            }
            "get_recent_logs" => to_json(logs::get_recent_logs()?),
            "set_log_level" => {
                let args: SetLogLevelArgs = parse_args(body.args)?;
                logs::set_log_level(state.clone(), args.level)?;
                to_json(())
            }
            "get_config" => to_json(config::get_config(state.clone())?),
            "set_config" => {
                let args: SetConfigArgs = parse_args(body.args)?;
                to_json(config::set_config(app.clone(), state.clone(), args.patch).await?)
            }
            "test_proxy" => {
                let args: TestProxyArgs = parse_args(body.args)?;
                to_json(endpoint::test_proxy(args.url).await?)
            }
            "list_endpoints" => to_json(endpoint::list_endpoints(state.clone())?),
            "create_endpoint" => {
                let args: CreateEndpointArgs = parse_args(body.args)?;
                to_json(endpoint::create_endpoint(state.clone(), args.req)?)
            }
            "update_endpoint" => {
                let args: UpdateEndpointArgs = parse_args(body.args)?;
                to_json(endpoint::update_endpoint(
                    app.clone(),
                    state.clone(),
                    args.id,
                    args.req,
                )?)
            }
            "delete_endpoint" => {
                let args: IdArg = parse_args(body.args)?;
                endpoint::delete_endpoint(state.clone(), args.id)?;
                to_json(())
            }
            "reorder_endpoints" => {
                let args: ReorderArgs = parse_args(body.args)?;
                endpoint::reorder_endpoints(app.clone(), state.clone(), args.ordered_ids)?;
                to_json(())
            }
            "clone_endpoint" => {
                let args: IdArg = parse_args(body.args)?;
                to_json(endpoint::clone_endpoint(state.clone(), args.id)?)
            }
            "test_endpoint" => {
                let args: TestEndpointArgs = parse_args(body.args)?;
                to_json(
                    endpoint::test_endpoint(
                        app.clone(),
                        state.clone(),
                        args.id,
                        args.model,
                        args.mode,
                    )
                    .await?,
                )
            }
            "test_all_endpoints" => {
                let args: TestAllEndpointsArgs = parse_args(body.args)?;
                to_json(endpoint::test_all_endpoints(app.clone(), state.clone(), args.mode).await?)
            }
            "query_endpoint_balance" => {
                let args: QueryEndpointBalanceArgs = parse_args(body.args)?;
                to_json(endpoint::query_endpoint_balance(state.clone(), args.id).await?)
            }
            "test_endpoint_balance_query" => {
                let args: TestEndpointBalanceQueryArgs = parse_args(body.args)?;
                to_json(
                    endpoint::test_endpoint_balance_query(
                        state.clone(),
                        args.id,
                        args.balance_query,
                    )
                    .await?,
                )
            }
            "probe_endpoint_balance_templates" => {
                let args: ProbeEndpointBalanceTemplatesArgs = parse_args(body.args)?;
                to_json(
                    endpoint::probe_endpoint_balance_templates(
                        state.clone(),
                        args.id,
                        args.custom_path,
                    )
                    .await?,
                )
            }
            "generate_balance_template_with_ai" => {
                let args: GenerateBalanceTemplateWithAiArgs = parse_args(body.args)?;
                to_json(
                    endpoint::generate_balance_template_with_ai(
                        state.clone(),
                        args.id,
                        args.ai_model,
                        args.samples,
                    )
                    .await?,
                )
            }
            "fetch_endpoint_models" => {
                let args: FetchEndpointModelsArgs = parse_args(body.args)?;
                to_json(
                    models::fetch_endpoint_models(
                        state.clone(),
                        args.api_url,
                        args.api_key,
                        args.transformer,
                        args.use_proxy,
                    )
                    .await?,
                )
            }
            "get_rules_config" => to_json(rules::get_rules_config(state.clone())?),
            "set_rules_config" => {
                let args: SetRulesArgs = parse_args(body.args)?;
                to_json(rules::set_rules_config(app.clone(), state.clone(), args.config).await?)
            }
            "reset_rules_config" => {
                to_json(rules::reset_rules_config(app.clone(), state.clone()).await?)
            }
            "get_models" => {
                let args: ForceRefreshArgs = parse_args(body.args)?;
                let force = args.force_refresh.or(args.force_refresh_camel);
                to_json(models::get_models(state.clone(), force).await?)
            }
            "count_tokens" => {
                let args: CountTokensArgs = parse_args(body.args)?;
                to_json(tokens::count_tokens(args.request)?)
            }
            "sync_session_usage" => to_json(usage::sync_session_usage(state.clone()).await?),
            "get_usage_summary" => {
                let args: UsageFilterArgs = parse_args(body.args)?;
                to_json(usage::get_usage_summary(
                    state.clone(),
                    args.start,
                    args.end,
                    args.app_type,
                )?)
            }
            "get_usage_by_day_model" => {
                let args: UsageFilterArgs = parse_args(body.args)?;
                to_json(usage::get_usage_by_day_model(
                    state.clone(),
                    args.start,
                    args.end,
                    args.app_type,
                )?)
            }
            "test_webdav" => {
                let args: TestWebDavArgs = parse_args(body.args)?;
                to_json(webdav::test_webdav(args.config).await?)
            }
            "webdav_backup" => to_json(webdav::webdav_backup(state.clone()).await?),
            "webdav_restore" => {
                let args: WebDavRestoreArgs = parse_args(body.args)?;
                webdav::webdav_restore(state.clone(), args.filename, args.strategy).await?;
                to_json(())
            }
            "webdav_list_backups" => to_json(webdav::webdav_list_backups(state.clone()).await?),
            "webdav_delete_backup" => {
                let args: FilenameArg = parse_args(body.args)?;
                webdav::webdav_delete_backup(state.clone(), args.filename).await?;
                to_json(())
            }
            "check_for_updates" => {
                to_json(update::check_for_updates(app.clone(), state.clone()).await?)
            }
            "download_and_install" => {
                update::download_and_install(app.clone(), state.clone()).await?;
                to_json(())
            }
            "get_update_settings" => to_json(update::get_update_settings(state.clone())?),
            "set_update_settings" => {
                let args: UpdateSettingsArgs = parse_args(body.args)?;
                let auto_check = args
                    .auto_check
                    .ok_or_else(|| AppError::InvalidArgument("缺少 autoCheck".into()))?;
                let check_interval = args
                    .check_interval
                    .ok_or_else(|| AppError::InvalidArgument("缺少 checkInterval".into()))?;
                update::set_update_settings(state.clone(), auto_check, check_interval)?;
                to_json(())
            }
            "skip_version" => {
                let args: VersionArg = parse_args(body.args)?;
                update::skip_version(state.clone(), args.version)?;
                to_json(())
            }
            "export_config" => {
                let args: ExportConfigArgs = parse_args(body.args)?;
                backup::export_config(state.clone(), args.path)?;
                to_json(())
            }
            "import_config" => {
                let args: ImportConfigArgs = parse_args(body.args)?;
                let summary: ImportSummary =
                    backup::import_config(state.clone(), args.path, args.strategy)?;
                to_json(summary)
            }
            "list_profile_channels" => {
                let args: ToolConfigAppTypeArg = parse_args(body.args)?;
                to_json(tool_config::list_profile_channels(
                    app.clone(),
                    args.app_type,
                )?)
            }
            "get_profile_channel" => {
                let args: ToolConfigGetArg = parse_args(body.args)?;
                to_json(tool_config::get_profile_channel(
                    app.clone(),
                    args.app_type,
                    args.id,
                )?)
            }
            "save_profile_channel" => {
                let args: ToolConfigSaveArg = parse_args(body.args)?;
                to_json(tool_config::save_profile_channel(
                    app.clone(),
                    args.app_type,
                    args.req,
                )?)
            }
            "delete_profile_channel" => {
                let args: ToolConfigDeleteArg = parse_args(body.args)?;
                tool_config::delete_profile_channel(app.clone(), args.app_type, args.id)?;
                to_json(())
            }
            "extract_source_record" => {
                let args: ToolConfigAppTypeArg = parse_args(body.args)?;
                to_json(tool_config::extract_source_record(
                    app.clone(),
                    args.app_type,
                )?)
            }
            "apply_profile_config" => {
                let args: ToolConfigApplyArg = parse_args(body.args)?;
                tool_config::apply_profile_config(app.clone(), args.app_type, args.snapshot)?;
                to_json(())
            }
            "preview_claude_settings" => {
                let args: PreviewClaudeArgs = parse_args(body.args)?;
                to_json(tool_config::preview_claude_settings(
                    args.base,
                    args.fields,
                )?)
            }
            "parse_claude_fields" => {
                let args: ParseClaudeArgs = parse_args(body.args)?;
                to_json(tool_config::parse_claude_fields(args.snapshot)?)
            }
            "preview_codex_config" => {
                let args: PreviewCodexArgs = parse_args(body.args)?;
                to_json(tool_config::preview_codex_config(
                    args.config_toml,
                    args.fields,
                    args.goal_mode,
                )?)
            }
            "parse_codex_fields" => {
                let args: ParseCodexArgs = parse_args(body.args)?;
                to_json(tool_config::parse_codex_fields(
                    args.auth,
                    args.config_toml,
                )?)
            }
            "set_language" => {
                let args: HashMap<String, String> = parse_args(body.args)?;
                let lang = args
                    .get("lang")
                    .cloned()
                    .ok_or_else(|| AppError::InvalidArgument("缺少 lang".into()))?;
                window::set_language(app.clone(), state.clone(), lang)?;
                to_json(())
            }
            "apply_close_action" => {
                let args: WindowActionArg = parse_args(body.args)?;
                window::apply_close_action(app.clone(), args.action)?;
                to_json(())
            }
            "hide_to_tray" => {
                window::hide_to_tray(app.clone())?;
                to_json(())
            }
            "notify_window_shown" => {
                window::notify_window_shown(app.clone());
                to_json(())
            }
            command => Err(AppError::NotFound(format!("未知命令: {command}"))),
        }
    }
    .await;

    match result {
        Ok(data) => Json(InvokeResponse { data }).into_response(),
        Err(error) => json_error(StatusCode::BAD_REQUEST, error),
    }
}

pub async fn events_sse(
    State(_proxy_state): State<std::sync::Arc<crate::modules::proxy::forward::ProxyState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = crate::modules::web_admin::bridge::subscribe();
    let stream = BroadcastStream::new(receiver).filter_map(|message| match message {
        Ok(event) => {
            let data = json!({
                "event": event.event,
                "payload": serde_json::from_str::<Value>(&event.payload)
                    .unwrap_or(Value::String(event.payload)),
            });
            Some(Ok(Event::default().data(data.to_string())))
        }
        Err(_) => None,
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub async fn static_asset_root(
    State(proxy_state): State<std::sync::Arc<crate::modules::proxy::forward::ProxyState>>,
) -> Response {
    serve_static_asset(&proxy_state, "").await
}

pub async fn static_asset_favicon(
    State(proxy_state): State<std::sync::Arc<crate::modules::proxy::forward::ProxyState>>,
) -> Response {
    serve_static_asset(&proxy_state, "favicon.ico").await
}

pub async fn static_asset_root_assets(
    State(proxy_state): State<std::sync::Arc<crate::modules::proxy::forward::ProxyState>>,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    serve_static_asset(&proxy_state, &format!("assets/{path}")).await
}

pub async fn static_asset(
    State(proxy_state): State<std::sync::Arc<crate::modules::proxy::forward::ProxyState>>,
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    serve_static_asset(&proxy_state, &path).await
}

async fn serve_static_asset(
    proxy_state: &std::sync::Arc<crate::modules::proxy::forward::ProxyState>,
    path: &str,
) -> Response {
    match crate::modules::web_admin::static_files::load(&proxy_state.app, path) {
        Ok(Some((content_type, body))) => {
            let mut response = Response::new(axum::body::Body::from(body));
            response.headers_mut().insert(
                axum::http::header::CONTENT_TYPE,
                HeaderValue::from_str(&content_type)
                    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            );
            response
        }
        Ok(None) => json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            AppError::NotFound("Web 控制台静态资源不存在，请先构建前端 dist".into()),
        ),
        Err(error) => json_error(StatusCode::INTERNAL_SERVER_ERROR, error),
    }
}
