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
/// (the X button and Alt+F4 both bypass it, leaving the chat page stuck "in
/// call" with nothing visible). Runs through the page bridge: the page-side
/// `callRecover` op decides what (if anything) needs cleaning, so updates
/// never require a rebuild of the host.
pub fn reconcile_after_popup_close(account_id: &str) {
    let Some(bridge) = page_ops::page_for(Some(account_id)) else {
        return;
    };
    tauri::async_runtime::spawn(async move {
        let _ = bridge
            .evaluate(
                "JSON.stringify(function(){ try { if (!window.__zapdudeApiV1 || \
                 typeof window.__zapdudeApiV1.callRecover !== 'function') { return null; } \
                 return window.__zapdudeApiV1.callRecover(); } catch (error) { \
                 return String(error && error.message || error); } }())"
                    .to_string(),
            )
            .await;
    });
}
