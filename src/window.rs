//! ZapZap-style application shell: ONE window that hosts every webview.
//!
//! Layout (like the original ZapZap desktop client):
//!   - A narrow left rail (label `rail`) loaded from `rail.html`: the ZapZap
//!     logo, one circular avatar per account (click to switch), a "+" button
//!     to add an account in one click, and settings/about buttons at the
//!     bottom. The rail is the only thing the user always sees.
//!   - One WhatsApp Web webview per account (label `account-<id>`), each with
//!     an isolated WebView2 profile so every account stays logged in. They
//!     are stacked on top of each other; exactly one is visible at a time.
//!   - A welcome webview (label `welcome`) shown when no account is active.
//!
//! Multi-webview inside one window needs the Tauri `unstable` feature; all
//! webview creation happens from non-main threads (async commands, setup or
//! spawned threads) so the WebView2 main-thread deadlock does not apply.

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

/// Renderer must stay unthrottled in background so calls ring and hidden
/// accounts keep working. The three-flags disable mirrors the Python fork.
/// The webview2 default disables (UI PDF/SmartScreen) must be re-declared
/// whenever additional args are provided.
#[cfg(windows)]
const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection \
    --disable-background-timer-throttling \
    --disable-renderer-backgrounding \
    --disable-backgrounding-occluded-windows";

const CLIENT_HINTS_FIX_SCRIPT: &str = include_str!("../scripts/client_hints_fix.js");
const API_PAGE_BRIDGE_SCRIPT: &str = include_str!("../scripts/api_page_bridge.js");
const HIDE_DOWNLOAD_BANNERS_SCRIPT: &str = include_str!("../scripts/hide_download_banners.js");

/// Logical width of the left rail. The content area gets the rest.
pub const RAIL_WIDTH: f64 = 76.0;

/// Which content webview is currently visible ("welcome" or an account id).
static ACTIVE: Mutex<Option<String>> = Mutex::new(None);

pub fn account_webview_label(account_id: &str) -> String {
    format!("account-{account_id}")
}

/// The id of the webview currently shown (None = rail only / unknown).
pub fn active_content() -> Option<String> {
    ACTIVE.lock().ok().and_then(|guard| guard.clone())
}

fn combined_initialization_script() -> String {
    // Run only in the top-level document; WhatsApp Web subframes stay clean.
    format!(
        "(function () {{ if (window !== window.top) {{ return; }} {} {} {} }})();",
        CLIENT_HINTS_FIX_SCRIPT, API_PAGE_BRIDGE_SCRIPT, HIDE_DOWNLOAD_BANNERS_SCRIPT
    )
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
fn account_webview_builder(account: &crate::config::AccountConfig) -> WebviewBuilder<Wry> {
    let data_directory = crate::SHARED
        .get()
        .expect("shared globals must exist before webview creation")
        .settings
        .data_dir()
        .join("webview")
        .join(&account.id);
    let url: url::Url = WHATSAPP_URL.parse().expect("WhatsApp URL constant must parse");
    let mut builder = WebviewBuilder::new(account_webview_label(&account.id), WebviewUrl::External(url))
        .user_agent(USER_AGENT_DESKTOP)
        .initialization_script(combined_initialization_script())
        .enable_clipboard_access()
        .on_permission_request(|_webview, kind| permission_response(kind))
        .data_directory(data_directory)
        .focused(false);
    #[cfg(windows)]
    {
        builder = builder.additional_browser_args(BROWSER_ARGS);
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

/// Keep every webview glued to its slot after a window resize.
fn relayout(window: &Window<Wry>) {
    let Ok(content) = content_size(window) else {
        return;
    };
    for webview in window.webviews() {
        let is_rail = webview.label() == "rail";
        let position = if is_rail {
            LogicalPosition::new(0.0_f64, 0.0_f64)
        } else {
            LogicalPosition::new(RAIL_WIDTH, 0.0_f64)
        };
        let size = if is_rail {
            LogicalSize::new(RAIL_WIDTH, content.height)
        } else {
            LogicalSize::new(content.width, content.height)
        };
        let _ = webview.set_position(position);
        let _ = webview.set_size(size);
    }
}

/// Create the shell window: rail webview + welcome content, then show it.
pub fn create_shell(app: &AppHandle) -> tauri::Result<()> {
    let window = tauri::window::Window::builder(app, "main")
        .title("ZapZap")
        .inner_size(1180.0_f64, 780.0_f64)
        .min_inner_size(760.0_f64, 560.0_f64)
        .resizable(true)
        .visible(false)
        .build()?;

    let content = content_size(&window)?;

    // 1. The ZapZap rail (always visible, always on the left).
    window.add_child(
        WebviewBuilder::new("rail", WebviewUrl::App("rail.html".into())),
        LogicalPosition::new(0.0_f64, 0.0_f64),
        LogicalSize::new(RAIL_WIDTH, content.height),
    )?;

    // 2. Welcome / onboarding content (default until an account is opened).
    window.add_child(
        WebviewBuilder::new("welcome", WebviewUrl::App("welcome.html".into())),
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

/// Show one content webview (an account or "welcome") and hide the rest.
pub fn show_content(app: &AppHandle, label: &str) -> tauri::Result<()> {
    let Some(window) = app.get_window("main") else {
        return Ok(());
    };
    for webview in window.webviews() {
        if webview.label() == "rail" {
            continue; // the rail never changes
        }
        if webview.label() == label {
            let _ = webview.show();
        } else {
            let _ = webview.hide();
        }
    }
    if let Ok(mut active) = ACTIVE.lock() {
        *active = Some(label.to_string());
    }
    let _ = window.set_focus();
    Ok(())
}

/// Create the WhatsApp Web webview of an account in the shell, hidden.
/// Installs the API page bridge so automation keeps working while hidden.
pub fn create_account_webview_hidden(
    app: &AppHandle,
    account: &crate::config::AccountConfig,
) -> tauri::Result<()> {
    let label = account_webview_label(&account.id);
    if app.get_webview(&label).is_some() {
        return Ok(()); // already exists
    }
    let Some(window) = app.get_window("main") else {
        return Ok(());
    };
    let size = content_size(&window)?;
    let webview = window.add_child(
        account_webview_builder(account),
        LogicalPosition::new(RAIL_WIDTH, 0.0_f64),
        size,
    )?;
    let bus = crate::api::server::bus();
    crate::api::page_ops::install(&account.id, webview.clone(), bus);
    let _ = webview.hide();
    Ok(())
}

/// Create (if needed) and show an account's WhatsApp Web webview.
pub fn open_account_webview(
    app: &AppHandle,
    account: &crate::config::AccountConfig,
) -> tauri::Result<()> {
    create_account_webview_hidden(app, account)?;
    show_content(app, &account_webview_label(&account.id))
}

/// Hide an account webview and return to the welcome screen.
pub fn hide_account_content(app: &AppHandle, id: &str) -> tauri::Result<()> {
    let label = account_webview_label(id);
    if let Some(webview) = app.get_webview(&label) {
        let _ = webview.hide();
    }
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

/// Open (or focus) the settings window on the requested tab.
/// The tab is applied via `on_page_load` (fresh window) or a direct eval
/// (existing window); `settings.html` also polls `window.__zapTab` as a
/// belt-and-braces fallback.
pub fn create_settings_window(app: &AppHandle, tab: &str) -> tauri::Result<()> {
    let tab_for_js = tab.replace(['\\', '\'', ';'], "");
    let tab_for_load = tab_for_js.clone();
    let window = tauri::webview::WebviewWindowBuilder::new(
        app,
        "settings",
        WebviewUrl::App("settings.html".into()),
    )
    .title("Configuración — ZapZap")
    .inner_size(720.0_f64, 660.0_f64)
    .min_inner_size(560.0_f64, 500.0_f64)
    .resizable(true)
    .on_page_load(move |webview, payload| {
        if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
            let _ = webview.eval(&format!(
                "window.__zapTab = '{}'; window.__zapSetTab && window.__zapSetTab('{}');",
                tab_for_load, tab_for_load
            ));
        }
    })
    .build()?;
    let _ = window.set_focus();
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
            .show_menu_on_left_click(true)
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
