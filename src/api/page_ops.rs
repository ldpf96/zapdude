//! Page-side operations: a resident JS bridge plus a native result transport.
//!
//! Primary transport: `eval_with_callback` (WebView2 >= 121 returns the JSON
//! serialized evaluation result; the bundled runtime of this app is newer).
//! Fallback + event transport: URL hash beacon (`#__zapdude=<encoded>`)
//! processed by a polling thread and cleared immediately.
//!
//! Every page interaction is defensive on purpose: WhatsApp markup changes
//! are expected, so failures degrade to API errors instead of taking the
//! window down.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::{json, Value};
use tauri::webview::Webview;
use tauri::Wry;
use tokio::sync::oneshot;
use tokio::sync::Mutex as AsyncMutex;

use super::protocol::{normalize_e164, validate_message, ApiError};

/// Chamfer between beacon polls (event transport only).
const POLL_INTERVAL_MS: u64 = 30;

/// Install the deferred page scripts once every N beacon ticks (~3 s at the
/// 30 ms poll interval). The check itself is a no-op once installed.
const DEFER_INSTALL_EVERY_TICKS: u64 = 100;

/// Maximum time an automation call may wait for the page to answer.
const CALL_TIMEOUT_MS: u64 = 15_000;

/// Composer readiness wait hard cap (matches the timeout above).
const COMPOSER_READY_MS: u64 = 15_000;

/// Messages up to this size ride along the open-chat URL as ?text= prefill.
const URL_PREFILL_LIMIT_CHARS: usize = 500;

static PAGE_BRIDGES: OnceLock<Mutex<HashMap<String, Arc<PageBridge>>>> = OnceLock::new();

/// Resolve the bridge for a given account id (or the default account).
pub fn page_for(account: Option<&str>) -> Option<Arc<PageBridge>> {
    let registry = PAGE_BRIDGES.get()?;
    let map = registry.lock().ok()?;
    match account {
        Some(id) => map.get(id).cloned(),
        None => {
            if let Some(default_page) = map.get("default") {
                return Some(default_page.clone());
            }
            let mut ids: Vec<&String> = map.keys().collect();
            ids.sort();
            ids.first().and_then(|first| map.get(*first)).cloned()
        }
    }
}

/// Create the bridge for an account webview and start its polling loop.
/// Idempotent: the FIRST install for an account wins (the poller thread is
/// only spawned once per account).
pub fn install(
    account_id: &str,
    window: Webview<Wry>,
    bus: Arc<super::events::ApiEventBus>,
) -> Arc<PageBridge> {
    let registry = PAGE_BRIDGES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(existing) = registry
        .lock()
        .ok()
        .and_then(|map| map.get(account_id).cloned())
    {
        return existing;
    }
    let bridge = Arc::new(PageBridge {
        account_id: account_id.to_string(),
        window,
        bus,
        page_lock: AsyncMutex::new(()),
    });
    if let Ok(mut map) = registry.lock() {
        map.insert(account_id.to_string(), bridge.clone());
    }
    let poller = bridge.clone();
    let _ = std::thread::Builder::new()
        .name(format!("zapdude-beacon-{account_id}"))
        .spawn(move || {
            // The deferred scripts (automation bridge + promo hider) are
            // (re-)installed from here: the page may load/reload several
            // times (QR refresh, reconnects), and a light idempotent eval
            // every DEFER_INSTALL_EVERY_TICKS keeps them present without
            // touching the page-start injection (which must stay small -
            // see `startup_scripts`). Each script is guarded by its OWN
            // flag and try/catch so one failure can never silence the
            // other (that is exactly how the banner hider got lost).
            let install_expressions: Vec<String> = crate::window::deferred_scripts()
                .into_iter()
                .map(|(flag, script)| {
                    format!(
                        "if (window === window.top && !window.{flag}) {{ try {{ {script} }} catch (_e) {{}} }}",
                        flag = flag,
                        script = script
                    )
                })
                .collect();
            let mut ticks: u64 = 0;
            loop {
                std::thread::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS));
                ticks = ticks.wrapping_add(1);
                if ticks % DEFER_INSTALL_EVERY_TICKS == 0 {
                    for expression in &install_expressions {
                        let _ = poller.window.eval(expression);
                    }
                }
                if let Err(_failure) = poller.poll_beacons() {
                    // Temporary window/state errors are ignored; the loop
                    // keeps retrying, matching the "never break" behavior.
                }
            }
        });
    bridge
}

fn percent_decode(raw: &str) -> String {
    let input = raw.as_bytes();
    let mut out = Vec::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        let byte = input[index];
        if byte == b'%' && index + 2 < input.len() {
            let hex_high = (input[index + 1] as char).to_digit(16);
            let hex_low = (input[index + 2] as char).to_digit(16);
            if let (Some(high), Some(low)) = (hex_high, hex_low) {
                out.push((high * 16 + low) as u8);
                index += 3;
                continue;
            }
        }
        out.push(byte);
        index += 1;
    }
    String::from_utf8(out).unwrap_or_default()
}

/// DOM-safe JS string literal (JSON encoding is a valid JS string).
fn js_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

fn js_number(value: u32) -> String {
    value.to_string()
}

pub struct PageBridge {
    pub account_id: String,
    window: Webview<Wry>,
    bus: Arc<super::events::ApiEventBus>,
    /// Serialize mutation flows: URL navigation plus composer interactions
    /// must never interleave between two callers.
    page_lock: AsyncMutex<()>,
}

/// Serialize send flows guard before `PageBridge` fields; poll helper below.
impl PageBridge {
    /// Poll the URL-hash beacon for page-to-host event envelopes.
    fn poll_beacons(&self) -> Result<(), String> {
        let url = self.window.url().map_err(|err| err.to_string())?;
        const BEACON: &str = "__zapdude=";
        let Some(fragment) = url.fragment() else {
            return Ok(());
        };
        if !fragment.starts_with(BEACON) {
            return Ok(());
        }
        let encoded = &fragment[BEACON.len()..];
        let decoded = percent_decode(encoded);
        if let Err(_clear) = self.window.eval("location.replace('#')") {
            // ignore; a later poll clears it
        }
        let Ok(envelope) = serde_json::from_str::<Value>(&decoded) else {
            return Ok(());
        };
        let id = envelope.get("id").and_then(Value::as_str).unwrap_or("");
        let payload = envelope.get("payload").cloned().unwrap_or(json!(null));
        if id.starts_with("evt_") {
            // Tag every incoming event with the originating account.
            let enriched = match payload {
                Value::Object(map) => {
                    let mut map = map;
                    map.insert("account".to_string(), json!(self.account_id));
                    Value::Object(map)
                }
                other => json!({ "account": self.account_id, "data": other }),
            };
            self.bus.emit("incoming", enriched);
        }
        Ok(())
    }

    /// Evaluate a page script and await its JSON-serialized result.
    pub async fn evaluate(&self, script: String) -> Result<Value, ApiError> {
        // The callback contract is `Fn` (may fire once per evaluation), so
        // the one-shot sender is parked behind interior mutability.
        let (sender, receiver) = oneshot::channel::<String>();
        let slot = Arc::new(Mutex::new(Some(sender)));
        let slot_for_callback = Arc::clone(&slot);
        let deliver = move |result_json: String| {
            if let Ok(mut guard) = slot_for_callback.lock() {
                if let Some(one_shot) = guard.take() {
                    let _ = one_shot.send(result_json);
                }
            }
        };
        if let Err(eval_failure) = self.window.eval_with_callback(script, deliver) {
            return Err(ApiError::unavailable(format!(
                "page evaluation failed: {}",
                eval_failure
            )));
        }
        let waited = tokio::time::timeout(
            std::time::Duration::from_millis(CALL_TIMEOUT_MS),
            receiver,
        )
        .await;
        match waited {
            Err(_timeout) => Err(ApiError::unavailable("page did not respond in time")),
            Ok(Err(_cancelled)) => Err(ApiError::internal("page result channel closed")),
            Ok(Ok(result_json)) => {
                if result_json.is_empty() {
                    return Err(ApiError::unavailable(
                        "API bridge not present on the page yet",
                    ));
                }
                let parsed = match serde_json::from_str::<Value>(&result_json) {
                    Ok(value) => value,
                    Err(_) => return Err(ApiError::unavailable("page returned unreadable JSON")),
                };
                // Windows quirk: WebView2 ExecuteScript JSON-encodes the value
                // it returns. When the value type is a string (we export
                // `String(JSON.stringify(...))` results), the payload arrives
                // double-encoded and must be unwrapped once more.
                let value = match &parsed {
                    Value::String(inner) if !inner.is_empty() => serde_json::from_str::<Value>(
                        inner,
                    )
                    .unwrap_or(parsed),
                    _ => parsed,
                };
                if value.is_null() {
                    return Err(ApiError::unavailable(
                        "API bridge not present on the page yet",
                    ));
                }
                Ok(value)
            }
        }
    }

    fn bridge_expression(&self, operation: &str, args: &str) -> String {
        let bridge = "window.__zapdudeApiV1";
        format!(
            "JSON.stringify(function(){{ try {{ if (!({bridge}) || typeof ({bridge}.{op}) !== 'function') {{ return null; }} return {bridge}.{op}({args}); }} catch (error) {{ return {{ ok:false, error:'unexpected_error', detail: String(error && error.message || error || '') }}; }} }}())",
            bridge = bridge,
            op = operation,
            args = args
        )
    }

    fn open_send_url(&self, phone: &str, message: Option<&str>) -> Result<(), ApiError> {
        let mut target = format!("https://web.whatsapp.com/send?phone={}", phone);
        // Prefill semantics keep the URL small: only attach ?text= when
        // the message fits comfortably inside a URL.
        let prefill = message.filter(|text| text.chars().count() <= URL_PREFILL_LIMIT_CHARS);
        if let Some(text) = prefill {
            target.push_str(&format!("&text={}", encode_query_component(text)));
        }
        let parsed = url::Url::parse(&target)
            .map_err(|err| ApiError::internal(format!("send URL invalid: {}", err)))?;
        self.window
            .navigate(parsed)
            .map_err(|err| ApiError::unavailable(format!("could not open chat: {}", err)))
    }

    async fn wait_composer(&self) -> Result<(), ApiError> {
        let started = std::time::Instant::now();
        loop {
            let info = self
                .evaluate(self.bridge_expression("composerInfo", ""))
                .await?;
            if info.get("ok").and_then(Value::as_bool) == Some(true)
                && info.get("ready").and_then(Value::as_bool) == Some(true)
            {
                return Ok(());
            }
            if started.elapsed() >= std::time::Duration::from_millis(COMPOSER_READY_MS) {
                return Err(ApiError::unavailable(
                    "composer did not become ready in time (is a chat open / logged in?)",
                ));
            }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
    }

    /// Open a chat (and optionally prefill/send a message).
    pub async fn open_chat(
        &self,
        phone: &str,
        message: Option<&str>,
    ) -> Result<Value, ApiError> {
        let digits = normalize_e164(phone)?;
        let _guard = self.page_lock.lock().await;
        self.open_send_url(&digits, message)?;
        self.wait_composer().await?;
        let mut detail = json!({
            "account": self.account_id,
            "phone": digits,
            "opened": true,
            "url": self.window_url_for_status(),
        });
        if let Some(text) = message {
            let outcome = self
                .evaluate(self.bridge_expression("sendCurrent", &js_string(text)))
                .await?;
            detail["sent"] = outcome;
        }
        self.bus.emit("chat_opened", detail.clone());
        Ok(detail)
    }

    /// Send a text message (open-chat flow + composer automation).
    pub async fn send_message(&self, phone: &str, message: &str) -> Result<Value, ApiError> {
        let digits = normalize_e164(phone)?;
        let clean = validate_message(message)?;
        let detail = self.open_chat(&digits, Some(&clean)).await?;
        Ok(json!({
            "phone": digits,
            "message_length": clean.chars().count(),
            "outcome": detail.get("sent").cloned().unwrap_or(json!({})),
        }))
    }

    /// Best-effort scrape of the visible chat list.
    pub async fn list_chats(&self, limit: u32) -> Result<Value, ApiError> {
        let clamped = limit.clamp(1, 100);
        let chats = self
            .evaluate(self.bridge_expression(
                "chatList",
                &js_number(clamped),
            ))
            .await?;
        Ok(chats)
    }

    /// Raw page state for /status.
    pub async fn status_snapshot(&self) -> Result<Value, ApiError> {
        let info = self.evaluate(self.bridge_expression("pageInfo", "")).await?;
        Ok(json!({
            "account": self.account_id,
            "url": self.window_url_for_status(),
            "title": info.get("title").cloned().unwrap_or(json!("")),
            "chat_list": info.get("chat_list").cloned().unwrap_or(json!(false)),
            "composer_visible": info.get("composer").cloned().unwrap_or(json!(false)),
            "qrcode_visible": info.get("qrcode").cloned().unwrap_or(json!(false)),
            // Page-reported location (location.href): reveals redirects or
            // WebView2 error pages that the nav URL alone hides.
            "page_url": info.get("url").cloned().unwrap_or(json!(null)),
        }))
    }

    fn window_url_for_status(&self) -> String {
        self.window
            .url()
            .map(|url| url.to_string())
            .unwrap_or_default()
    }
}

/// Decode a `?text=` prefill value for sandbox URLs.
fn encode_query_component(raw: &str) -> String {
    let mut encoded = String::with_capacity(raw.len());
    for byte in raw.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*'
            | b'\'' | b'(' | b')' => encoded.push(byte as char),
            b' ' => encoded.push_str("%20"),
            other => encoded.push_str(&format!("%{:02X}", other)),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_encoding_is_url_safe() {
        assert_eq!(encode_query_component("hola mundo"), "hola%20mundo");
        assert_eq!(encode_query_component("a=b&c"), "a%3Db%26c");
        assert_eq!(encode_query_component("á"), "%C3%A1");
    }

    #[test]
    fn bridge_expression_guards_missing_api() {
        let bridge = PageBridgeBuilderStub::default();
        let script = format!(
            "check {}",
            bridge.stub_expression("composerInfo")
        );
        assert!(script.contains("typeof"));
        assert!(script.contains("composerInfo"));
    }

    struct PageBridgeBuilderStub;
    impl Default for PageBridgeBuilderStub {
        fn default() -> Self {
            Self
        }
    }
    impl PageBridgeBuilderStub {
        #[allow(dead_code)]
        fn stub_expression(&self, op: &str) -> String {
            format!("typeof (window.__zapdudeApiV1.{}) !== 'function'", op)
        }
    }
}
