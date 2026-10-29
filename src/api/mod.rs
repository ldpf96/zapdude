//! Local automation API for Zapdude.
//!
//! A loopback HTTP server usable by humans or agents: send messages, open
//! chats, list chats, and stream WhatsApp Web events over SSE or a webhook.
//! Disabled by default; enabled through the persisted `config.json`.

pub mod events;
pub mod page_ops;
pub mod protocol;
pub mod server;

/// Best-effort reconciliation of an account's WhatsApp page after its call
/// popup window vanished without going through WhatsApp's own end-call flow
/// (X button / Alt+F4 bypass it: the call keeps "burning" and the pill
/// stays on screen with nothing reachable). Sequence: click the leftover
/// pill; wait a beat; if a pill still matches, reload the page as the
/// failsafe (session-safe reload, audio dies, pill clears).
pub fn reconcile_after_popup_close(account_id: &str) {
    let account_id = account_id.to_string();
    let Some(bridge) = page_ops::page_for(Some(account_id.as_str())) else {
        return;
    };
    let click_script = String::from(
        "JSON.stringify(function(){ try { if (!window.__zapdudeApiV1 || \
         typeof window.__zapdudeApiV1.callRecover !== 'function') { return null; } \
         return window.__zapdudeApiV1.callRecover(false); } catch (error) { \
         return String(error && error.message || error); } }())",
    );
    let probe_script = String::from(
        "JSON.stringify(function(){ try { if (!window.__zapdudeApiV1 || \
         typeof window.__zapdudeApiV1.callRecover !== 'function') { return null; } \
         return window.__zapdudeApiV1.callRecover(true); } catch (error) { \
         return String(error && error.message || error); } }())",
    );
    let reload_script = String::from("location.reload(); void 0");
    tauri::async_runtime::spawn(async move {
        let _ = bridge.evaluate(click_script).await;
        tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
        match bridge.evaluate(probe_script).await {
            Ok(probe) => {
                let stuck = probe
                    .get("in_call")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                if stuck {
                    eprintln!(
                        "zapdude: account '{}' still shows a stuck call pill; reloading the page",
                        account_id
                    );
                    let _ = bridge.evaluate(reload_script).await;
                }
            }
            Err(_probe_failure) => {}
        }
    });
}
