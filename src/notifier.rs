//! User-facing notifications: the UNIFORM native toast pipeline.
//!
//! Page-level `Notification` calls on the WhatsApp webviews are captured by
//! the notification hook script and forwarded through the beacon transport
//! into the API event bus (kind "incoming", payload kind "notification").
//! This module turns those events into Windows toasts - ONE surface for
//! every case (window visible or hidden): the WinRT toast carries the
//! registered Zapdude identity (AUMID), the contact avatar when the
//! payload includes one, and raises the window on activation.
//!
//! History: an earlier iteration showed an in-app branded overlay while
//! the window was visible and a native toast while hidden. Real usage made
//! the mixed surfaces read as "different notifications per account"; the
//! uniform native route was chosen after that feedback (git history keeps
//! the overlay implementation and the `/api/ui/toasts` endpoints).
//!
//! Everything is best-effort: a failing toast never breaks the shell
//! (events stay available through the API).

use serde_json::{json, Value};
use tauri::Wry;

/// React to every bus event; filters the notification payloads out.
pub fn start(app_handle: tauri::AppHandle<Wry>) {
    let bus = crate::api::server::bus();
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
                        raise_native_toast(&event["data"], &app_handle);
                    }
                }
                Err(_lagged) => continue,
            }
        }
    });
}

fn toast_field(data: &Value, key: &str) -> String {
    match data {
        Value::Object(map) => map
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        _ => String::new(),
    }
}

fn json_payload(data: &Value) -> Value {
    json!({
        "account": toast_field(data, "account"),
        "title": toast_field(data, "title"),
        "body": toast_field(data, "body"),
        "tag": toast_field(data, "tag"),
        "icon": toast_field(data, "icon"),
        "at": toast_field(data, "at"),
    })
}

/// Native Windows toast raised through the WinRT API with the REGISTERED
/// Zapdude identity. The AUMID is what the Start Menu shortcut carries, so
/// attribution reads "Zapdude" on every build: the notification plugin's
/// plugin route fell back to the "powershell" attribution for shell
/// launches, which is what the user flagged. Clicking the toast brings the
/// main window back.
#[cfg(windows)]
fn raise_native_toast(data: &Value, app_handle: &tauri::AppHandle<tauri::Wry>) {
    use tauri_winrt_notification::{Duration, Toast};

    const REGISTERED_AUMID: &str = "app.zapzap.desktop";
    let payload = json_payload(data);
    let title = payload
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let body = payload
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let account = payload
        .get("account")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let tag = payload
        .get("tag")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let icon = payload
        .get("icon")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if title.is_empty() && body.is_empty() {
        return;
    }
    let contact = if title.is_empty() {
        "New message".to_string()
    } else {
        title.clone()
    };
    let mut detail = if body.is_empty() {
        "message".to_string()
    } else {
        body.clone()
    };
    if !account.is_empty() {
        detail.push_str(&format!("   [{account}]"));
    }

    let icon_path = fetch_icon_cached(&tag, &icon);

    let mut toast = Toast::new(REGISTERED_AUMID)
        .title("Zapdude")
        .text1(&contact)
        .text2(&detail)
        .duration(Duration::Short);
    if let Some(path) = icon_path {
        toast = toast.image(&path, "Contact avatar");
    }
    let handle_for_click = app_handle.clone();
    toast = toast.on_activated(move |_action| {
        crate::window::show_main_window(&handle_for_click);
        Ok(())
    });
    if let Err(error) = toast.show() {
        eprintln!("zapdude: native toast failed: {error}");
    }
}

#[cfg(not(windows))]
fn raise_native_toast(data: &Value, _app_handle: &tauri::AppHandle<tauri::Wry>) {
    // The native fallback ships for Windows first; other platforms keep
    // notifications flowing through the API event stream (macOS toasts
    // land in a follow-up).
    let _ = data;
}

/// Best-effort avatar fetch for the native toast, cached in the data dir
/// keyed by the notification tag (stable per chat: repeated toasts reuse
/// the cached copy, contacts recycle their own). Handles both http(s)
/// icon URLs and inline data:image payloads (WhatsApp ships those, for
/// instance, for missed-call notifications).
#[cfg(windows)]
fn fetch_icon_cached(tag: &str, icon_url: &str) -> Option<std::path::PathBuf> {
    if icon_url.is_empty() || icon_url.len() > 4096 {
        return None;
    }
    let record = crate::SHARED.get()?;
    let cache_dir = record.settings.data_dir().join("toast-icons");
    if std::fs::create_dir_all(&cache_dir).is_err() {
        return None;
    }
    let key: String = tag
        .chars()
        .map(|character| if character.is_ascii_alphanumeric() { character } else { '_' })
        .collect();
    let filename = format!("avatar-{}.png", if key.is_empty() { "generic".to_string() } else { key });
    let path = cache_dir.join(&filename);
    let cached = std::fs::metadata(&path)
        .map(|meta| meta.len() > 0)
        .unwrap_or(false);
    if cached {
        return Some(path);
    }
    if icon_url.starts_with("data:image/") {
        let encoded = icon_url.split(',').nth(1).unwrap_or_default();
        if encoded.is_empty() {
            return None;
        }
        let bytes = data_url_to_bytes(encoded)?;
        if bytes.is_empty() || bytes.len() > 1_048_576 {
            return None;
        }
        if std::fs::write(&path, &bytes).is_err() {
            return None;
        }
        return Some(path);
    }
    if !icon_url.starts_with("https://") {
        return None;
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(4)))
        .build()
        .into();
    let mut reader = agent.get(icon_url).call().ok()?.into_body().into_reader();
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
    Some(path)
}

#[cfg(windows)]
fn data_url_to_bytes(encoded: &str) -> Option<Vec<u8>> {
    use base64::Engine as _;
    let clean: String = encoded.chars().filter(|ch| !ch.is_whitespace()).collect();
    base64::engine::general_purpose::STANDARD.decode(clean).ok()
}
