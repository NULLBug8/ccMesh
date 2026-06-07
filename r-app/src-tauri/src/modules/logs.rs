use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::{Context, Layer};

const CAPACITY: usize = 500;
const LOG_EVENT: &str = "log-line";

static BUFFER: OnceLock<Mutex<VecDeque<LogLine>>> = OnceLock::new();
static APP: OnceLock<AppHandle> = OnceLock::new();
static LEVEL: AtomicU8 = AtomicU8::new(2); // 0=trace 1=debug 2=info 3=warn 4=error

/// 结构化字段（tracing 事件的 key=value，message 除外）。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogField {
    pub key: String,
    pub value: String,
}

/// 一条日志（推送给前端 / 环形缓冲）。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub time: String,
    pub level: String,
    /// 事件来源（tracing target，通常为模块路径）。
    pub target: String,
    pub message: String,
    /// 结构化字段（key=value），不含 message。
    pub fields: Vec<LogField>,
}

fn buffer() -> &'static Mutex<VecDeque<LogLine>> {
    BUFFER.get_or_init(|| Mutex::new(VecDeque::with_capacity(CAPACITY)))
}

fn level_num(l: &Level) -> u8 {
    match *l {
        Level::TRACE => 0,
        Level::DEBUG => 1,
        Level::INFO => 2,
        Level::WARN => 3,
        Level::ERROR => 4,
    }
}

pub fn level_from_str(s: &str) -> u8 {
    match s.trim().to_ascii_lowercase().as_str() {
        "trace" => 0,
        "debug" => 1,
        "warn" => 3,
        "error" => 4,
        _ => 2,
    }
}

/// 动态设置捕获/推送级别（不影响控制台 fmt 层）。
pub fn set_level(s: &str) {
    LEVEL.store(level_from_str(s), Ordering::Relaxed);
}

/// 保存 AppHandle 以推送 `log-line` 事件（setup 中调用）。
pub fn set_app_handle(handle: AppHandle) {
    let _ = APP.set(handle);
}

/// 最近日志快照。
pub fn recent() -> Vec<LogLine> {
    buffer().lock().unwrap().iter().cloned().collect()
}

#[derive(Default)]
struct LogVisitor {
    message: String,
    fields: Vec<LogField>,
}
impl Visit for LogVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields.push(LogField {
                key: field.name().to_string(),
                value: value.to_string(),
            });
        }
    }
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            self.fields.push(LogField {
                key: field.name().to_string(),
                value: format!("{value:?}"),
            });
        }
    }
}

/// tracing 捕获层：按 atomic 级别过滤，写入环形缓冲并推送 `log-line` 事件。
pub struct CaptureLayer;

impl<S: Subscriber> Layer<S> for CaptureLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let level = event.metadata().level();
        if level_num(level) < LEVEL.load(Ordering::Relaxed) {
            return;
        }
        let mut visitor = LogVisitor::default();
        event.record(&mut visitor);
        let line = LogLine {
            time: chrono::Local::now().format("%H:%M:%S%.3f").to_string(),
            level: level.to_string(),
            target: event.metadata().target().to_string(),
            message: visitor.message,
            fields: visitor.fields,
        };
        {
            let mut buf = buffer().lock().unwrap();
            if buf.len() >= CAPACITY {
                buf.pop_front();
            }
            buf.push_back(line.clone());
        }
        if let Some(app) = APP.get() {
            let _ = app.emit(LOG_EVENT, line);
        }
    }
}
