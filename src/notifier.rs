//! User-facing notifications: the branded in-app toast pipeline.
//!
//! Page-level `Notification` calls on the WhatsApp webviews are captured by
//! the notification hook script and forwarded through the beacon transport
//! into the API event bus (kind "incoming", payload kind "notification").
//! This module turns those events into toasts:
//!   - main window visible: maintain the `toast-overlay` child webview (one
//!     per shell window, anchored to the top-right of the content area)
//!     which renders the branded stack and reports clicks/dismissals; and
//!   - main window hidden (tray): raise a native Windows toast so messages
//!     stay reachable while Zapdude lives in the background.
//!
//! Everything is best-effort: a failing overlay or toast never breaks the
//! shell (events stay available through the API).

use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::webview::WebviewBuilder;
use tauri::{LogicalPosition, LogicalSize, Manager, Wry, WebviewUrl};

/// Overlay geometry: one column of up to four cards hugging the top-right
/// of the content area. The webview only covers the cards' region, so chat
/// clicks stay available everywhere else.
const OVERLAY_WIDTH: f64 = 330.0;
const OVERLAY_HEIGHT: f64 = 420.0;
const OVERLAY_MARGIN_RIGHT: f64 = 14.0;
const OVERLAY_MARGIN_TOP: f64 = 10.0;

/// Queue ceiling: WhatsApp can burst several notifications at once (group
/// chats waking up); keep the stack bounded.
const QUEUE_CAPACITY: usize = 12;

struct ToastItem {
    seq: u64,
    account: String,
    title: String,
    body: String,
    tag: String,
    icon: String,
    at: u64,
}

static TOAST_SEQ: AtomicU64 = AtomicU64::new(0);
static TOAST_QUEUE: LazyLock<Mutex<Vec<ToastItem>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|span| span.as_secs())
        .unwrap_or(0)
}

/// React to every bus event; filters the notification payloads out.
pub fn start(app_handle: tauri::AppHandle<Wry>) {
    let bus = crate::api::server::bus();
    let handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        let mut receiver = bus.subscribe();
        loop {
            let outcome = receiver.recv().await;
            match outcome {
                Ok(event) => {
                    if event.get("kind").and_then(Value::as_str) == Some("incoming")
                        && event
                            .pointer("/data/kind")
                            .and_then(Value::as_str)
                            .map(|kind| kind == "notification")
                            .unwrap_or(false)
                    {
                        route_notification(&handle, &event["data"]);
                    }
                }
                Err(_lagged) => continue,
            }
        }
    });
}

/// Decide the surface for one notification event.
fn route_notification(app_handle: &tauri::AppHandle<Wry>, data: &Value) {
    let item = ToastItem {
        seq: TOAST_SEQ.fetch_add(1, Ordering::Relaxed),
        account: data
            .get("account")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        title: data
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        body: data
            .get("body")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        tag: data
            .get("tag")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        icon: data
            .get("icon")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        at: unix_seconds(),
    };
    let Some(window) = app_handle.get_window("main") else {
        raise_native_toast(&item);
        return;
    };
    let visible = window.is_visible().unwrap_or(false)
        && !window.is_minimized().unwrap_or(true);
    if visible {
        enqueue(item);
        ensure_overlay(app_handle, &window);
    } else {
        raise_native_toast(&item);
    }
}

/// Keep the queue bounded (drop oldest entries past the ceiling).
fn enqueue(item: ToastItem) {
    if let Ok(mut queue) = TOAST_QUEUE.lock() {
        queue.push(item);
        while queue.len() > QUEUE_CAPACITY {
            queue.remove(0);
        }
    }
    schedule_overlay_autoclose();
}

/// Safety net for the overlay life cycle: 20 s after the last enqueue an
/// idle queue closes the overlay even if the page-side release never
/// arrives (crashed page, fetch stall). Cards fade after ~6.5 s, so 20 s
/// of silence always means the visible stack is gone.
fn schedule_overlay_autoclose() {
    let Some(app_handle) = crate::APP_HANDLE.get().cloned() else {
        return;
    };
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        let queue_empty = TOAST_QUEUE
            .lock()
            .map(|queue| queue.is_empty())
            .unwrap_or(true);
        if queue_empty {
            if let Some(webview) = app_handle.get_webview("toast-overlay") {
                let _ = webview.close();
            }
        }
    });
}

fn snapshot(since: u64) -> (u64, Vec<Value>) {
    let queue = match TOAST_QUEUE.lock() {
        Ok(guard) => guard,
        Err(poison) => poison.into_inner(),
    };
    let cursor = TOAST_SEQ.load(Ordering::Relaxed);
    let toasts = queue
        .iter()
        .filter(|item| item.seq > since)
        .map(|item| {
            json!({
                "seq": item.seq,
                "account": item.account,
                "title": item.title,
                "body": item.body,
                "tag": item.tag,
                "icon": item.icon,
                "at": item.at,
            })
        })
        .collect();
    (cursor, toasts)
}

fn find_toast(seq: u64) -> Option<(String, String)> {
    let queue = TOAST_QUEUE.lock().ok()?;
    queue
        .iter()
        .find(|item| item.seq == seq)
        .map(|item| (item.account.clone(), item.tag.clone()))
}

/// Create the overlay webview when missing; always on top of the content
/// area (children added later win the z-order, same lesson as the drawer).
fn ensure_overlay(app_handle: &tauri::AppHandle<Wry>, window: &tauri::window::Window<Wry>) {
    if app_handle.get_webview("toast-overlay").is_some() {
        return;
    }
    let content = match window_inner_content(window) {
        Some(size) => size,
        None => return,
    };
    let (position, size) = overlay_slot(&content);
    let base = crate::window::ui_base_url();
    if let Ok(url) = format!("{base}/toast").parse::<url::Url>() {
        if let Err(error) = window.add_child(
            WebviewBuilder::new("toast-overlay", WebviewUrl::External(url))
                // Floating cards, not a panel: the unused surface stays
                // invisible over WhatsApp.
                .transparent(true),
            position,
            size,
        ) {
            eprintln!("zapdude: toast overlay could not open: {error}");
        }
    }
}

fn window_inner_content(
    window: &tauri::window::Window<Wry>,
) -> Option<LogicalSize<f64>> {
    let scale = window.scale_factor().ok()?;
    let physical = window.inner_size().ok()?;
    let logical = physical.to_logical::<f64>(scale);
    Some(LogicalSize::new(
        (logical.width - crate::window::RAIL_WIDTH).max(1.0),
        logical.height.max(1.0),
    ))
}

/// Where the overlay webview sits, computed from the content size. Exposed
/// for the shell relayout so the watchdog keeps it glued to its corner.
pub fn overlay_slot(content: &LogicalSize<f64>) -> (LogicalPosition<f64>, LogicalSize<f64>) {
    (
        LogicalPosition::new(
            (crate::window::RAIL_WIDTH + content.width - OVERLAY_WIDTH - OVERLAY_MARGIN_RIGHT)
                .max(crate::window::RAIL_WIDTH),
            OVERLAY_MARGIN_TOP,
        ),
        LogicalSize::new(OVERLAY_WIDTH, OVERLAY_HEIGHT),
    )
}

/// Native fallback for a hidden window: a Windows toast raised through the
/// WinRT API with the REGISTERED Zapdude identity. The AUMID is what the
/// Start Menu shortcut carries, so attribution reads "Zapdude" on every
/// build (the notification plugin's default falls back to "powershell"
/// when the exe was launched from a shell - the look the user flagged).
///
/// Clicking the toast brings the main window back via the callback.
#[cfg(windows)]
fn raise_native_toast(item: &ToastItem) {
    use tauri_winrt_notification::{Duration, Toast};

    const REGISTERED_AUMID: &str = "app.zapzap.desktop";

    let contact = if item.title.is_empty() {
        "New message".to_string()
    } else {
        item.title.clone()
    };
    let mut detail = item.body.clone();
    if detail.is_empty() {
        detail = "message".to_string();
    }
    if !item.account.is_empty() {
        detail.push_str(&format!("   [{}]", item.account));
    }

    let icon_source = fetch_icon_temp(item.seq, &item.icon);

    let mut toast = Toast::new(REGISTERED_AUMID)
        .title("Zapdude")
        .text1(&contact)
        .text2(&detail)
        .duration(Duration::Short);
    if let Some(path) = icon_source {
        toast = toast.image(&path, "Contact avatar");
    }
    if let Some(app_handle) = crate::APP_HANDLE.get().cloned() {
        toast = toast.on_activated(move |_action| {
            crate::window::show_main_window(&app_handle);
            Ok(())
        });
    }
    if let Err(error) = toast.show() {
        eprintln!("zapdude: native toast failed: {error}");
    }
}

#[cfg(not(windows))]
fn raise_native_toast(item: &ToastItem) {
    // The native fallback ships for Windows first; other platforms keep
    // notifications flowing through the in-app overlay whenever the window
    // is visible (macOS toasts would raises through NCUserNotificationCenter
    // in a follow-up).
    let _ = item;
}

/// Best-effort avatar fetch for the native toast: tiny cache key by seq in
/// the temp folder; never on the foreground-critical path of messages.
#[cfg(windows)]
fn fetch_icon_temp(seq: u64, icon_url: &str) -> Option<std::path::PathBuf> {
    if !icon_url.starts_with("https://") || icon_url.len() > 2048 {
        return None;
    }
    let Some(record) = crate::SHARED.get() else { return None; };
    let cache_dir = record.settings.data_dir().join("toast-icons");
    if std::fs::create_dir_all(&cache_dir).is_err() {
        return None;
    }
    let extension = if icon_url.to_ascii_lowercase().ends_with("png") { "png" } else { "jpg" };
    let path = cache_dir.join(format!("toast-{seq}.{extension}"));
    let already = std::fs::metadata(&path).map(|meta| meta.len() > 0).unwrap_or(false);
    if !already {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(4)))
            .build()
            .into();
        let Ok(response) = agent.get(icon_url).call() else {
            return None;
        };
        let mut reader = response.into_body().into_reader();
        let mut bytes = Vec::new();
        if std::io::Read::read_to_end(&mut reader, &mut bytes).is_err() {
            return None;
        }
        if bytes.is_empty() || bytes.len() > 1_048_576 {
            return None;
        }
        if std::fs::write(&path, &bytes).is_err() {
            return None;
        }
    }
    Some(path)
}

/// Serve the current queue snapshot for the overlay page (`since` excludes
/// already-shown seqs). Also used by the API docs as a passive listener.
pub fn toasts_snapshot(since: u64) -> Value {
    let (cursor, toasts) = snapshot(since);
    json!({
        "ok": true,
        "cursor": cursor,
        "toasts": toasts,
    })
}

/// Click/dismiss handling from the overlay page. A click focuses the
/// account and lets the page-level onclick handlers route WhatsApp into the
/// right chat; a dismissal frees the card only.
pub fn toast_action(seq: u64, action: &str) {
    if action != "click" {
        return;
    }
    let Some((account, tag)) = find_toast(seq) else {
        return;
    };
    focus_account_and_fire_click(&account, &tag);
}

fn focus_account_and_fire_click(account: &str, tag: &str) {
    // Account focus is async-friendly: reuse the regular open flow on a
    // thread so the HTTP handler returns instantly.
    if let Some(app_handle) = crate::APP_HANDLE.get().cloned() {
        if let Some(shared) = crate::SHARED.get() {
            if let Some(found) = shared
                .settings
                .current_accounts()
                .into_iter()
                .find(|entry| entry.id == account)
            {
                let handle = app_handle.clone();
                std::thread::spawn(move || {
                    if let Err(error) = crate::window::open_account_webview(&handle, &found) {
                        eprintln!("zapdude: toast could not open account '{}': {error}", found.id);
                    }
                });
            }
        }
    }
    // Hand the click to the page so WhatsApp drives its own chat routing.
    if let Some(bridge) = crate::api::page_ops::page_for(Some(account)) {
        let script = format!(
            "JSON.stringify(function(){{ try {{ return (window.__zapdudeFireNotificationClick \
             ? window.__zapdudeFireNotificationClick({tag}) : null); }} catch (error) {{ \
             return {{ ok: false, error: String(error && error.message || error) }}; }} }}())",
            tag = serde_json::to_string(tag).unwrap_or_else(|_| "\"\"".to_string())
        );
        tauri::async_runtime::spawn(async move {
            let _ = bridge.evaluate(script).await;
        });
    }
}

/// Release the overlay: clear the queue and close the webview (fired by the
/// overlay page when its stack is empty and idle).
pub fn toast_release(app_handle: &tauri::AppHandle<Wry>) {
    if let Ok(mut queue) = TOAST_QUEUE.lock() {
        queue.clear();
    }
    if let Some(webview) = app_handle.get_webview("toast-overlay") {
        // Close AFTER the response goes out via a thread sleep beat: closing
        // during the request handling tears the response down sometimes.
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(120)).await;
            let _ = webview.close();
        });
    }
}
