//! ZapZap-style application shell: ONE window that hosts every webview.
//!
//! Layout (like the original ZapZap desktop client):
//!   - A narrow left rail (label `rail`) — the ZapZap logo, one circular
//!     avatar per account (click to switch), a "+" button to add an account
//!     in one click, and settings/about buttons at the bottom.
//!   - One WhatsApp Web webview per account (label `account-<id>`), each with
//!     an isolated WebView2 profile so every account stays logged in.
//!   - A welcome webview (label `welcome`) used as a branded backdrop.
//!
//! Visibility model: the active content webview sits next to the rail;
//! every other content webview is PARKED just outside the visible client
//! area (still rendered, never `hide()`-d). WebView2 blanks webviews that
//! are created hidden and shown later — parking keeps them alive and
//! paintable, so switching accounts never shows a gray page.
//!
//! The rail/welcome/settings pages are SERVED BY THE LOOPBACK API SERVER
//! (same origin), so their `fetch` calls always work — no IPC, no CORS,
//! no file:// resolution problems. Multi-webview inside one window needs
//! the Tauri `unstable` feature; webview creation happens from non-main
//! threads (setup, async commands or spawned threads) so the WebView2
//! main-thread deadlock does not apply.

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

/// Renderer args. WARNING (empirical, WebView2 + wry 0.57): passing ANY
/// `additional_browser_args` on child webviews combined with the Chrome
/// user-agent breaks WebView2 composition — the page loads and JS runs but
/// the compositor renders a flat gray surface. The same flags applied by
/// wry internally (no custom args) work fine, so we do NOT pass custom
/// browser args in production; this hook remains for debugging only.
#[cfg(windows)]
const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --disable-background-timer-throttling --disable-renderer-backgrounding --disable-backgrounding-occluded-windows";

const CLIENT_HINTS_FIX_SCRIPT: &str = include_str!("../scripts/client_hints_fix.js");
const API_PAGE_BRIDGE_SCRIPT: &str = include_str!("../scripts/api_page_bridge.js");
const HIDE_DOWNLOAD_BANNERS_SCRIPT: &str = include_str!("../scripts/hide_download_banners.js");

/// Logical width of the left rail. The content area gets the rest.
pub const RAIL_WIDTH: f64 = 76.0;

/// Inactive content webviews park at 1x1 px inside the window. Keeping them
/// tiny (instead of hidden) means switching accounts always changes the
/// webview's REAL size, which forces WebView2 to re-compose — hidden
/// webviews and same-size moves leave a gray/blank page behind.
const PARKED_SIZE: f64 = 1.0;

/// Which content webview is currently in the content slot ("welcome" or an
/// account id). Everything else is parked.
static ACTIVE: Mutex<Option<String>> = Mutex::new(None);

pub fn account_webview_label(account_id: &str) -> String {
    format!("account-{account_id}")
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
/// beacon poller). The automation bridge and the cosmetic promo hider do
/// not need to run before WhatsApp's own scripts, and deferring them keeps
/// the document-start injection small enough that WebView2 keeps painting.
pub fn deferred_scripts() -> String {
    let opts = std::env::var("ZAPDUKE_WEBVIEW_OPTS").unwrap_or_else(|_| "all".to_string());
    let want = |name: &str| {
        !opts.contains("scripts1") && !opts.contains("scripts2") && !opts.contains("scripts3")
            || opts.contains(name)
    };
    let mut body = String::new();
    if want("scripts2") {
        body.push_str(API_PAGE_BRIDGE_SCRIPT);
    }
    if want("scripts3") {
        body.push_str(HIDE_DOWNLOAD_BANNERS_SCRIPT);
    }
    body
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

/// Builder for one WhatsApp Web account webview (identity + bridge).
///
/// DEBUG BISECTION: ZAPDUKE_WEBVIEW_OPTS (comma list) selects which options
/// are applied, to isolate which one breaks WebView2 painting:
///   ua | scripts | data | args | perms  (default: all)
fn account_webview_builder(account: &crate::config::AccountConfig) -> WebviewBuilder<Wry> {
    let opts = std::env::var("ZAPDUKE_WEBVIEW_OPTS").unwrap_or_else(|_| "all".to_string());
    let has = |name: &str| {
        // 'args' is debug-only: custom browser args + the Chrome UA break
        // WebView2 composition on child webviews (gray page) — see the
        // BROWSER_ARGS doc. Default ("all") never applies them.
        if name == "args" {
            return opts.split(',').any(|o| o.trim() == "args" || o.trim() == "argsbg" || o.trim() == "argsms" || o.trim() == "argsry");
        }
        opts == "all" || opts.split(',').any(|o| o.trim() == name)
    };
    let url: url::Url = WHATSAPP_URL.parse().expect("WhatsApp URL constant must parse");
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
fn content_size(window: &Window<Wry>) -> tauri::Result<LogicalSize<f64>> {
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
/// parked outside the visible area (never hidden, so it keeps rendering).
fn relayout(window: &Window<Wry>) {
    let Ok(content) = content_size(window) else {
        return;
    };
    let active = ACTIVE.lock().ok().and_then(|guard| guard.clone());
    for webview in window.webviews() {
        let is_rail = webview.label() == "rail";
        let is_active = active.as_deref() == Some(webview.label());
        let position = if is_rail {
            LogicalPosition::new(0.0_f64, 0.0_f64)
        } else if is_active {
            LogicalPosition::new(RAIL_WIDTH, 0.0_f64)
        } else {
            LogicalPosition::new(RAIL_WIDTH, 0.0_f64)
        };
        let size = if is_rail {
            LogicalSize::new(RAIL_WIDTH, content.height)
        } else if is_active {
            LogicalSize::new(content.width, content.height)
        } else {
            LogicalSize::new(PARKED_SIZE, PARKED_SIZE)
        };
        let _ = webview.set_position(position);
        let _ = webview.set_size(size);
    }
}

/// Create the shell window: rail webview + welcome backdrop, then show it.
/// The first account is opened right after by `auto_open_first_account`.
pub fn create_shell(app: &AppHandle) -> tauri::Result<()> {
    let window = tauri::window::Window::builder(app, "main")
        .title("ZapZap")
        .inner_size(1180.0_f64, 780.0_f64)
        .min_inner_size(760.0_f64, 560.0_f64)
        .resizable(true)
        .visible(false)
        .build()?;

    let content = content_size(&window)?;
    let base = ui_base_url();
    let rail_url: url::Url = format!("{base}/").parse().expect("rail URL must parse");
    let welcome_url: url::Url = format!("{base}/welcome")
        .parse()
        .expect("welcome URL must parse");

    // 1. The ZapZap rail (always visible, always on the left).
    window.add_child(
        WebviewBuilder::new("rail", WebviewUrl::External(rail_url)),
        LogicalPosition::new(0.0_f64, 0.0_f64),
        LogicalSize::new(RAIL_WIDTH, content.height),
    )?;

    // 2. Welcome / branded backdrop (replaced by an account webview as soon
    //    as one is opened; only visible if the app truly has no accounts).
    window.add_child(
        WebviewBuilder::new("welcome", WebviewUrl::External(welcome_url)),
        LogicalPosition::new(RAIL_WIDTH, 0.0_f64),
        LogicalSize::new(content.width, content.height),
    )?;

    if let Ok(mut active) = ACTIVE.lock() {
        *active = Some("welcome".to_string());
    }

    // Re-flow webviews when the window is resized.
    let watcher = window.clone();
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Resized(_)) {
            relayout(&watcher);
        }
    });

    let _ = window.show();
    let _ = window.set_focus();
    Ok(())
}

/// ZapZap behavior: the app opens STRAIGHT into the first account's WhatsApp
/// Web (the QR if it is not logged in) — no welcome interstitial, no forms.
/// If no account exists yet, the first one is created automatically.
pub fn auto_open_first_account(app: &AppHandle) {
    let Some(shared) = crate::SHARED.get().cloned() else {
        return;
    };
    let handle = app.clone();
    std::thread::spawn(move || {
        // Give the shell a beat to finish laying out.
        std::thread::sleep(std::time::Duration::from_millis(400));
        let mut accounts = shared.settings.current_accounts();
        if accounts.is_empty() {
            let account = crate::config::AccountConfig {
                id: "account-1".to_string(),
                label: "Cuenta 1".to_string(),
                enabled: true,
            };
            accounts.push(account.clone());
            if let Err(err) = shared.settings.update_accounts(accounts) {
                eprintln!("zapdude: could not persist the first account: {err}");
            }
            accounts = vec![account];
        }
        if let Err(err) = open_account_webview(&handle, &accounts[0]) {
            eprintln!("zapdude: could not open the first account: {err}");
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
/// The webview is born at FULL content size in the content slot — WebView2
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
        return Ok(());
    }
    let Some(window) = app.get_window("main") else {
        return Ok(());
    };
    let size = content_size(&window)?;
    // `add_child` dispatches to the main thread, so the registry may not see
    // a creation that is still queued. If the label reports "already exists"
    // the webview is materializing — treat it as success instead of failing
    // the open flow (which would leave the welcome screen stuck on screen).
    // The creating path installs the bridge itself, so no double install.
    match window.add_child(
        account_webview_builder(account),
        LogicalPosition::new(RAIL_WIDTH, 0.0_f64),
        size,
    ) {
        Ok(webview) => {
            let bus = crate::api::server::bus();
            crate::api::page_ops::install(&account.id, webview, bus);
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
pub fn open_account_webview(
    app: &AppHandle,
    account: &crate::config::AccountConfig,
) -> tauri::Result<()> {
    ensure_account_webview(app, account)?;
    show_content(app, &account_webview_label(&account.id))
}

/// Return to the welcome backdrop (account webview stays parked).
pub fn hide_account_content(app: &AppHandle, _id: &str) -> tauri::Result<()> {
    show_content(app, "welcome")
}

/// Close an account webview entirely (used when removing the account).
pub fn close_account_webview(app: &AppHandle, id: &str) {
    let label = account_webview_label(id);
    if let Some(webview) = app.get_webview(&label) {
        let _ = webview.close();
    }
    if active_content().as_deref() == Some(label.as_str()) {
        let _ = show_content(app, "welcome");
    }
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
            other if other.starts_with("account-") => continue, // WhatsApp views keep their session
            _ => continue,
        };
        if let Ok(url) = target.parse::<url::Url>() {
            let _ = webview.navigate(url);
        }
    }
    if let Some(settings) = app.get_webview_window("settings") {
        let _ = settings.navigate(format!("{base}/settings?tab=api").parse().unwrap());
    }
}

/// Open (or focus) the settings window on the requested tab.
/// The tab travels as a query param (`?tab=api|agent|about`) which the
/// server-served page reads natively.
pub fn create_settings_window(app: &AppHandle, tab: &str) -> tauri::Result<()> {
    let safe_tab: String = tab
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    let url: url::Url = format!("{}/settings?tab={}", ui_base_url(), safe_tab)
        .parse()
        .expect("settings URL must parse");
    tauri::webview::WebviewWindowBuilder::new(app, "settings", WebviewUrl::External(url))
        .title("Configuración — ZapZap")
        .inner_size(720.0_f64, 660.0_f64)
        .min_inner_size(560.0_f64, 500.0_f64)
        .resizable(true)
        .build()?;
    Ok(())
}

/// Tray: quick account switching, hide-all and exit (closes the whole app).
pub fn initialize_tray(app: &App) {
    let build = move || -> tauri::Result<()> {
        let shared = crate::SHARED.get().expect("shared globals");
        let accounts = shared.settings.current_accounts();
        let mut item_pool: Vec<MenuItem<tauri::Wry>> = Vec::new();
        for account in &accounts {
            let item = MenuItem::with_id(
                app,
                format!("show-{}", account.id),
                format!("Mostrar {}", account.label),
                true,
                None::<&str>,
            )?;
            item_pool.push(item);
        }
        let separator = PredefinedMenuItem::separator(app)?;
        let hide_all = MenuItem::with_id(app, "hide-all", "Ocultar ventanas", true, None::<&str>)?;
        let quit = MenuItem::with_id(app, "quit", "Salir de ZapZap", true, None::<&str>)?;
        let menu_items: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = item_pool
            .iter()
            .map(|item| item as &dyn tauri::menu::IsMenuItem<tauri::Wry>)
            .chain(std::iter::once(&separator as &dyn tauri::menu::IsMenuItem<tauri::Wry>))
            .chain(std::iter::once(&hide_all as &dyn tauri::menu::IsMenuItem<tauri::Wry>))
            .chain(std::iter::once(&quit as &dyn tauri::menu::IsMenuItem<tauri::Wry>))
            .collect();
        let menu = Menu::with_items(app, &menu_items)?;
        let icon = app.default_window_icon().cloned();
        let mut tray_builder = TrayIconBuilder::with_id("zapdude-tray")
            .menu(&menu)
            .on_menu_event(|app_handle, event| {
                crate::window::handle_tray_menu_event(app_handle, event.id().as_ref());
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
            let _ = show_content(app, "welcome");
        }
        "quit" => {
            crate::api::server::shutdown_loopback_server();
            app.exit(0);
        }
        _ => {}
    }
}
