//! Application windows: one WhatsApp Web window per account, each with an
//! isolated WebView2 profile (user-data folder) so all the user's accounts
//! stay logged in side by side. A tray menu gives people one-click account
//! switching; the automation API addresses accounts by id.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::webview::{PermissionKind, PermissionResponse};
use tauri::tray::TrayIconBuilder;
use tauri::{App, AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

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

pub fn account_window_label(account_id: &str) -> String {
    format!("account-{account_id}")
}

fn combined_initialization_script() -> String {
    // Run only in the top-level document; WhatsApp Web subframes stay clean.
    format!(
        "(function () {{ if (window !== window.top) {{ return; }} {} {} }})();",
        CLIENT_HINTS_FIX_SCRIPT, API_PAGE_BRIDGE_SCRIPT
    )
}

/// Permissions WhatsApp Web needs for calls, screen sharing, notifications
/// and clipboard interactions.
fn permission_response(kind: PermissionKind) -> PermissionResponse {
    match kind {
        PermissionKind::Microphone
        | PermissionKind::Camera
        | PermissionKind::DisplayCapture
        | PermissionKind::Notifications
        | PermissionKind::ClipboardRead
        | PermissionKind::Autoplay
        | PermissionKind::PointerLock => PermissionResponse::Allow,
        _ => PermissionResponse::Default,
    }
}

/// Build a single account window with its own WebView2 data directory.
fn create_account_window(
    app: &AppHandle,
    account: &crate::config::AccountConfig,
    visible: bool,
) -> tauri::Result<tauri::webview::WebviewWindow> {
    let shared = crate::SHARED
        .get()
        .expect("shared globals must exist before window creation")
        .clone();
    let url: url::Url = WHATSAPP_URL
        .parse()
        .expect("WhatsApp URL constant must parse");
    let data_directory = shared
        .settings
        .data_dir()
        .join("webview")
        .join(&account.id);
    let mut builder = WebviewWindowBuilder::new(
        app,
        account_window_label(&account.id),
        WebviewUrl::External(url),
    )
    .title(format!("ZapZap — {}", account.label))
    .inner_size(1280.0_f64, 860.0_f64)
    .min_inner_size(480.0_f64, 560.0_f64)
    .resizable(true)
    .user_agent(USER_AGENT_DESKTOP)
    .initialization_script(combined_initialization_script())
    .enable_clipboard_access()
    .on_permission_request(|_webview, kind| permission_response(kind))
    .data_directory(data_directory)
    .visible(visible);

    #[cfg(windows)]
    {
        builder = builder.additional_browser_args(BROWSER_ARGS);
    }

    let window = builder.build()?;
    Ok(window)
}

/// Tray: the human interface for switching between account windows and
/// closing the whole app (closing an account window just hides it).
pub fn initialize_tray(app: &App) {
    let build = move || -> tauri::Result<()> {
        let shared = crate::SHARED.get().expect("shared globals");
        let accounts = shared.settings.current_accounts();
        let mut item_pool: Vec<MenuItem<tauri::Wry>> = Vec::new();
        for account in &accounts {
            let label = if account.enabled {
                format!("Show {}", account.label)
            } else {
                format!("{} (disabled)", account.label)
            };
            let item = MenuItem::with_id(
                app,
                format!("show-{}", account.id),
                label,
                true,
                None::<&str>,
            )?;
            item_pool.push(item);
        }
        let separator = PredefinedMenuItem::separator(app)?;
        let hide_all =
            MenuItem::with_id(app, "hide-all", "Hide all windows", true, None::<&str>)?;
        let quit = MenuItem::with_id(app, "quit", "Exit ZapZap", true, None::<&str>)?;
        //Build the borrowed reference list.
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
            .tooltip("ZapZap")
            .on_menu_event(|app_handle, event| {
                crate::window::handle_tray_menu_event(app_handle, event.id().as_ref());
            });
        if let Some(icon) = icon {
            tray_builder = tray_builder.icon(icon);
        }
        let _tray = tray_builder.build(app.handle())?;
        Ok(())
    };
    if let Err(failure) = build() {
        eprintln!("zapdude: tray could not initialize: {failure}");
    }
}

/// Create the main ZapZap application window (the human interface).
/// Loads the local `ui/index.html` frontend: account list, settings, About.
/// We serve the file directly (file://) instead of `WebviewUrl::App` so it
/// works without a separate frontend build step.
pub fn create_main_window(app: &AppHandle) -> tauri::Result<tauri::webview::WebviewWindow> {
    let app_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("zapdude"))
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::temp_dir().join("zapdude"));
    // ui/index.html lives two levels up from the data dir:
    //   %APPDATA%\app.zapzap.desktop\ -> ../zapdude-rs/ui/index.html
    let html_path = app_dir.join("zapdude-rs").join("ui").join("index.html");
    let url = if html_path.exists() {
        format!("file://{}", html_path.display())
    } else {
        // Fallback: look next to the executable.
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_default();
        let alt = exe_dir.join("ui").join("index.html");
        if alt.exists() {
            format!("file://{}", alt.display())
        } else {
            // Last resort: bundle App URL.
            return WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("index.html".into()))
                .title("ZapZap")
                .inner_size(900.0_f64, 650.0_f64)
                .min_inner_size(700.0_f64, 500.0_f64)
                .resizable(true)
                .build()
                .map(|w| {
                    // Since file:// fails, open WhatsApp Web directly so the
                    // app is still usable (fallback mode).
                    let _ = w.navigate("https://web.whatsapp.com".parse().unwrap());
                    w
                });
        }
    };
    eprintln!("zapdude: main window loading URL: {}", url);
    let url: url::Url = url.parse().expect("main window URL must parse");
    let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
        .title("ZapZap")
        .inner_size(900.0_f64, 650.0_f64)
        .min_inner_size(700.0_f64, 500.0_f64)
        .resizable(true)
        .build()?;
    Ok(window)
}

/// Create an account window and install its page bridge (polling + beacon).
/// `visible` controls whether the WhatsApp Web window is shown immediately.
pub fn create_account_with_bridge(
    app: &AppHandle,
    account: &crate::config::AccountConfig,
    visible: bool,
) -> tauri::Result<tauri::webview::WebviewWindow> {
    let window = create_account_window(app, account, visible)?;
    let bus = crate::api::server::bus();
    crate::api::page_ops::install(&account.id, window.clone(), bus);
    Ok(window)
}

/// Route tray menu clicks: show an account window, hide them all, or exit.
pub fn handle_tray_menu_event(app: &AppHandle, event_id: &str) {
    let shared = crate::SHARED.get().expect("shared globals");
    let accounts = shared.settings.current_accounts();
    for account in &accounts {
        if event_id == format!("show-{}", account.id) {
            if let Some(window) = app.get_webview_window(&account_window_label(&account.id)) {
                let _ = window.show();
                let _ = window.set_focus();
            }
            return;
        }
    }
    match event_id {
        "hide-all" => {
            for account in &accounts {
                if let Some(window) = app.get_webview_window(&account_window_label(&account.id)) {
                    let _ = window.hide();
                }
            }
        }
        "quit" => {
            crate::api::server::shutdown_loopback_server();
            app.exit(0);
        }
        _ => {
            // Root/unmapped items are ignored.
        }
    }
}
