//! Zapdude - WhatsApp Web desktop client with a local automation API.
//! Shell layout is inspired by the client whose idea motivated this
//! project (ZapZap by Rafael Tosta); the implementation is original code.
#![windows_subsystem = "windows"]
//!   - ONE window ("main") hosts every webview:
//!       * `rail`        - the narrow account rail (logo, avatars, "+")
//!       * `welcome`     - technical backdrop (never a destination)
//!       * `drawer`      - full-screen options panel (logo click, lazy)
//!       * `account-<id>` - one WhatsApp Web webview per account, switched
//!         in place like browser tabs, each with its own isolated profile.
//!         Created LAZILY on first open (startup stays instant no matter
//!         how many accounts exist).
//!   - A tray menu mirrors the rail for quick account switching.
//!   - A loopback HTTP API lets people and AI agents control the app.

mod api;
mod config;
mod window;

use std::sync::OnceLock;

use tauri::Manager;

/// Global shared state used by the window and the API server threads.
pub static SHARED: OnceLock<SharedGlobals> = OnceLock::new();

/// Global AppHandle so HTTP endpoints can show/hide account webviews.
pub static APP_HANDLE: OnceLock<AppHandleBox> = OnceLock::new();

pub type AppHandleBox = tauri::AppHandle<tauri::Wry>;

#[derive(Clone, Debug)]
pub struct SharedGlobals {
    pub settings: config::SettingsState,
}

/// Only one instance may run: a second one would fight over the loopback
/// API port and the WebView2 profile directories (zombie instances made the
/// shell load a stale app state). The mutex releases automatically when the
/// process dies, so crashes never lock the app out.
#[cfg(windows)]
fn enforce_single_instance() {
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateMutexW(
            lp_mutex_attributes: *mut core::ffi::c_void,
            b_initial_owner: i32,
            lp_name: *const u16,
        ) -> *mut core::ffi::c_void;
        fn GetLastError() -> u32;
    }
    const ERROR_ALREADY_EXISTS: u32 = 183;
    let name: Vec<u16> = "Local\\app.zapzap.desktop.single\0"
        .encode_utf16()
        .collect();
    unsafe {
        CreateMutexW(std::ptr::null_mut(), 0, name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            eprintln!("zapdude: another Zapdude instance is already running; exiting");
            std::process::exit(0);
        }
    }
}

#[cfg(not(windows))]
fn enforce_single_instance() {}

fn main() {
    enforce_single_instance();
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("app.zapzap.desktop"));
            let settings = config::SettingsState::load(&data_dir);
            eprintln!(
                "zapdude: settings loaded, {} accounts, api.enabled={}",
                settings.current_accounts().len(),
                settings.current().enabled
            );
            let _ = SHARED.set(SharedGlobals {
                settings: settings.clone(),
            });
            let _ = APP_HANDLE.set(app.handle().clone());

            // 1. The loopback API MUST be listening before any shell webview
            //    exists: the rail/welcome pages load http://127.0.0.1:<port>,
            //    and a webview that navigates before the bind shows WebView2's
            //    "can't reach this page" error at every startup.
            crate::api::server::start_loopback_server();

            // 2. The shell: rail + welcome backdrop.
            let app_handle = app.handle().clone();
            window::create_shell(&app_handle)?;

            // 3. Open the first account's WhatsApp Web right away (the app
            //    opens straight into the QR - no welcome interstitial).
            //    Creates the first account if none exists.
            //    Other accounts are created LAZILY on their first open, so
            //    startup and adding many accounts stay fast (they are "just
            //    web pages", but each WhatsApp Web is heavy; loading N of
            //    them at once made everything crawl).
            window::auto_open_first_account(&app_handle);

            // 4. Tray icon with per-account entries.
            window::initialize_tray(app);

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // The shell hides to tray; auxiliary windows (settings) close.
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Zapdude");
}
