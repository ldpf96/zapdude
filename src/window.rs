//! Zapdude application shell: ONE window that hosts every webview.
//!
//! Layout (single-window design inspired by the client whose idea
//! motivated this project - ZapZap by Rafael Tosta; the code is original):
//!   - A narrow left rail (label `rail`) - the Zapdude logo, one circular
//!     avatar per account (click to switch), a "+" button to add an account
//!     in one click, and settings/about buttons at the bottom.
//!   - One WhatsApp Web webview per account (label `account-<id>`), each with
//!     an isolated WebView2 profile so every account stays logged in.
//!   - A welcome webview (label `welcome`) used as a branded backdrop.
//!
//! Visibility model: the active content webview sits next to the rail;
//! every other content webview is PARKED just outside the visible client
//! area (still rendered, never `hide()`-d). WebView2 blanks webviews that
//! are created hidden and shown later - parking keeps them alive and
//! paintable, so switching accounts never shows a gray page.
//!
//! The rail/welcome/settings pages are SERVED BY THE LOOPBACK API SERVER
//! (same origin), so their `fetch` calls always work - no IPC, no CORS,
//! no file:// resolution problems. Multi-webview inside one window needs
//! the Tauri `unstable` feature; webview creation happens from non-main
//! threads (setup, async commands or spawned threads) so the WebView2
//! main-thread deadlock does not apply.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::webview::{PermissionKind, PermissionResponse, WebviewBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::window::Window;
use tauri::{App, AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl, Wry};

/// Desktop identity chosen to match the Chrome 156 stable channel, aligned
/// with the JS client-hints patch so voice/video call entry points stay
/// available on WhatsApp Web (the WebView2 runtime still reports its own
/// Sec-CH-UA brand on the HTTP layer; the JS-facing identity is what the
/// webapp feature gates read).
const USER_AGENT_DESKTOP: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
     Chrome/156.0.8078.49 Safari/537.36";

const WHATSAPP_URL: &str = "https://web.whatsapp.com";

/// Facebook chat embedding. Messenger (the standalone chat surface) keeps
/// the page anchored on the conversation UI only - never the full Facebook
/// feed panel. Login happens INSIDE this view, exactly like WhatsApp's QR:
/// email + password + 2FA, as the user would on facebook.com.
const FACEBOOK_URL: &str = "https://www.messenger.com";

/// The page each account's webview loads, by service. The page IS the
/// integration: Zapdude never replaces the chat's own client, only drives
/// it from injected page bridges (the "adapt to the page" contract).
pub fn account_service_url(service: &str) -> &'static str {
    if service == crate::config::SERVICE_FACEBOOK {
        FACEBOOK_URL
    } else {
        WHATSAPP_URL
    }
}

/// Renderer args. WARNING (empirical, WebView2 + wry 0.57): passing ANY
/// `additional_browser_args` on child webviews combined with the Chrome
/// user-agent breaks WebView2 composition - the page loads and JS runs but
/// the compositor renders a flat gray surface. The same flags applied by
/// wry internally (no custom args) work fine, so we do NOT pass custom
/// browser args in production; this hook remains for debugging only.
#[cfg(windows)]
const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --disable-background-timer-throttling --disable-renderer-backgrounding --disable-backgrounding-occluded-windows";

const CLIENT_HINTS_FIX_SCRIPT: &str = include_str!("../scripts/client_hints_fix.js");
const API_PAGE_BRIDGE_SCRIPT: &str = include_str!("../scripts/api_page_bridge.js");
const FACEBOOK_BRIDGE_SCRIPT: &str = include_str!("../scripts/facebook_bridge.js");
const HIDE_DOWNLOAD_BANNERS_SCRIPT: &str = include_str!("../scripts/hide_download_banners.js");
const CALL_ANCHOR_SCRIPT: &str = include_str!("../scripts/call_anchor.js");

const UNREAD_WATCHER_SCRIPT: &str = include_str!("../scripts/unread_watcher.js");
const CALL_STYLE_SCRIPT: &str = include_str!("../scripts/call_style.js");
const NOTIFICATION_HOOK_SCRIPT: &str = include_str!("../scripts/notification_hook.js");
const SW_GUARD_SCRIPT: &str = include_str!("../scripts/sw_guard.js");

/// Logical width of the left rail. The content area gets the rest.
pub const RAIL_WIDTH: f64 = 76.0;

/// X position where inactive content webviews park: FULL size, completely
/// outside the visible client area. Parking by POSITION (never by size and
/// never by hide()) keeps WhatsApp's layout warm - switching accounts is a
/// position-only move, so there is no re-render and the switch is instant.
const PARKED_X: f64 = -24000.0;

/// Which content webview is currently in the content slot ("welcome" or an
/// account id). Everything else is parked.
static ACTIVE: Mutex<Option<String>> = Mutex::new(None);

/// Whether the options drawer is over the content area.
static DRAWER_OPEN: Mutex<bool> = Mutex::new(false);

/// Account webviews use ids, not labels, for their webview identity.
pub fn account_webview_label(account_id: &str) -> String {
    format!("account-{account_id}")
}

/// Toggle the options screen (brand-logo click): a FULL-SCREEN panel over
/// the content area with the API settings, agent docs and About - in the
/// SAME window (no auxiliary window). Rebuilt on every open so it always
/// renders on top and shows the current state.
pub fn toggle_drawer(app: &AppHandle) -> tauri::Result<()> {
    let open_now = DRAWER_OPEN.lock().map(|guard| *guard).unwrap_or(false);
    let open = !open_now;
    if let Ok(mut guard) = DRAWER_OPEN.lock() {
        *guard = open;
    }
    let Some(window) = app.get_window("main") else {
        return Ok(());
    };
    if open {
        let content = content_size(&window)?;
        // The click-open strip must not hover over the options drawer:
        // opening the drawer dismisses it (fresh state on each side).
        hide_picker(app);
        let base = ui_base_url();
        let url: url::Url = format!("{base}/drawer").parse().expect("drawer URL must parse");
        // Rebuild from scratch: a webview created before an account would
        // sit BELOW it in z-order and appear "gone" once that account
        // covers the content area. Closing + re-adding always lands on top.
        if let Some(existing) = app.get_webview("drawer") {
            let _ = existing.close();
        }
        window.add_child(
            WebviewBuilder::new("drawer", WebviewUrl::External(url)),
            LogicalPosition::new(RAIL_WIDTH, 0.0_f64),
            LogicalSize::new(content.width, content.height),
        )?;
    }
    // Park/show the drawer RIGHT NOW: closing used to wait for the 2s
    // geometry watchdog (set_focus does not fire a Focused event when the
    // window already has focus), which read as "the X takes forever".
    relayout(&window);
    let _ = window.set_focus();
    Ok(())
}

/// The rail "+" hover menu: opens a horizontal service strip right of the
/// rail (drawer-mechanics child webview). BORN at full content size (paint
/// law) and immediately re-anchored to the strip slot beside the "+" tile
/// by relayout; recreated per open so z-order always lands on top (drawer
/// parity); destroyed on close. State + hover coordination:
///   PICKER_OPEN           strip visible over the content slot?
///   PICKER_TOUCH          last time the cursor touched the strip page
///   show/toggle/hide      explicit endpoints (toggle from + click)
///   touch                 the strip reports cursor presence (cancels the
///                         soft-close the rail schedules when the cursor
///                         leaves the + toward the strip)
///   soft_close            rail cursor left the +: hide after a short
///                         grace UNLESS the strip reports a touch first
///                         (the cursor crossed the 4px gap into the menu).
pub const SERVICE_PICKER_WINDOW: &str = "service-picker";

static PICKER_OPEN: Mutex<bool> = Mutex::new(false);

fn picker_open_now() -> bool {
    PICKER_OPEN.lock().map(|guard| *guard).unwrap_or(false)
}

static PICKER_TOUCH: Mutex<Option<std::time::Instant>> = Mutex::new(None);

/// Open (idempotent) the strip. Recreates the webview so it always lands
/// ABOVE the account views, then lets relayout shrink it into the strip
/// beside the "+" (born at FULL content size for the compositor).
pub fn show_picker(app: &AppHandle) {
    if picker_open_now() {
        return;
    }
    let Some(window) = app.get_window("main") else {
        return;
    };
    if let Ok(mut open) = PICKER_OPEN.lock() {
        *open = true;
    }
    if let Some(existing) = app.get_webview(SERVICE_PICKER_WINDOW) {
        let _ = existing.close();
    }
    if let Some(window) = app.get_window("main") {
        if let Ok(content) = content_size(&window) {
            let base = ui_base_url();
            if let Ok(url) = format!("{base}/picker").parse::<url::Url>() {
                let _ = window.add_child(
                    WebviewBuilder::new(SERVICE_PICKER_WINDOW, WebviewUrl::External(url))
                        .transparent(true),
                    LogicalPosition::new(RAIL_WIDTH, 0.0_f64),
                    LogicalSize::new(content.width, content.height),
                );
            }
        }
    }
    if let Some(window) = app.get_window("main") {
        relayout(&window);
    }
}

/// Toggle for the + click: hover already opens, so click closes when open.
pub fn toggle_picker(app: &AppHandle) {
    if picker_open_now() {
        hide_picker(app);
    } else {
        show_picker(app);
    }
}

/// Close the strip (pick, strip mouseleave, Esc, outside click).
pub fn hide_picker(app: &AppHandle) {
    let was_open = picker_open_now();
    if let Ok(mut open) = PICKER_OPEN.lock() {
        *open = false;
    }
    if !was_open {
        return;
    }
    if let Some(webview) = app.get_webview(SERVICE_PICKER_WINDOW) {
        let _ = webview.close();
    }
    if let Some(window) = app.get_window("main") {
        relayout(&window);
    }
}

/// The strip page's cursor entered: cancel any pending soft-close.
pub fn picker_touch() {
    if let Ok(mut touch) = PICKER_TOUCH.lock() {
        *touch = Some(std::time::Instant::now());
    }
}

/// The rail's cursor left the +: schedule a soft close. With CLICK-open the
/// cursor leaves the + the moment it moves anywhere else, so the grace is
/// generous (1.2s) - the strip's touch pings cancel it when the cursor is
/// heading into the menu, and picking a service closes it anyway.
pub fn soft_close_picker(app: &AppHandle) {
    if !picker_open_now() {
        return;
    }
    let scheduled = std::time::Instant::now();
    let handle = app.clone();
    std::thread::Builder::new()
        .name("zapdude-picker-softclose".into())
        .spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1200));
            let touched_recently = PICKER_TOUCH
                .lock()
                .ok()
                .and_then(|touch| *touch)
                .map(|touch_at| touch_at > scheduled)
                .unwrap_or(false);
            if picker_open_now() && !touched_recently {
                hide_picker(&handle);
            }
        })
        .ok();
}

/// The label of the content webview currently shown (None = rail only).
pub fn active_content() -> Option<String> {
    ACTIVE.lock().ok().and_then(|guard| guard.clone())
}

/// Base URL of the loopback server that serves the shell UI + API.
pub fn ui_base_url() -> String {
    let port = crate::SHARED
        .get()
        .map(|shared| shared.settings.current().port)
        .unwrap_or(crate::config::DEFAULT_API_PORT);
    format!("http://127.0.0.1:{port}")
}

/// Scripts injected at document-start (BEFORE any page script runs).
/// Keep this list MINIMAL: a large script injected via
/// AddScriptToExecuteOnDocumentCreated on child webviews reliably breaks
/// WebView2 painting (the page loads and JS runs, but the compositor shows
/// a flat gray surface). Only the JS identity fix needs document-start.
fn startup_scripts() -> Vec<String> {
    let opts = std::env::var("ZAPDUKE_WEBVIEW_OPTS").unwrap_or_else(|_| "all".to_string());
    let want = |name: &str| {
        !opts.contains("scripts1") && !opts.contains("scripts2") && !opts.contains("scripts3")
            || opts.contains(name)
    };
    let mut scripts = Vec::new();
    if want("scripts1") {
        scripts.push(format!(
            "if (window === window.top) {{ {} }}",
            CLIENT_HINTS_FIX_SCRIPT
        ));
    }
    scripts
}

/// Scripts installed shortly AFTER the document loads (via eval from the
/// beacon poller) as `(idempotency-flag, script)` pairs. Each script gets
/// its OWN flag and its OWN try/catch: a shared wrapper meant one failing
/// script silently disabled the other forever (the download-banner hider
/// was lost this way whenever the bridge had already set its flag).
/// The automation bridge and the cosmetic promo hider do not need to run
/// before WhatsApp's own scripts, and deferring them keeps the
/// document-start injection small enough that WebView2 keeps painting.
///
/// Per service: WhatsApp keeps its historical set UNCHANGED (it must never
/// regress); Facebook gets the SHARED notification capture plus its own
/// adaptive page bridge under the same `__zapdudeApiV1` contract. The
/// WhatsApp selectors and promos hider never run on Facebook pages - the
/// other service's DOM means nothing to them, and anything we do not
/// positively recognize must not touch the page.
pub fn deferred_scripts(service: Option<&str>) -> Vec<(&'static str, &'static str)> {
    let opts = std::env::var("ZAPDUKE_WEBVIEW_OPTS").unwrap_or_else(|_| "all".to_string());
    let want = |name: &str| {
        !opts.contains("scripts1") && !opts.contains("scripts2") && !opts.contains("scripts3")
            || opts.contains(name)
    };
    let is_facebook = service == Some(crate::config::SERVICE_FACEBOOK);
    let mut pairs = Vec::new();
    if want("scripts2") {
        // SAME bridge name `__zapdudeApiV1` on every service: the host's
        // bridge_expression guards stay service-agnostic, and an unknown op
        // answers `unknown_operation` instead of breaking the endpoint.
        pairs.push((
            "__zapdudeApiV1",
            if is_facebook {
                FACEBOOK_BRIDGE_SCRIPT
            } else {
                API_PAGE_BRIDGE_SCRIPT
            },
        ));
    }
    if !is_facebook && want("scripts3") {
        pairs.push(("__zapdudeHidePromos", HIDE_DOWNLOAD_BANNERS_SCRIPT));
        // The service-worker guard (square toast fix - sw.js raises its
        // own un-stylable OS notification from its realm) shares the
        // WhatsApp-only group: the Messenger surface has no sw.js of
        // this kind.
        pairs.push(("__zapdudeSwGuardV1", SW_GUARD_SCRIPT));
    }
    // The notification capture installs once the bridge it leans on is
    // present (fallback beacon transport works even alone, but the bridge
    // keeps the two order-stable). own flag, own try/catch: a failure can
    // never silence the other scripts (the banner-hider lesson). SHARED by
    // every service: it is generic page-level Notification capture, the
    // one choke point through which the embedded chat surfaces announce
    // incoming messages.
    if want("scripts2") {
        pairs.push(("__zapdudeNotifyV1", NOTIFICATION_HOOK_SCRIPT));
        // The unread badge source: every account page (both services)
        // samples its own document title (WhatsApp + Messenger both
        // publish "(n) ..." natively) and keeps the host's per-account
        // store fresh. Cheap, drift-proof, and the rail badge + the
        // drawer toggle consume the same data.
        pairs.push(("__zapdudeUnreadV1", UNREAD_WATCHER_SCRIPT));
    }
    if !is_facebook && want("scripts3") {
        pairs.push(("__zapdudeCallAnchor", CALL_ANCHOR_SCRIPT));
        pairs.push(("__zapdudeCallStyleV1", CALL_STYLE_SCRIPT));
    }
    pairs
}

/// Permissions WhatsApp Web needs for calls, screen sharing and clipboard.
fn permission_response(kind: PermissionKind) -> PermissionResponse {
    match kind {
        PermissionKind::Microphone
        | PermissionKind::Camera
        | PermissionKind::DisplayCapture
        | PermissionKind::Notifications
        | PermissionKind::ClipboardRead
        | PermissionKind::PointerLock => PermissionResponse::Allow,
        _ => PermissionResponse::Default,
    }
}

/// Sequence for popup window labels (pop-out calls, etc.).
static POPUP_SEQ: AtomicU32 = AtomicU32::new(0);

/// Registry of LIVE popup windows: label -> metadata. Entries let the close
/// handler know which account the popup belonged to (the account webview
/// must reconcile its call state when its popup vanishes) and serve the
/// debug harness listing.
struct PopupEntry {
    account: String,
    label: String,
    url: String,
}

static POPUP_INDEX: std::sync::LazyLock<Mutex<HashMap<String, PopupEntry>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Open a POP-OUT window (WhatsApp call windows, media popups): a real
/// native window the user can move anywhere, sharing the ACCOUNT's
/// WebView2 environment (same session/cookies) - without this, WebView2
/// denies every window.open and WhatsApp shows "modify your browser to
/// allow popups".
#[cfg(windows)]
fn popup_window(
    account_id: &str,
    url: url::Url,
    features: &tauri::webview::NewWindowFeatures,
) -> tauri::webview::NewWindowResponse<Wry> {
    let Some(handle) = crate::APP_HANDLE.get().cloned() else {
        return tauri::webview::NewWindowResponse::Deny;
    };
    let seq = POPUP_SEQ.fetch_add(1, Ordering::Relaxed);
    let label = format!("popup-{account_id}-{seq}");
    let (width, height) = features
        .size()
        .map(|size| (size.width.max(280.0), size.height.max(240.0)))
        .unwrap_or((480.0_f64, 640.0_f64));
    // The opener's environment already carries this account's user-data
    // folder - sharing it is REQUIRED on Windows (a different environment
    // with the same data folder fails to create) and is what gives the
    // popup the account's session.
    let environment = features.opener().environment.clone();
    let built = tauri::webview::WebviewWindowBuilder::new(&handle, &label, WebviewUrl::External(url.clone()))
        .title("Zapdude call")
        .inner_size(width, height)
        .min_inner_size(280.0_f64, 240.0_f64)
        .resizable(true)
        .user_agent(USER_AGENT_DESKTOP)
        .initialization_script(format!(
            "if (window === window.top) {{ {} }}",
            CLIENT_HINTS_FIX_SCRIPT
        ))
        .on_permission_request(|_webview, kind| permission_response(kind))
        .with_environment(environment)
        .build();
    match built {
        Ok(window) => {
            // Popups are real windows: their caption bar must follow the
            // shell theme too (re-applying everywhere is idempotent).
            apply_theme_everywhere(&handle);
            let _ = window.set_focus();
            // Popup lifecycle tracking: WhatsApp's call popup going away (X
            // button, Alt+F4, or WhatsApp calling window.close()) is a
            // decision the ACCOUNT webview never hears about - the chat page
            // keeps the "in call" state with nothing visible (the stuck
            // state seen live). Register the popup, emit the lifecycle
            // events for API consumers, and prompt the account page to
            // reconcile when the popup vanishes.
            let entry = PopupEntry {
                account: account_id.to_string(),
                label: label.clone(),
                url: url.to_string(),
            };
            if let Ok(mut map) = POPUP_INDEX.lock() {
                map.insert(label.clone(), entry);
            }
            crate::api::server::bus().emit(
                "popup_opened",
                serde_json::json!({
                    "account": account_id,
                    "label": label,
                    "url": url.to_string(),
                }),
            );            let close_label = label.clone();
            window.on_window_event(move |event| {
                if !matches!(event, tauri::WindowEvent::Destroyed) {
                    return;
                }
                let entry = POPUP_INDEX
                    .lock()
                    .ok()
                    .and_then(|mut map| map.remove(&close_label));
                if let Some(entry) = entry {
                    crate::api::server::bus().emit(
                        "popup_closed",
                        serde_json::json!({
                            "account": entry.account,
                            "label": entry.label,
                            "url": entry.url,
                        }),
                    );
                    crate::api::reconcile_after_popup_close(&entry.account);
                }
            });
            tauri::webview::NewWindowResponse::Create { window }
        }
        Err(err) => {
            eprintln!("zapdude: popup window '{label}' failed: {err}");
            tauri::webview::NewWindowResponse::Deny
        }
    }
}

/// Builder for one WhatsApp Web account webview (identity + bridge).
///
/// DEBUG BISECTION: ZAPDUKE_WEBVIEW_OPTS (comma list) selects which options
/// are applied, to isolate which one breaks WebView2 painting:
///   ua | scripts | data | args | perms  (default: all)
fn account_webview_builder(account: &crate::config::AccountConfig) -> WebviewBuilder<Wry> {
    let opts = std::env::var("ZAPDUKE_WEBVIEW_OPTS").unwrap_or_else(|_| "all".to_string());
    let has = |name: &str| {
        // 'args' is debug-only: custom browser args + the Chrome UA break
        // WebView2 composition on child webviews (gray page) - see the
        // BROWSER_ARGS doc. Default ("all") never applies them.
        if name == "args" {
            return opts.split(',').any(|o| o.trim() == "args" || o.trim() == "argsbg" || o.trim() == "argsms" || o.trim() == "argsry");
        }
        opts == "all" || opts.split(',').any(|o| o.trim() == name)
    };
    // The page IS the integration: each service loads its own real web
    // chat (WhatsApp Web / Messenger), authenticated inside the view.
    let url: url::Url = account_service_url(&account.service)
        .parse()
        .expect("service URL constant must parse");
    let mut builder = WebviewBuilder::new(account_webview_label(&account.id), WebviewUrl::External(url));
    if has("ua") {
        builder = builder.user_agent(USER_AGENT_DESKTOP);
    }
    if has("scripts") {
        for script in startup_scripts() {
            builder = builder.initialization_script(script);
        }
    }
    if has("perms") {
        builder = builder
            .enable_clipboard_access()
            .on_permission_request(|_webview, kind| permission_response(kind));
    }
    if has("data") {
        let data_directory = crate::SHARED
            .get()
            .expect("shared globals must exist before webview creation")
            .settings
            .data_dir()
            .join("webview")
            .join(&account.id);
        builder = builder.data_directory(data_directory);
    }
    if has("data") && has("ua") {
        builder = builder.focused(false);
    }
    #[cfg(windows)]
    if has("ua") {
        // Pop-out windows (calls): without this handler WebView2 DENIES
        // every window.open (wry's default is SetHandled(true)) and
        // WhatsApp tells the user to "modify the browser to allow popups".
        let account_id = account.id.clone();
        builder = builder.on_new_window(move |url, features| {
            popup_window(&account_id, url, &features)
        });
    }
    #[cfg(windows)]
    if has("args") {
        // Bisectable variants: 'argsbg' = only the backgrounding flags,
        // 'argsms' = only the ms* feature disables, 'argsry' = the wry
        // defaults + autoplay policy, 'args' = full set.
        let args = if opts.contains("argsry") {
            "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required"
        } else if opts.contains("argsbg") {
            "--disable-background-timer-throttling --disable-renderer-backgrounding --disable-backgrounding-occluded-windows"
        } else if opts.contains("argsms") {
            "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection"
        } else {
            BROWSER_ARGS
        };
        builder = builder.additional_browser_args(args);
    }
    builder
}

/// Logical size available for content webviews (everything right of the rail).
pub(crate) fn content_size(window: &Window<Wry>) -> tauri::Result<LogicalSize<f64>> {
    let scale = window.scale_factor()?;
    let physical = window.inner_size()?;
    let logical = physical.to_logical::<f64>(scale);
    Ok(LogicalSize::new(
        (logical.width - RAIL_WIDTH).max(1.0),
        logical.height.max(1.0),
    ))
}

/// Keep every webview glued to its slot after a window resize. The active
/// content webview sits right of the rail; every other content webview is
/// parked at FULL SIZE outside the visible area (position-only parking, so
/// account switches never trigger a WhatsApp re-render).
fn relayout(window: &Window<Wry>) {
    let Ok(content) = content_size(window) else {
        return;
    };
    let active = ACTIVE.lock().ok().and_then(|guard| guard.clone());
    let drawer_open = DRAWER_OPEN.lock().map(|guard| *guard).unwrap_or(false);
    let picker_open = picker_open_now();
    for webview in window.webviews() {
        let label = webview.label().to_string();
        let (position, size) = if label == "rail" {
            (
                LogicalPosition::new(0.0_f64, 0.0_f64),
                LogicalSize::new(RAIL_WIDTH, content.height),
            )
        } else if label == "drawer" {
            if drawer_open {
                // Full-screen options: the panel covers the WHOLE content area
                // (the rail stays visible so the brand logo keeps toggling).
                (
                    LogicalPosition::new(RAIL_WIDTH, 0.0_f64),
                    LogicalSize::new(content.width, content.height),
                )
            } else {
                (
                    LogicalPosition::new(PARKED_X, 0.0_f64),
                    LogicalSize::new(content.width, content.height),
                )
            }
        } else if label == SERVICE_PICKER_WINDOW {
            if picker_open {
                // The + click menu strip: a horizontal panel anchored
                // immediately RIGHT of the rail, vertically aligned with
                // the "+" tile. The + is PINNED near the rail's bottom with
                // a 16px air gap (spans [H-76, H-32]); the 72px strip card
                // tops at H-88 so its vertical centre sits on the tile's
                // line. The webview was born FULL content size (paint
                // law) - this branch shrinks it into the strip slot right
                // after creation.
                (
                    LogicalPosition::new(RAIL_WIDTH + 6.0, (content.height - 88.0).max(0.0)),
                    LogicalSize::new(178.0, 88.0),
                )
            } else {
                (
                    LogicalPosition::new(PARKED_X, 0.0_f64),
                    LogicalSize::new(content.width, content.height),
                )
            }
        } else if active.as_deref() == Some(label.as_str()) {
            (
                LogicalPosition::new(RAIL_WIDTH, 0.0_f64),
                LogicalSize::new(content.width, content.height),
            )
        } else {
            (
                LogicalPosition::new(PARKED_X, 0.0_f64),
                LogicalSize::new(content.width, content.height),
            )
        };
        let _ = webview.set_position(position);
        let _ = webview.set_size(size);
    }
}

/// Any ENABLED account configured? `create_shell` decides the welcome
/// webview's variant from this single check: with accounts it boots as a
/// pure brand splash (no picker), with none it boots as the real picker.
fn has_enabled_accounts() -> bool {
    crate::SHARED
        .get()
        .map(|shared| {
            shared
                .settings
                .current_accounts()
                .iter()
                .any(|account| account.enabled)
        })
        .unwrap_or(false)
}

/// Create the shell window: rail webview + welcome backdrop, then show it
/// (unless the user chose "start minimized" - then only the tray exists
/// until the user opens the window from the tray).
/// The first account is opened right after by `auto_open_first_account`.
pub fn create_shell(app: &AppHandle) -> tauri::Result<()> {
    // Only AUTOSTART launches honor the "start minimized" preference: a
    // manual click on the app icon must bring a visible window up no matter
    // the setting (the tray exists for close-to-tray, not to hide manual
    // launches; the auto-started process carries /startminimized).
    let start_minimized = crate::SHARED
        .get()
        .map(|shared| shared.settings.current_prefs().start_minimized)
        .unwrap_or(false)
        && launched_by_autostart();
    let window = tauri::window::Window::builder(app, "main")
        .title("Zapdude")
        .inner_size(1180.0_f64, 780.0_f64)
        .min_inner_size(760.0_f64, 560.0_f64)
        .resizable(true)
        .visible(false)
        .build()?;
    // Native frame + (later) every message view must match the theme BEFORE
    // anything is visible - otherwise the window flashes the OS look.
    apply_theme_everywhere(app);

    let content = content_size(&window)?;
    let base = ui_base_url();
    let rail_url: url::Url = format!("{base}/").parse().expect("rail URL must parse");
    let welcome_url: url::Url = if has_enabled_accounts() {
        // Booting WITH accounts: the welcome webview renders only the brand
        // beat (?splash=1 strips the account picker). The full "create an
        // account" screen flashing for one second at every startup read
        // like a reachable destination - and nothing ever led back to it
        // (the user: that screen belongs to the no-accounts state, where it
        // is the real picker).
        format!("{base}/welcome?splash=1")
    } else {
        // No accounts: the welcome IS the account picker (the WhatsApp /
        // Messenger add buttons) - a real destination.
        format!("{base}/welcome")
    }
    .parse()
    .expect("welcome URL must parse");

    // 1. The Zapdude rail (always visible, always on the left).
    window.add_child(
        WebviewBuilder::new("rail", WebviewUrl::External(rail_url)),
        LogicalPosition::new(0.0_f64, 0.0_f64),
        LogicalSize::new(RAIL_WIDTH, content.height),
    )?;

    // 2. Welcome / branded splash (replaced by an account webview as soon
    //    as one is opened; a proper destination when no accounts remain).
    window.add_child(
        WebviewBuilder::new("welcome", WebviewUrl::External(welcome_url)),
        LogicalPosition::new(RAIL_WIDTH, 0.0_f64),
        LogicalSize::new(content.width, content.height),
    )?;

    if let Ok(mut active) = ACTIVE.lock() {
        *active = Some("welcome".to_string());
    }

    // Re-flow webviews when the window is resized, scaled or focused.
    let watcher = window.clone();
    window.on_window_event(move |event| {
        if matches!(
            event,
            tauri::WindowEvent::Resized(_)
                | tauri::WindowEvent::ScaleFactorChanged { .. }
                | tauri::WindowEvent::Focused(_)
        ) {
            relayout(&watcher);
        }
    });

    // GEOMETRY WATCHDOG: the four corners of the active webview must ALWAYS
    // hug the window's client area (the user hit a state where the chat
    // drifted off after some resize/dpi dance). relayout is idempotent and
    // cheap; running it every couple of seconds makes drift impossible to
    // survive, whatever event we missed.
    let guard = window.clone();
    std::thread::Builder::new()
        .name("zapdude-geometry".into())
        .spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            if !guard.is_minimized().unwrap_or(false) {
                relayout(&guard);
            }
        })
        .ok();

    if !start_minimized {
        let _ = window.show();
        let _ = window.set_focus();
    }
    Ok(())
}

/// Startup opens EVERY enabled account (not just the first): parked webviews
/// keep receiving messages and firing notifications, so all accounts are
/// reachable and notify from the first second - without a manual click per
/// account (the user hit exactly that: account 2 stayed silent until they
/// opened it manually). The FIRST account takes the visible content slot;
/// the rest park outside the viewport (still rendered). Every account is
/// "just a web page", so early-bird memory cost grows with account count;
/// if that ever hurts, a lazy-accounts preference can return.
pub fn auto_open_all_accounts(app: &AppHandle) {
    let Some(shared) = crate::SHARED.get().cloned() else {
        return;
    };
    let handle = app.clone();
    std::thread::spawn(move || {
        // A single branded-splash beat (the styled welcome) before the
        // accounts take over - no long blank interstitial.
        std::thread::sleep(std::time::Duration::from_millis(150));
        let mut accounts = shared.settings.current_accounts();
        if accounts.is_empty() {
            let account = crate::config::AccountConfig {
                id: "account-1".to_string(),
                label: "Account 1".to_string(),
                enabled: true,
                ..crate::config::AccountConfig::default()
            };
            accounts.push(account.clone());
            if let Err(err) = shared.settings.update_accounts(accounts) {
                eprintln!("zapdude: could not persist the first account: {err}");
            }
            accounts = vec![account];
        }
        // Sequential creation: the FIRST enabled account ends up visible,
        // every other one is created and immediately parked by relayout.
        let mut first: Option<crate::config::AccountConfig> = None;
        for account in accounts.iter().filter(|account| account.enabled) {
            if first.is_none() {
                first = Some(account.clone());
                if let Err(err) = open_account_webview(&handle, account) {
                    eprintln!("zapdude: could not open account '{}': {}", account.id, err);
                }
                continue;
            }
            if let Err(err) = ensure_account_webview(&handle, account) {
                eprintln!("zapdude: could not open account '{}': {}", account.id, err);
            }
        }
        // Belt and suspenders: make sure the FIRST account holds the
        // content slot even if a later open flow raced it.
        if let Some(account) = first {
            let _ = show_content(
                &handle,
                &account_webview_label(&account.id),
            );
        }
    });
}

/// Move the content webview `label` into the content slot and park the rest.
/// Pure geometry (no hide/show) so WebView2 never blanks a webview.
pub fn show_content(app: &AppHandle, label: &str) -> tauri::Result<()> {
    let Some(window) = app.get_window("main") else {
        return Ok(());
    };
    if let Ok(mut active) = ACTIVE.lock() {
        *active = Some(label.to_string());
    }
    relayout(&window);
    let _ = window.set_focus();
    Ok(())
}

/// Create the WhatsApp Web webview of an account, installing the API bridge.
/// The webview is born at FULL content size in the content slot - WebView2
/// only starts its composition pipeline for webviews that begin life at a
/// real size (born-tiny webviews stay gray forever). `relayout` immediately
/// parks it if it is not the active one. Does nothing if it already exists.
pub fn ensure_account_webview(
    app: &AppHandle,
    account: &crate::config::AccountConfig,
) -> tauri::Result<()> {
    let label = account_webview_label(&account.id);
    if let Some(existing) = app.get_webview(&label) {
        // Already created earlier in this session; make sure the bridge is
        // installed (idempotent) and return.
        let bus = crate::api::server::bus();
        crate::api::page_ops::install(&account.id, existing, bus);
        // Keep the profile's color scheme in lockstep with the shell theme
        // (cheap and idempotent - also covers views opened before this
        // feature existed, e.g. the first open after upgrading).
        apply_theme_everywhere(app);
        return Ok(());
    }
    let Some(window) = app.get_window("main") else {
        return Ok(());
    };
    let size = content_size(&window)?;
    // `add_child` dispatches to the main thread, so the registry may not see
    // a creation that is still queued. If the label reports "already exists"
    // the webview is materializing - treat it as success instead of failing
    // the open flow (which would leave the welcome screen stuck on screen).
    // The creating path installs the bridge itself, so no double install.
    match window.add_child(
        account_webview_builder(account),
        LogicalPosition::new(RAIL_WIDTH, 0.0_f64),
        size,
    ) {
        Ok(webview) => {
            let bus = crate::api::server::bus();
            // REPLACE the bridge: the webview was created fresh, so any
            // previous bridge for this account pointed at a destroyed view
            // (close + re-open flows) and every automation call would time
            // out against a dead target.
            crate::api::page_ops::replace(&account.id, webview, bus);
            // Fresh view: pin the profile's color scheme to the shell theme
            // BEFORE WhatsApp's own boot code reads the media feature.
            apply_theme_everywhere(app);
            Ok(())
        }
        Err(err) => {
            if err.to_string().contains("already exists") {
                Ok(())
            } else {
                Err(err)
            }
        }
    }
}

/// Create (if needed) and show an account's WhatsApp Web webview.
/// Closing the options drawer is part of opening an account: the QR/chat
/// takes the whole content area.
pub fn open_account_webview(
    app: &AppHandle,
    account: &crate::config::AccountConfig,
) -> tauri::Result<()> {
    if let Ok(mut open) = DRAWER_OPEN.lock() {
        *open = false;
    }
    ensure_account_webview(app, account)?;
    show_content(app, &account_webview_label(&account.id))
}

/// After an account is hidden or removed: open the FIRST REMAINING account,
/// or fall back to the branded welcome screen (which has an explicit
/// "Add account" button). Never auto-creates an empty account here -
/// deleting the last one intentionally lands on the welcome screen.
fn open_first_remaining_or_welcome(app: &AppHandle) {
    let Some(shared) = crate::SHARED.get() else {
        return;
    };
    let accounts = shared.settings.current_accounts();
    let handle = app.clone();
    if let Some(first) = accounts.into_iter().find(|account| account.enabled) {
        std::thread::spawn(move || {
            if let Err(err) = open_account_webview(&handle, &first) {
                eprintln!("zapdude: could not open account '{}': {}", first.id, err);
            }
        });
    } else {
        // The welcome webview may still carry the ?splash=1 variant from a
        // booted-with-accounts shell; the picker needs its add buttons back
        // before it takes the screen.
        let base = ui_base_url();
        if let Some(webview) = app.get_webview("welcome") {
            if let Ok(url) = format!("{base}/welcome").parse::<url::Url>() {
                let _ = webview.navigate(url);
            }
        }
        let _ = show_content(app, "welcome");
    }
}

/// Return to a sensible view after an account is hidden: the first remaining
/// account or the welcome screen with its "Add account" button (no
/// auto-created empty account).
pub fn hide_account_content(app: &AppHandle, _id: &str) -> tauri::Result<()> {
    open_first_remaining_or_welcome(app);
    Ok(())
}

/// Close an account webview entirely (used when removing the account).
/// If the removed account was on screen, fall through to the first remaining
/// account, or to the welcome screen when none remains (no empty account is
/// auto-created - the user asked for that explicitly). The WebView2 profile
/// of the account is ALSO deleted: removing an account means a clean slate
/// (no orphaned sessions on disk).
pub fn close_account_webview(app: &AppHandle, id: &str) {
    let label = account_webview_label(id);
    let was_active = active_content().as_deref() == Some(label.as_str());
    // Cancel the bridge FIRST (its poller holds a Webview handle of the
    // view being destroyed).
    crate::api::page_ops::remove(id);
    if let Some(webview) = app.get_webview(&label) {
        let _ = webview.close();
    }
    // Best-effort profile wipe (ignores errors - e.g. WebView2 still
    // flushing the folder).
    if let Some(shared) = crate::SHARED.get() {
        let profile = shared
            .settings
            .data_dir()
            .join("webview")
            .join(id);
        let _ = std::fs::remove_dir_all(&profile);
    }
    if was_active {
        open_first_remaining_or_welcome(app);
    }
}

/// Navigate ONLY the rail + welcome webviews to the (possibly new) base
/// URL. Used after preference changes (theme) so every shell page shows
/// the same look instantly - no half-dark/half-light state.
pub fn reload_shell_pages(app: &AppHandle) {
    let base = ui_base_url();
    let Some(window) = app.get_window("main") else {
        return;
    };
    for webview in window.webviews() {
        let target = match webview.label() {
            "rail" => format!("{base}/"),
            "welcome" => format!("{base}/welcome"),
            _ => continue,
        };
        if let Ok(url) = target.parse::<url::Url>() {
            let _ = webview.navigate(url);
        }
    }
}

/// Push the shell theme into every WhatsApp view (accounts + popups).
///
/// WHY NATIVE: with no explicit theme chosen, WhatsApp Web follows the
/// `prefers-color-scheme` media feature (its default "system" look), which
/// used to mean "whatever Windows is set to" - so a light shell was glued
/// to a dark chat area, and toggling light mode visibly changed only the
/// 76px rail. The WebView2 profile API exists for exactly this: setting
/// `ICoreWebView2Profile::PreferredColorScheme` pins the media feature to
/// light/dark for every view on that profile, letting WhatsApp's own
/// theming code (JS + CSS media queries) resolve the matching palette with
/// no injected CSS to maintain against WhatsApp redesigns.
///
/// Idempotent (applied once per view; profiles are per account), cheap, and
/// silent on older runtimes lacking the interface: theming may not follow
/// there, but the app never breaks. A theme chosen INSIDE WhatsApp still
/// wins - this emulates "follow the app look", it is not a hammer.
#[cfg(windows)]
pub fn apply_theme_to_message_views(app: &AppHandle) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_13, COREWEBVIEW2_PREFERRED_COLOR_SCHEME_DARK,
        COREWEBVIEW2_PREFERRED_COLOR_SCHEME_LIGHT,
    };
    use windows_core::Interface;
    let theme = crate::SHARED
        .get()
        .map(|shared| shared.settings.current_prefs().theme)
        .unwrap_or_else(|| "dark".to_string());
    let scheme = if theme.eq_ignore_ascii_case("light") {
        COREWEBVIEW2_PREFERRED_COLOR_SCHEME_LIGHT
    } else {
        COREWEBVIEW2_PREFERRED_COLOR_SCHEME_DARK
    };
    for (label, webview) in app.webviews() {
        // Message views only: shell pages theme themselves via CSS, and
        // popups ride the parent account's profile, so they get the value
        // too (setting it on every view sharing a profile is harmless).
        if !(label.starts_with("account-") || label.starts_with("popup-")) {
            continue;
        }
        let _ = webview.with_webview(move |platform| unsafe {
            // Runs on the main thread: WebView2 COM objects are STA-bound,
            // so touching them anywhere else is undefined behavior.
            let controller = platform.controller();
            let Ok(core) = controller.CoreWebView2() else {
                return;
            };
            let Ok(core13) = core.cast::<ICoreWebView2_13>() else {
                return; // pre-1165 runtime: no profile handle available
            };
            let Ok(profile) = core13.Profile() else {
                return;
            };
            let _ = profile.SetPreferredColorScheme(scheme);
        });
    }
}

/// Non-Windows builds keep theming shell-only: the rail/welcome/drawer
/// pages follow the preference through CSS on their own.
#[cfg(not(windows))]
pub fn apply_theme_to_message_views(_app: &AppHandle) {}

/// Match the NATIVE window frames (titlebars) to the shell theme. Windows
/// paints caption bars following the OS setting, so a light shell grew a
/// dark system titlebar over it - jarring after a theme toggle.
#[cfg(windows)]
fn apply_window_theme(app: &AppHandle) {
    let theme = crate::SHARED
        .get()
        .map(|shared| shared.settings.current_prefs().theme)
        .unwrap_or_else(|| "dark".to_string());
    let use_dark = !theme.eq_ignore_ascii_case("light");
    for (label, window) in app.windows() {
        if label == "main" || label.starts_with("popup-") {
            let _ = window.set_theme(Some(if use_dark {
                tauri::Theme::Dark
            } else {
                tauri::Theme::Light
            }));
        }
    }
}

#[cfg(not(windows))]
fn apply_window_theme(_app: &AppHandle) {}

/// One-stop theming hook: shell pages, WhatsApp views and native frames.
pub fn apply_theme_everywhere(app: &AppHandle) {
    apply_window_theme(app);
    apply_theme_to_message_views(app);
}

/// Navigate rail/welcome/settings webviews to the (possibly new) base URL.
/// Used after the API port changes so the shell keeps working.
pub fn reload_shell_ui(app: &AppHandle) {
    let base = ui_base_url();
    let Some(window) = app.get_window("main") else {
        return;
    };
    for webview in window.webviews() {
        let target = match webview.label() {
            "rail" => format!("{base}/"),
            "welcome" => format!("{base}/welcome"),
            "drawer" => format!("{base}/drawer"),
            other if other.starts_with("account-") => continue, // WhatsApp views keep their session
            _ => continue,
        };
        if let Ok(url) = target.parse::<url::Url>() {
            let _ = webview.navigate(url);
        }
    }
}

/// Build the tray menu for the CURRENT account list (rebuilt on changes so
/// "Show <account>" never goes stale).
fn build_tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let shared = crate::SHARED.get().expect("shared globals");
    let accounts = shared.settings.current_accounts();
    let mut item_pool: Vec<MenuItem<tauri::Wry>> = Vec::new();
    for account in &accounts {
        let item = MenuItem::with_id(
            app,
            format!("show-{}", account.id),
            format!("Show {}", account.label),
            true,
            None::<&str>,
        )?;
        item_pool.push(item);
    }
    let separator = PredefinedMenuItem::separator(app)?;
    let hide_all = MenuItem::with_id(app, "hide-all", "Hide all windows", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Zapdude", true, None::<&str>)?;
    let menu_items: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = item_pool
        .iter()
        .map(|item| item as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
        .chain(std::iter::once(&separator as &dyn tauri::menu::IsMenuItem<tauri::Wry>))
        .chain(std::iter::once(&hide_all as &dyn tauri::menu::IsMenuItem<tauri::Wry>))
        .chain(std::iter::once(&quit as &dyn tauri::menu::IsMenuItem<tauri::Wry>))
        .collect();
    Menu::with_items(app, &menu_items)
}

/// Show + focus the main window (tray "Show <account>" must bring the
/// window BACK even if the user closed it to the tray - showing the webview
/// alone does nothing while the window is hidden).
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Tray: quick account switching, hide-all and exit (closes the whole app).
pub fn initialize_tray(app: &App) {
    let build = move || -> tauri::Result<()> {
        let menu = build_tray_menu(app.handle())?;
        let icon = app.default_window_icon().cloned();
        let mut tray_builder = TrayIconBuilder::with_id("zapdude-tray")
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_menu_event(|app_handle, event| {
                crate::window::handle_tray_menu_event(app_handle, event.id().as_ref());
            })
            .on_tray_icon_event(|tray, event| {
                // Left click on the tray icon itself: bring the window back.
                if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, button_state: tauri::tray::MouseButtonState::Up, .. } = event {
                    show_main_window(tray.app_handle());
                }
            });
        if let Some(icon) = icon {
            tray_builder = tray_builder.icon(icon);
        }
        tray_builder.build(app.handle())?;
        Ok(())
    };
    if let Err(failure) = build() {
        eprintln!("zapdude: tray could not initialize: {failure}");
    }
}

/// Rebuild the tray menu after the account registry changed (add/remove)
/// so the entries always match reality.
pub fn refresh_tray(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = handle.tray_by_id("zapdude-tray") {
            match build_tray_menu(&handle) {
                Ok(menu) => {
                    let _ = tray.set_menu(Some(menu));
                }
                Err(err) => eprintln!("zapdude: tray menu refresh failed: {err}"),
            }
        }
    });
}

/// Route tray menu clicks: switch to an account webview, hide everything,
/// or exit the app.
pub fn handle_tray_menu_event(app: &AppHandle, event_id: &str) {
    let shared = crate::SHARED.get().expect("shared globals");
    if let Some(account) = shared
        .settings
        .current_accounts()
        .into_iter()
        .find(|account| event_id == format!("show-{}", account.id))
    {
        // The window must come back FIRST (it may be hidden in the tray):
        // showing a webview inside a hidden window does nothing visible.
        show_main_window(app);
        // Webview creation must never happen on the event thread.
        let handle = app.clone();
        std::thread::spawn(move || {
            if let Err(err) = open_account_webview(&handle, &account) {
                eprintln!("zapdude: could not open account '{}': {}", account.id, err);
            }
        });
        return;
    }
    match event_id {
        "hide-all" => {
            // Hide every window (main + popouts): the tray keeps the app alive.
            for (_, window) in app.windows() {
                let _ = window.hide();
            }
        }
        "quit" => {
            crate::api::server::shutdown_loopback_server();
            app.exit(0);
        }
        _ => {}
    }
}

/// True when the process was spawned by the Windows Run-key autostart
/// entry, which passes the `/startminimized` argument (see apply_autostart).
/// Manual launches (icon clicks) do not pass it and always show the window.
#[cfg(windows)]
fn launched_by_autostart() -> bool {
    std::env::args()
        .any(|arg| arg == "/startminimized" || arg == "--startminimized")
}

#[cfg(not(windows))]
fn launched_by_autostart() -> bool {
    false
}

/// Windows: register/unregister Zapdude in the HKCU Run key so it launches
/// when the user logs in (no admin needed). The registered command carries
/// `/startminimized` so the auto-launched instance stays in the tray while
/// manual launches still open the window.
#[cfg(windows)]
pub fn apply_autostart(enable: bool) -> Result<(), String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE};
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run = hkcu
        .open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Run",
            KEY_SET_VALUE | KEY_QUERY_VALUE,
        )
        .map_err(|err| err.to_string())?;
    if enable {
        let exe = std::env::current_exe().map_err(|err| err.to_string())?;
        let command = format!("\"{}\" /startminimized", exe.to_string_lossy());
        run.set_value("Zapdude", &command)
            .map_err(|err| err.to_string())
    } else {
        match run.delete_value("Zapdude") {
            Ok(()) => Ok(()),
            // Not registered: removing again is a no-op, not an error.
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }
}

#[cfg(not(windows))]
pub fn apply_autostart(_enable: bool) -> Result<(), String> {
    Ok(())
}
