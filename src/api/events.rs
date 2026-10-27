//! Event bus for the automation API: ring buffer + SSE fan-out + optional
//! HMAC-signed webhook delivery (thread-backed, non-blocking).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

/// Keep the most recent events for late SSE clients and /api/events polling.
const RING_CAPACITY: usize = 200;

pub struct ApiEventBus {
    ring: Mutex<VecDeque<Value>>,
    tx: tokio::sync::broadcast::Sender<Value>,
    webhook: Mutex<WebhookConfig>,
}

#[derive(Clone, Default)]
struct WebhookConfig {
    url: String,
    secret: String,
}

impl ApiEventBus {
    pub fn new() -> Arc<Self> {
        let (tx, _rx) = tokio::sync::broadcast::channel(256);
        Arc::new(Self {
            ring: Mutex::new(VecDeque::with_capacity(RING_CAPACITY)),
            tx,
            webhook: Mutex::new(WebhookConfig::default()),
        })
    }

    /// Current unix milliseconds.
    fn unix_millis() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis())
            .unwrap_or(0)
    }

    /// Build an event envelope: id, kind, at, data.
    pub fn to_event(kind: &str, data: Value) -> Value {
        let id_self = Self::unix_millis();
        json!({
            "id": format!("evt_{}_{}", id_self, std::process::id()),
            "kind": kind,
            "at": id_self / 1000,
            "data": data,
        })
    }

    /// Publish to the ring buffer, live subscribers and (optional) webhook.
    pub fn emit(&self, kind: &str, data: Value) -> Value {
        let event = Self::to_event(kind, data);
        {
            let mut ring = match self.ring.lock() {
                Ok(guard) => guard,
                Err(poison) => poison.into_inner(),
            };
            if ring.len() >= RING_CAPACITY {
                ring.pop_front();
            }
            ring.push_back(event.clone());
        }
        let _ = self.tx.send(event.clone());
        self.maybe_send_webhook(&event);
        event
    }

    fn maybe_send_webhook(&self, event: &Value) {
        let config = match self.webhook.lock() {
            Ok(guard) => guard.clone(),
            Err(poison) => poison.into_inner().clone(),
        };
        if config.url.is_empty() {
            return;
        }
        let payload = event.to_string();
        let secret = config.secret;
        let signature = crate::api::protocol::sign_payload(&secret, payload.as_bytes());
        std::thread::spawn(move || {
            deliver_webhook(&config.url, &payload, &signature);
        });
    }

    /// Configure (or clear) the webhook receiver.
    pub fn set_webhook(&self, url: &str, secret: &str) {
        let mut guard = match self.webhook.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        };
        guard.url = url.to_string();
        guard.secret = secret.to_string();
    }

    pub fn webhook_config(&self) -> (String, String) {
        match self.webhook.lock() {
            Ok(guard) => (guard.url.clone(), guard.secret.clone()),
            Err(poison) => {
                let config = poison.into_inner().clone();
                (config.url, config.secret)
            }
        }
    }

    /// Events after `since` (exclusive by string id), or the whole ring when
    /// `since` is None.
    pub fn since(&self, since: Option<&str>) -> Vec<Value> {
        let ring = match self.ring.lock() {
            Ok(guard) => guard,
            Err(poison) => poison.into_inner(),
        };
        match since {
            None => ring.iter().cloned().collect(),
            Some(marker) => {
                let position = ring
                    .iter()
                    .position(|event| event.get("id").and_then(|id| id.as_str()) == Some(marker));
                match position {
                    Some(index) => ring.iter().skip(index + 1).cloned().collect(),
                    None => ring.iter().cloned().collect(),
                }
            }
        }
    }

    /// Subscribe to live events (for SSE).
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Value> {
        self.tx.subscribe()
    }
}

fn deliver_webhook(url: &str, body: &str, signature: &str) {
    // Blocking, tiny HTTPS client; failures are swallowed by design: the
    // webhook is best-effort and must never take the app down.
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(5)))
        .build()
        .into();
    let outcome = agent
        .post(url)
        .header("Content-Type", "application/json")
        .header("X-ZapZap-Signature", signature)
        .header("User-Agent", "ZapZap-API/1")
        .send(body.to_string());
    if let Err(_failure) = outcome {
        // webhook delivery failure ignored (receiver down / bad TLS / 5xx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_keeps_latest_events() {
        let bus = ApiEventBus::new();
        let event = bus.emit("test", json!({ "n": 1 }));
        assert_eq!(event["kind"], json!("test"));
        let history = bus.since(None);
        assert_eq!(history.last(), Some(&event));
        let after = bus.since(event["id"].as_str());
        assert!(after.is_empty());
    }

    #[test]
    fn set_and_read_webhook() {
        let bus = ApiEventBus::new();
        bus.set_webhook("https://example.com/hook", "s3cret");
        let (url, secret) = bus.webhook_config();
        assert_eq!(url, "https://example.com/hook");
        assert_eq!(secret, "s3cret");
    }
}
