//! Tauri commands exposed to the shell UI (`rail.html`, `welcome.html`,
//! `settings.html`) through `window.__TAURI__.core.invoke(...)`.
//!
//! The ZapZap philosophy applies here too: adding an account is ONE click —
//! no forms, no ids to type. The backend picks `account-N` / `Cuenta N`.

use serde::Serialize;
use tauri::{AppHandle, Manager, Wry};

use crate::api::page_ops::page_for;
use crate::config::AccountConfig;
use crate::window;

/// Snapshot of one account as seen by the rail.
#[derive(Clone, Serialize)]
pub struct AccountView {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    /// True when this account's WhatsApp Web webview is the visible one.
    pub active: bool,
    /// "detecting" | "scan_qr" | "logged_in"
    pub session_state: String,
}

/// API configuration mirror (settings window).
#[derive(Clone, Serialize, serde::Deserialize)]
pub struct ApiConfigView {
    pub enabled: bool,
    pub port: u16,
    pub api_key: String,
    pub webhook_url: String,
    pub webhook_secret: String,
    pub expose_network: bool,
    pub version: String,
}

fn three_state(snapshot: &serde_json::Value) -> String {
    let composer = snapshot
        .get("composer_visible")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let chat_list = snapshot
        .get("chat_list")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let qrcode = snapshot
        .get("qrcode_visible")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if composer || chat_list {
        "logged_in".to_string()
    } else if qrcode {
        "scan_qr".to_string()
    } else {
        "detecting".to_string()
    }
}

/// List every configured account with its live state (polled by the rail).
#[tauri::command]
pub async fn get_accounts(_app: AppHandle) -> Vec<AccountView> {
    let Some(shared) = crate::SHARED.get() else {
        return Vec::new();
    };
    let settings = shared.settings.clone();
    let active = window::active_content();
    let mut views = Vec::new();
    for account in settings.current_accounts() {
        let session_state = if let Some(bridge) = page_for(Some(&account.id)) {
            match bridge.status_snapshot().await {
                Ok(snapshot) => three_state(&snapshot),
                Err(_) => "detecting".to_string(),
            }
        } else {
            "detecting".to_string()
        };
        views.push(AccountView {
            id: account.id.clone(),
            label: account.label.clone(),
            enabled: account.enabled,
            active: active.as_deref() == Some(&window::account_webview_label(&account.id)),
            session_state,
        });
    }
    views
}

/// ONE-CLICK account creation (ZapZap style): no form, no ids to type.
/// Picks `account-N` / `Cuenta N`, creates the webview and switches to it so
/// the user immediately sees the QR code.
#[tauri::command]
pub async fn add_account(app: AppHandle) -> Result<AccountView, String> {
    let Some(shared) = crate::SHARED.get() else {
        return Err("app state missing".to_string());
    };
    let settings = shared.settings.clone();
    let mut accounts = settings.current_accounts();
    // First free "account-N" slot (label stays in Spanish like the original).
    let mut n = accounts.len() + 1;
    let mut id = format!("account-{n}");
    while accounts.iter().any(|a| a.id == id) {
        n += 1;
        id = format!("account-{n}");
    }
    let label = format!("Cuenta {n}");
    let account = AccountConfig {
        id: id.clone(),
        label: label.clone(),
        enabled: true,
    };
    accounts.push(account.clone());
    settings.update_accounts(accounts).map_err(|e| e.to_string())?;

    let app_for_open = app.clone();
    let account_for_open = account.clone();
    // add_child must not run inside a command that may itself be on the main
    // thread; tauri's async commands already run off-thread, but spawning
    // keeps the WebView2 deadlock window closed.
    let _ = tauri::async_runtime::spawn(async move {
        let _ = window::open_account_webview(&app_for_open, &account_for_open);
    });

    Ok(AccountView {
        id: account.id,
        label: account.label,
        enabled: true,
        active: true,
        session_state: "detecting".to_string(),
    })
}

/// Switch the shell to an account's WhatsApp Web webview.
#[tauri::command]
pub async fn open_account(app: AppHandle, id: String) -> Result<(), String> {
    let Some(shared) = crate::SHARED.get() else {
        return Err("app state missing".to_string());
    };
    let settings = shared.settings.clone();
    let Some(account) = settings.account(&id) else {
        return Err(format!("account '{id}' not found"));
    };
    window::open_account_webview(&app, &account).map_err(|e| e.to_string())
}

/// Hide an account webview and show the welcome screen again.
#[tauri::command]
pub async fn hide_account(app: AppHandle, id: String) -> Result<(), String> {
    window::hide_account_content(&app, &id).map_err(|e| e.to_string())
}

/// Remove an account from config and destroy its webview.
#[tauri::command]
pub async fn remove_account(app: AppHandle, id: String) -> Result<(), String> {
    let Some(shared) = crate::SHARED.get() else {
        return Err("app state missing".to_string());
    };
    let settings = shared.settings.clone();
    let before = settings.current_accounts().len();
    let accounts: Vec<AccountConfig> = settings
        .current_accounts()
        .into_iter()
        .filter(|a| a.id != id)
        .collect();
    if accounts.len() == before {
        return Err(format!("account '{id}' not found"));
    }
    settings.update_accounts(accounts).map_err(|e| e.to_string())?;
    window::close_account_webview(&app, &id);
    Ok(())
}

/// Open (or focus) the settings window on a given tab ("api" | "agent" | "about").
#[tauri::command]
pub async fn open_settings(app: AppHandle, tab: Option<String>) -> Result<(), String> {
    let tab = tab.unwrap_or_else(|| "api".to_string());
    let safe_tab = tab
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>();
    if let Some(existing) = app.get_webview_window("settings") {
        let _ = existing.show();
        let _ = existing.set_focus();
        let _ = existing.eval(&format!(
            "window.__zapSetTab && window.__zapSetTab('{}')",
            safe_tab
        ));
        return Ok(());
    }
    window::create_settings_window(&app, &safe_tab).map_err(|e| e.to_string())
}

/// Current API configuration (settings window).
#[tauri::command]
pub async fn get_api_config() -> ApiConfigView {
    let api = crate::SHARED
        .get()
        .map(|shared| shared.settings.current())
        .unwrap_or_default();
    ApiConfigView {
        enabled: api.enabled,
        port: api.port,
        api_key: api.api_key,
        webhook_url: api.webhook_url,
        webhook_secret: api.webhook_secret,
        expose_network: api.expose_network,
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Save API configuration from the settings window and restart the server.
#[tauri::command]
pub async fn save_api_config(config: ApiConfigView) -> Result<(), String> {
    let Some(shared) = crate::SHARED.get() else {
        return Err("app state missing".to_string());
    };
    let settings = shared.settings.clone();
    let mut api = crate::config::ApiConfig {
        enabled: config.enabled,
        port: config.port,
        api_key: config.api_key,
        expose_network: config.expose_network,
        webhook_url: config.webhook_url,
        webhook_secret: config.webhook_secret,
    };
    api = api.sanitized();
    settings.update_api(api.clone()).map_err(|e| e.to_string())?;
    // Always restart so port/key changes take effect immediately.
    crate::api::server::shutdown_loopback_server();
    if api.enabled {
        crate::api::server::start_loopback_server();
    }
    Ok(())
}

/// Quit the whole application (tray + UI both can call this).
#[tauri::command]
pub fn quit_app(app: AppHandle) {
    crate::api::server::shutdown_loopback_server();
    app.exit(0);
}

#[allow(dead_code)]
fn _type_assertion(app: &AppHandle<Wry>) {
    // Ensures the runtime type stays Wry across refactors.
    let _ = app.get_webview_window("main");
}
