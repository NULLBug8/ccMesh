use std::sync::OnceLock;

use tokio::sync::broadcast;

#[derive(Clone, Debug)]
pub struct AdminEvent {
    pub event: String,
    pub payload: String,
}

static EVENT_BUS: OnceLock<broadcast::Sender<AdminEvent>> = OnceLock::new();

fn sender() -> &'static broadcast::Sender<AdminEvent> {
    EVENT_BUS.get_or_init(|| {
        let (tx, _) = broadcast::channel(512);
        tx
    })
}

pub fn subscribe() -> broadcast::Receiver<AdminEvent> {
    sender().subscribe()
}

pub fn emit_json(event: &str, payload: String) {
    let _ = sender().send(AdminEvent {
        event: event.to_string(),
        payload,
    });
}

pub fn emit<T: serde::Serialize>(event: &str, payload: &T) {
    if let Ok(json) = serde_json::to_string(payload) {
        emit_json(event, json);
    }
}
