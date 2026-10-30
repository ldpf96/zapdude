//! User-facing notifications: the UNIFORM native toast pipeline.
//!
//! Page-level `Notification` calls on the WhatsApp webviews are captured by
//! the notification hook script and forwarded through the beacon transport
//! into the API event bus (kind "incoming", payload kind "notification").
//! This module turns those events into Windows toasts - ONE surface for
//! every case (window visible or hidden). Toast anatomy (the "easy to read,
//! not saturated" bar the user set): contact name as the bold first line,
//! the message below it, the RENAMED account label as a third line only in
//! multi-account shells, and the contact avatar as a small round icon at
//! the LEFT (appLogoOverride). Clicking opens the account that produced
//! the notification.
//!
//! History: an earlier iteration showed an in-app branded overlay while
//! the window was visible and a native toast while hidden. Real usage made
//! the mixed surfaces read as "different notifications per account"; the
//! uniform native route was chosen after that feedback (git history keeps
//! the overlay implementation and the `/api/ui/toasts` endpoints). Later
//! feedback ("la foto sale abajo y gigante", "dice account-1") shrank the
//! toast to the anatomy above: `.image()` placed the avatar as a
//! full-width bottom banner, and the raw `[account-1]` id glued itself to
//! the message tail.
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
    use tauri_winrt_notification::{Duration, IconCrop, Toast};

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
    // Contact (bold first line) + message. No account line in the toast:
    // the user feedback said the account label reads as if the number
    // belonged to the CONTACT, not to the account receiving it, and the
    // WinRT attribution row cannot be changed per toast (it is the AUMID
    // display name). The account story lives in the CLICK-THROUGH below:
    // the toast opens the account that produced it.
    //
    // UNIFORM TOAST LAW (user feedback, 2026-10-30: "todas deben ocupar
    // el mismo tamaño"): WinRT only GROWS a toast when text wraps or an
    // operand is missing, so (a) both text lines are clamped to roughly
    // one rendered line each and (b) the icon slot is ALWAYS filled -
    // contact avatar when the payload carries one, the app icon as the
    // universal fallback when it does not. Shape then stops depending on
    // the event contents entirely.
    let contact = if title.is_empty() {
        clamp_line("New message", 28)
    } else {
        clamp_line(&title, 28)
    };
    let mut toast = Toast::new(REGISTERED_AUMID)
        .title(&contact)
        .duration(Duration::Short);
    if !body.is_empty() {
        toast = toast.text1(&uniform_body(&body));
    }
    let icon_path = fetch_icon_cached(&tag, &icon).or_else(fallback_icon_path);
    if let Some(path) = &icon_path {
        // appLogoOverride + circular crop: the small round avatar at the
        // LEFT of the toast. The previous `.image` call rendered as a
        // full-width banner at the BOTTOM, which read as a giant photo.
        toast = toast.icon(path, IconCrop::Circular, "Contact avatar");
    }
    let account_for_click = account_note(&account);
    let tag_for_click = tag.clone();
    let handle_for_click = app_handle.clone();
    toast = toast.on_activated(move |_action| {
        crate::window::show_main_window(&handle_for_click);
        // Same choreography as tray "Show <account>": the window comes back
        // first, the webview opens on a worker thread (webview creation must
        // never happen on a toast callback thread).
        if let Some(snapshot) = &account_for_click {
            let handle = handle_for_click.clone();
            let snapshot = snapshot.clone();
            let tag = tag_for_click.clone();
            std::thread::spawn(move || {
                let still_there = crate::SHARED
                    .get()
                    .map(|shared| {
                        shared
                            .settings
                            .current_accounts()
                            .iter()
                            .any(|acc| acc.id == snapshot.id && acc.enabled)
                    })
                    .unwrap_or(false);
                if still_there {
                    if let Err(err) =
                        crate::window::open_account_webview(&handle, &snapshot)
                    {
                        eprintln!(
                            "zapdude: toast click could not open account '{}': {}",
                            snapshot.id, err
                        );
                    }
                }
                // THE CHAT, not just the account: the notification hook
                // stores each notification's own onclick handlers, and
                // firing them is what makes WhatsApp open the exact
                // conversation. Without this the click-through landed on
                // the account with WhatsApp's grey "no chat selected"
                // placeholder (the user's report). Best effort + a few
                // retry beats in case the page is still mounting its
                // bridge; empty tags fire all empty-tag instances.
                let account_id = snapshot.id.clone();
                tauri::async_runtime::spawn(async move {
                    let Some(bridge) =
                        crate::api::page_ops::page_for(Some(account_id.as_str()))
                    else {
                        return;
                    };
                    let encoded_tag =
                        serde_json::to_string(&tag).unwrap_or_else(|_| "\"\"".to_string());
                    let script = format!(
                        "JSON.stringify(function(){{ try {{ if (typeof window.__zapdudeFireNotificationClick !== 'function') {{ return null; }} return window.__zapdudeFireNotificationClick({tag}); }} catch (error) {{ return {{ ok:false, error: String(error && error.message || error) }}; }} }}())",
                        tag = encoded_tag
                    );
                    for _attempt in 0..6u32 {
                        match bridge.evaluate(script.clone()).await {
                            Ok(value) => {
                                if value.get("ok").and_then(Value::as_bool) == Some(true) {
                                    break;
                                }
                            }
                            Err(_failure) => {}
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                    }
                });
            });
        }
        Ok(())
    });
    if let Err(error) = toast.show() {
        eprintln!("zapdude: native toast failed: {error}");
    }
}

/// Resolve a notification's account id into the account that the toast
/// CLICK-THROUGH must open (the user-facing account identity lives in the
/// label; the id is only the internal key). `None` when the shell has at
/// most one enabled account (clicking just surfaces the window, that
/// account is the ambient one), when the id is unknown (late toast after
/// account removal), or when not on Windows.
#[cfg(windows)]
fn account_note(account_id: &str) -> Option<crate::config::AccountConfig> {
    if account_id.is_empty() {
        return None;
    }
    let record = crate::SHARED.get()?;
    let enabled: Vec<crate::config::AccountConfig> = record
        .settings
        .current_accounts()
        .into_iter()
        .filter(|acc| acc.enabled)
        .collect();
    match enabled.len() {
        0 | 1 => None,
        _ => enabled.into_iter().find(|acc| acc.id == account_id),
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
    if icon_url.is_empty() {
        return None;
    }
    // Length gate, REVISED (user report: "la llamada perdida no sale con
    // la foto"): CDN links stay short, but INLINE data avatars are
    // whole-image payloads and large by design - WhatsApp ships the
    // missed-call icon as one (real capture: 83,374 chars), which the
    // old flat 4096 cap silently dropped so the toast rendered bare.
    // The cap now only guards against absurd sizes; the decoder's own
    // byte limit (1 MB) is the real memory guard.
    if !icon_url.starts_with("data:") && icon_url.len() > 4096 {
        return None;
    }
    if icon_url.len() > 300_000 {
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

/// The uniform-size law needs the icon slot ALWAYS occupied: several
/// payloads arrive without an avatar (fetch failures, anonymous events).
/// This writes the app icon into the toast-icon cache once and returns
/// its path as the universal fallback (the circular crop renders it
/// round, same geometry as contact avatars).
#[cfg(windows)]
fn fallback_icon_path() -> Option<std::path::PathBuf> {
    const FALLBACK_ICON_BYTES: &[u8] = include_bytes!("../icons/48x48.png");
    let record = crate::SHARED.get()?;
    let cache_dir = record.settings.data_dir().join("toast-icons");
    if std::fs::create_dir_all(&cache_dir).is_err() {
        return None;
    }
    let path = cache_dir.join("zapdude-fallback.png");
    let cached = std::fs::metadata(&path)
        .map(|meta| meta.len() > 0)
        .unwrap_or(false);
    if cached {
        return Some(path);
    }
    if std::fs::write(&path, FALLBACK_ICON_BYTES).is_err() {
        return None;
    }
    Some(path)
}

/// One-line clamp for a toast text row: control characters (the
/// missed-call payload ships \r\n runs) collapse into single spaces and
/// the length caps with an ellipsis, so a row can never wrap into a
/// second line and grow the whole toast.
fn clamp_line(text: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut count = 0;
    let mut space_seen = false;
    for character in text.chars() {
        if character.is_control() || character == ' ' {
            if space_seen {
                continue;
            }
            space_seen = true;
            out.push(' ');
            continue;
        }
        space_seen = false;
        if count >= max_chars {
            out.push('\u{2026}');
            return out;
        }
        out.push(character);
        count += 1;
    }
    out.trim_end().to_string()
}

/// Toast-class text1 renders AT MOST two lines in the ToastGeneric
/// template (inline capture: ~32 characters per line at this width).
/// The user asked for ONE size at birth, so short bodies get an
/// invisible second line (a non-breaking space) instead of letting the
/// payload decide the height: every classic toast = icon + one bold
/// title line + two body lines.
fn uniform_body(text: &str) -> String {
    let collapsed = clamp_line(text, 60);
    if collapsed.chars().count() <= 32 {
        format!("{}\n\u{00A0}", collapsed)
    } else {
        collapsed
    }
}

#[cfg(test)]
mod tests {
    use super::{clamp_line, uniform_body};

    #[test]
    fn clamp_collapses_line_breaks_into_one_space() {
        assert_eq!(clamp_line("Missed \r\n voice  call", 40), "Missed voice call");
    }

    #[test]
    fn clamp_caps_length_with_ellipsis() {
        let long = "abcdefghij".repeat(12);
        assert_eq!(clamp_line(&long, 10), format!("abcdefghij\u{2026}"));
    }

    #[test]
    fn clamp_keeps_short_text_untouched() {
        assert_eq!(clamp_line("Hola", 40), "Hola");
    }

    #[test]
    fn uniform_body_pads_short_payloads_to_two_lines() {
        // Short: NBSP-padded invisible second line.
        let padded = uniform_body("Missed voice call");
        assert!(padded.contains('\n'));
        assert!(padded.ends_with('\u{00A0}'));
    }

    #[test]
    fn uniform_body_leaves_wide_payloads_naked() {
        let wide = "1234567890".repeat(4); // 40 chars: wraps naturally
        assert!(!uniform_body(&wide).contains('\u{00A0}'));
    }
}
