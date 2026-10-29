//! Loopback HTTP server for the automation API (axum).
//!
//! Endpoints (account-aware):
//!   GET  /api/healthz                - availability probe (no key required)
//!   GET  /api/config                 - read API config (localhost only)
//!   POST /api/config                 - save API config (localhost only)
//!   GET  /api/accounts               - configured accounts (localhost only)
//!   POST /api/accounts               - add an account (localhost only)
//!   DELETE /api/accounts/{id}        - remove an account (localhost only)
//!   POST /api/accounts/{id}/open     - show an account in the shell
//!   POST /api/accounts/{id}/hide     - hide an account webview
//!   GET  /api/status?account=<id>    - app + page snapshot (requires key)
//!   POST /api/open-chat              - {"account?":"work","phone":"55..."}
//!   POST /api/message                - {"account?":"work","phone","message"}
//!   GET  /api/chats?limit=20         - best-effort visible chat list
//!   GET  /api/events?since=<id>      - polled history (all accounts)
//!   GET  /api/events/stream          - SSE push (all accounts)
//!   GET|POST /api/webhook            - set an HTTPS receiver for events
//!
//! The server always binds the loopback interface so the desktop UI and
//! local tooling can reach it. Automation endpoints additionally require
//! `X-API-Key` (or `Authorization: Bearer`) AND `enabled: true` in config.
//! Rate limit: 60 requests / rolling 60 s. Body cap: 1 MB.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, OnceLock};

use axum::extract::{DefaultBodyLimit, Path, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};
use tauri::Manager;

use super::events::ApiEventBus;
use super::page_ops::page_for;
use super::protocol::{
    error_payload, extract_api_key, ApiError, LOCAL_API_VERSION, MAX_BODY_BYTES,
};
use crate::config::{self, AccountConfig, SettingsState};

/// The shell UI is served BY this server (same origin as the API), so the
/// rail/welcome/drawer pages never suffer CORS or path issues.
const RAIL_HTML: &str = include_str!("../../ui/rail.html");
const WELCOME_HTML: &str = include_str!("../../ui/welcome.html");
const DRAWER_HTML: &str = include_str!("../../ui/drawer.html");

/// Holder guarded by a Mutex so the server can be stopped and restarted
/// (port changes) without losing the slot.
static SERVER: OnceLock<Mutex<Option<ServerHolder>>> = OnceLock::new();
static RATE: OnceLock<super::protocol::RateLimiter> = OnceLock::new();
static BUS: OnceLock<Arc<ApiEventBus>> = OnceLock::new();

struct ServerHolder {
    shutdown: Arc<std::sync::atomic::AtomicBool>,
}

fn server_slot() -> &'static Mutex<Option<ServerHolder>> {
    SERVER.get_or_init(|| Mutex::new(None))
}

fn server_running() -> bool {
    server_slot()
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|holder| !holder.shutdown.load(std::sync::atomic::Ordering::Relaxed)))
        .unwrap_or(false)
}

pub fn bus() -> Arc<ApiEventBus> {
    BUS.get_or_init(ApiEventBus::new).clone()
}

pub fn rate_limiter() -> &'static super::protocol::RateLimiter {
    RATE.get_or_init(super::protocol::RateLimiter::new)
}

#[derive(Clone)]
pub struct ApiState {
    pub settings: SettingsState,
    pub bus: Arc<ApiEventBus>,
}

fn settings() -> Option<SettingsState> {
    crate::SHARED.get().map(|shared| shared.settings.clone())
}

/// Start the loopback server (idempotent). Always binds 127.0.0.1 so only
/// local processes reach the API unless `expose_network` is enabled.
pub fn start_loopback_server() {
    {
        let guard = match server_slot().lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        if let Some(holder) = guard.as_ref() {
            if !holder.shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                return; // already running
            }
        }
    }
    let Some(state_settings) = settings() else {
        return;
    };
    let app_settings = state_settings.current();

    // Schedule the webhook receiver if one was persisted.
    if !app_settings.webhook_url.is_empty() {
        bus().set_webhook(&app_settings.webhook_url, &app_settings.webhook_secret);
    }
    let state = ApiState {
        settings: state_settings,
        bus: bus(),
    };
    let host = if app_settings.expose_network { "0.0.0.0" } else { "127.0.0.1" };
    let bind_addr = format!("{host}:{}", app_settings.port);

    let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shutdown_handle = shutdown.clone();
    let thread = std::thread::Builder::new()
        .name("zapdude-api".into())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("zapdude-api-io")
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(err) => {
                    eprintln!("zapdude: API runtime could not start: {}", err);
                    return;
                }
            };
            let app = router(state);
            let serve = async {
                // Bind with retries: right after a config-triggered restart
                // the previous listener may still hold the port for a moment
                // (os error 10048). Without retries the API would stay dead.
                let mut listener = None;
                for attempt in 0..40u32 {
                    match tokio::net::TcpListener::bind(&bind_addr).await {
                        Ok(bound) => {
                            listener = Some(bound);
                            break;
                        }
                        Err(err) => {
                            if attempt == 0 || attempt % 8 == 0 {
                                eprintln!(
                                    "zapdude: API bind retry #{} on {}: {}",
                                    attempt + 1,
                                    bind_addr,
                                    err
                                );
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                        }
                    }
                }
                match listener {
                    Some(listener) => {
                        if let Err(err) = axum::serve(
                            listener,
                            app.into_make_service_with_connect_info::<SocketAddr>(),
                        )
                        .await
                        {
                            eprintln!("zapdude: API server stopped: {}", err);
                        }
                    }
                    None => {
                        eprintln!("zapdude: API bind failed on {} (gave up)", bind_addr);
                    }
                }
            };
            let watcher = {
                let shutdown_flag = shutdown_handle.clone();
                async move {
                    loop {
                        if shutdown_flag.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    }
                }
            };
            let _exit = runtime.block_on(async {
                tokio::select! {
                    _ = serve => 0u8,
                    _ = watcher => 1u8,
                }
            });
            runtime.shutdown_timeout(std::time::Duration::from_secs(2));
        });
    if thread.is_ok() {
        eprintln!(
            "zapdude: API server on http://{}:{} [v{}]",
            host, app_settings.port, LOCAL_API_VERSION
        );
        if let Ok(mut guard) = server_slot().lock() {
            *guard = Some(ServerHolder { shutdown });
        }
    }
}

/// Stop the server (automation endpoints unreachable; restartable).
pub fn shutdown_loopback_server() {
    if let Ok(mut guard) = server_slot().lock() {
        if let Some(holder) = guard.take() {
            holder
                .shutdown
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

/// Resolve a target account id: explicit request wins, then the first
/// enabled account.
fn resolve_account_id(
    state: &ApiState,
    requested: Option<&str>,
) -> Result<String, ApiError> {
    let target = requested.map(str::trim).filter(|raw| !raw.is_empty());
    match target {
        Some(id) => {
            if state.settings.account(id).is_some() {
                Ok(id.to_string())
            } else {
                Err(ApiError::bad_request(format!("unknown account '{id}'")))
            }
        }
        None => Ok(state
            .settings
            .primary_account()
            .map(|account| account.id)
            .unwrap_or_else(|| "default".to_string())),
    }
}

/// Automation endpoints stay closed while the API is disabled in config.
fn require_api_enabled(state: &ApiState) -> Result<(), ApiError> {
    if state.settings.current().enabled {
        Ok(())
    } else {
        Err(ApiError::unavailable(
            "API is disabled - enable it in Zapdude settings",
        ))
    }
}

fn router(state: ApiState) -> Router {
    Router::new()
        // --- Shell UI (same origin as the API: fetch always works) ---
        .route("/", get(serve_rail))
        .route("/welcome", get(serve_welcome))
        .route("/drawer", get(serve_drawer))
        .route("/api/ui/quit", post(ui_quit))
        .route("/api/ui/drawer", post(ui_toggle_drawer))
        .route("/api/ui/show", post(ui_show))
        // --- Management endpoints (no auth, used by the shell UI + local tools) ---
        .route("/api/healthz", get(healthz))
        .route("/api/prefs", get(get_prefs).post(post_prefs))
        .route("/api/config", get(get_config).post(post_config))
        .route("/api/accounts", get(list_accounts).post(add_account_http))
        .route("/api/accounts/{id}", delete(remove_account_http))
        .route("/api/accounts/{id}/open", post(open_account_http))
        .route("/api/accounts/{id}/hide", post(hide_account_http))
        .route("/api/accounts/{id}/rename", post(rename_account_http))
        // --- Automation endpoints (require X-API-Key + enabled) ---
        .route("/api/status", get(status))
        .route("/api/message", post(message))
        .route("/api/open-chat", post(open_chat))
        .route("/api/chats", get(chats))
        .route("/api/call", post(call_start))
        .route("/api/call/end", post(call_end))
        .route("/api/events", get(events))
        .route("/api/events/stream", get(events_stream))
        .route("/api/webhook", get(webhook_get).post(webhook_post))
        // --- Debug harness: unreachable in production (env-gated handlers,
        //     each call additionally requires the API key). ---
        .route("/api/_debug/eval", post(debug_eval))
        .route("/api/_debug/windows", get(debug_windows))
        .route("/api/_debug/windows/{label}/close", post(debug_close_window))
        .route("/api/_debug/emit", post(debug_emit))
        .with_state(state)
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
}

// -------------------------------------------------------------------------
// Shared helpers
// -------------------------------------------------------------------------

fn json_ok(extra: Value) -> (StatusCode, Json<Value>) {
    let mut base = json!({ "ok": true });
    if let (Value::Object(base_obj), Value::Object(extra_obj)) = (&mut base, &extra) {
        for (key, value) in extra_obj {
            base_obj.insert(key.clone(), value.clone());
        }
    }
    (StatusCode::OK, Json(base))
}

fn json_error(err: &ApiError) -> (StatusCode, Json<Value>) {
    (
        StatusCode::from_u16(err.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(serde_json::to_value(error_payload(err)).unwrap_or(json!({}))),
    )
}

fn authorize_call(headers: &HeaderMap) -> Result<(), ApiError> {
    let Some(state_settings) = settings() else {
        return Err(ApiError::internal("app state not initialized"));
    };
    let expected = state_settings.api_key();
    let provided = extract_api_key(
        headers.get("X-API-Key").and_then(|value| value.to_str().ok()),
        headers.get("Authorization").and_then(|value| value.to_str().ok()),
    );
    match provided {
        Some(candidate) if candidate == expected && !expected.is_empty() => {
            if rate_limiter().allow() {
                Ok(())
            } else {
                Err(ApiError::rate_limited())
            }
        }
        _ => Err(ApiError::unauthorized()),
    }
}

fn parse_query_to_map(raw: Option<&str>) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    if let Some(query) = raw {
        for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
            map.insert(key.into_owned(), value.into_owned());
        }
    }
    map
}

// -------------------------------------------------------------------------
// Management handlers (no auth; reachability gated by bind address)
// -------------------------------------------------------------------------

async fn serve_rail() -> impl IntoResponse {
    axum::response::Html(RAIL_HTML)
}

async fn serve_welcome() -> impl IntoResponse {
    axum::response::Html(WELCOME_HTML)
}

async fn serve_drawer() -> impl IntoResponse {
    axum::response::Html(DRAWER_HTML)
}

/// Toggle the options drawer (brand-logo click in the rail).
async fn ui_toggle_drawer() -> (StatusCode, Json<Value>) {
    if let Some(app_handle) = crate::APP_HANDLE.get() {
        if let Err(err) = crate::window::toggle_drawer(app_handle) {
            return json_error(&ApiError::internal(err.to_string()));
        }
        json_ok(json!({ "ok": true }))
    } else {
        json_error(&ApiError::internal("app handle not available"))
    }
}

/// Session state derived from a page snapshot (three honest states).
fn three_state(snapshot: &Value) -> String {
    let composer = snapshot
        .get("composer_visible")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let chat_list = snapshot
        .get("chat_list")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let qrcode = snapshot
        .get("qrcode_visible")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if composer || chat_list {
        "logged_in".to_string()
    } else if qrcode {
        "scan_qr".to_string()
    } else {
        "detecting".to_string()
    }
}

/// Bring the first instance's window back: a second launch (desktop
/// shortcut, Start Menu, exe double click) hits single-instance handling and
/// pings this endpoint instead of dying silently (main.rs). No key required:
/// the loopback binding alone already restricts this to local callers.
async fn ui_show() -> (StatusCode, Json<Value>) {
    if let Some(app_handle) = crate::APP_HANDLE.get() {
        crate::window::show_main_window(app_handle);
        json_ok(json!({ "shown": true }))
    } else {
        json_error(&ApiError::internal("app handle not available"))
    }
}

/// Quit the whole app from the shell UI.
async fn ui_quit() -> (StatusCode, Json<Value>) {
    if let Some(app_handle) = crate::APP_HANDLE.get() {
        let handle = app_handle.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(200));
            shutdown_loopback_server();
            handle.exit(0);
        });
        json_ok(json!({ "quitting": true }))
    } else {
        json_error(&ApiError::internal("app handle not available"))
    }
}

async fn healthz() -> impl IntoResponse {
    json_ok(json!({
        "service": "zapdude-api",
        "api_version": LOCAL_API_VERSION,
        "server_running": server_running(),
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

// --- App preferences (theme, autostart, minimized start) -------------------

async fn get_prefs() -> (StatusCode, Json<Value>) {
    let Some(state_settings) = settings() else {
        return json_error(&ApiError::internal("app state not initialized"));
    };
    let prefs = state_settings.current_prefs();
    json_ok(json!({
        "theme": prefs.theme,
        "autostart": prefs.autostart,
        "start_minimized": prefs.start_minimized,
    }))
}

#[derive(Deserialize)]
struct PrefsBody {
    theme: String,
    autostart: bool,
    start_minimized: bool,
}

async fn post_prefs(State(state): State<ApiState>, Json(body): Json<PrefsBody>) -> (StatusCode, Json<Value>) {
    let prefs = config::AppPrefs {
        theme: body.theme,
        autostart: body.autostart,
        start_minimized: body.start_minimized,
    }
    .sanitized();
    if let Err(err) = state.settings.update_prefs(prefs.clone()) {
        return json_error(&ApiError::internal(err));
    }
    // Windows autostart registration (Run key). Errors degrade: the pref
    // persists, the registry just may not have changed.
    let autostart_result = crate::window::apply_autostart(prefs.autostart);
    if let Err(err) = autostart_result {
        eprintln!("zapdude: autostart registry update failed: {err}");
    }
    // Re-apply the theme everywhere: rail + welcome reload with the fresh
    // preference (the drawer applied it locally already), and the account
    // webviews get the new color scheme pinned on their WebView2 profiles.
    // This is what keeps the whole window UNIFORM - pages that only read
    // prefs at load would otherwise mix light/dark after a toggle.
    if let Some(app_handle) = crate::APP_HANDLE.get() {
        crate::window::reload_shell_pages(app_handle);
        crate::window::apply_theme_everywhere(app_handle);
    }
    json_ok(json!({
        "theme": prefs.theme,
        "autostart": prefs.autostart,
        "start_minimized": prefs.start_minimized,
    }))
}

/// Rename an account (label only - ids are stable identities).
async fn rename_account_http(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    let raw = body.get("label").and_then(Value::as_str).unwrap_or("");
    let label: String = raw.trim().chars().take(40).collect();
    if label.is_empty() {
        return json_error(&ApiError::bad_request("label is required"));
    }
    let accounts: Vec<AccountConfig> = state
        .settings
        .current_accounts()
        .into_iter()
        .map(|mut account| {
            if account.id == id {
                account.label = label.clone();
            }
            account
        })
        .collect();
    if !accounts.iter().any(|account| account.id == id) {
        return json_error(&ApiError::bad_request(format!("account '{id}' not found")));
    }
    if let Err(err) = state.settings.update_accounts(accounts) {
        return json_error(&ApiError::internal(err));
    }
    if let Some(app_handle) = crate::APP_HANDLE.get() {
        crate::window::refresh_tray(app_handle);
    }
    json_ok(json!({ "id": id, "label": label }))
}

async fn get_config() -> (StatusCode, Json<Value>) {
    let Some(state_settings) = settings() else {
        return json_error(&ApiError::internal("app state not initialized"));
    };
    let api = state_settings.current();
    json_ok(json!({
        "enabled": api.enabled,
        "port": api.port,
        "api_key": api.api_key,
        "webhook_url": api.webhook_url,
        "webhook_secret": api.webhook_secret,
    }))
}

#[derive(Deserialize)]
struct ConfigBody {
    enabled: bool,
    port: u16,
    api_key: String,
    webhook_url: String,
    webhook_secret: String,
}

async fn post_config(
    State(state): State<ApiState>,
    Json(body): Json<ConfigBody>,
) -> (StatusCode, Json<Value>) {
    let mut api = config::ApiConfig {
        enabled: body.enabled,
        port: body.port,
        api_key: body.api_key,
        expose_network: false,
        webhook_url: body.webhook_url,
        webhook_secret: body.webhook_secret,
    };
    api = api.sanitized();
    if let Err(err) = state.settings.update_api(api.clone()) {
        return json_error(&ApiError::internal(err));
    }
    // Respond first; restart the server afterwards so port/key changes take
    // effect, then re-point the shell webviews at the (possibly new) port.
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(250));
        shutdown_loopback_server();
        start_loopback_server();
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Some(app_handle) = crate::APP_HANDLE.get() {
            crate::window::reload_shell_ui(app_handle);
        }
    });
    json_ok(json!({
        "enabled": api.enabled,
        "port": api.port,
        "api_key": api.api_key,
        "webhook_url": api.webhook_url,
        "webhook_secret": api.webhook_secret,
    }))
}

/// Cached session states (id → (state, fetched_at)) so the rail UI never
/// waits on a page snapshot: responses are instant, stale entries refresh
/// in the background.
static SESSION_STATE_CACHE: OnceLock<Mutex<HashMap<String, (String, std::time::Instant)>>> =
    OnceLock::new();

fn session_state_cache() -> &'static Mutex<HashMap<String, (String, std::time::Instant)>> {
    SESSION_STATE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

const SESSION_STATE_TTL: std::time::Duration = std::time::Duration::from_millis(2500);

fn cached_session_state(account_id: &str) -> Option<String> {
    session_state_cache()
        .lock()
        .ok()?
        .get(account_id)
        .filter(|(_, fetched)| fetched.elapsed() < SESSION_STATE_TTL)
        .map(|(state, _)| state.clone())
}

fn spawn_session_state_refresh(account_id: String) {
    tokio::spawn(async move {
        let state = if let Some(bridge) = page_for(Some(&account_id)) {
            tokio::time::timeout(
                std::time::Duration::from_millis(1200),
                bridge.status_snapshot(),
            )
            .await
            .ok()
            .and_then(|result| result.ok())
            .map(|snapshot| three_state(&snapshot))
            .unwrap_or_else(|| "detecting".to_string())
        } else {
            "detecting".to_string()
        };
        if let Ok(mut cache) = session_state_cache().lock() {
            cache.insert(account_id, (state, std::time::Instant::now()));
        }
    });
}

async fn list_accounts() -> (StatusCode, Json<Value>) {
    let Some(state_settings) = settings() else {
        return json_error(&ApiError::internal("app state not initialized"));
    };
    let active = crate::window::active_content();
    let accounts = state_settings.current_accounts();
    // Background refresh for anything stale - the response NEVER waits.
    for account in &accounts {
        if cached_session_state(&account.id).is_none() {
            spawn_session_state_refresh(account.id.clone());
        }
    }
    let account_entries: Vec<Value> = accounts
        .iter()
        .map(|account| {
            json!({
                "id": account.id,
                "label": account.label,
                "enabled": account.enabled,
                "active": active.as_deref()
                    == Some(crate::window::account_webview_label(&account.id).as_str()),
                "session_state": cached_session_state(&account.id)
                    .unwrap_or_else(|| "detecting".to_string()),
            })
        })
        .collect();
    json_ok(json!({ "accounts": account_entries }))
}

/// Create an account the easy way: no body required, ids are generated
/// (`account-N`, label `Account N`). The new account's WhatsApp Web opens
/// right away so the QR is immediately on screen.
async fn add_account_http(State(state): State<ApiState>) -> (StatusCode, Json<Value>) {
    let mut accounts = state.settings.current_accounts();
    let mut n = accounts.len() + 1;
    let mut id = format!("account-{n}");
    while accounts.iter().any(|a| a.id == id) {
        n += 1;
        id = format!("account-{n}");
    }
    let label = format!("Account {n}");
    let account = AccountConfig {
        id: id.clone(),
        label: label.clone(),
        enabled: true,
    };
    accounts.push(account.clone());
    if let Err(err) = state.settings.update_accounts(accounts) {
        return json_error(&ApiError::internal(err));
    }
    if let Some(app_handle) = crate::APP_HANDLE.get() {
        let handle = app_handle.clone();
        std::thread::spawn(move || {
            if let Err(err) = crate::window::open_account_webview(&handle, &account) {
                eprintln!(
                    "zapdude: could not open new account '{}': {}",
                    account.id, err
                );
            }
            crate::window::refresh_tray(&handle);
        });
    }
    json_ok(json!({ "id": id, "label": label, "created": true }))
}

async fn remove_account_http(Path(id): Path<String>) -> (StatusCode, Json<Value>) {
    let Some(state_settings) = settings() else {
        return json_error(&ApiError::internal("app state not initialized"));
    };
    let before = state_settings.current_accounts().len();
    let accounts: Vec<AccountConfig> = state_settings
        .current_accounts()
        .into_iter()
        .filter(|a| a.id != id)
        .collect();
    if accounts.len() == before {
        return json_error(&ApiError::bad_request(format!("account '{id}' not found")));
    }
    if let Err(err) = state_settings.update_accounts(accounts) {
        return json_error(&ApiError::internal(err));
    }
    if let Some(app_handle) = crate::APP_HANDLE.get() {
        crate::window::close_account_webview(app_handle, &id);
        crate::window::refresh_tray(app_handle);
    }
    json_ok(json!({ "id": id }))
}

async fn open_account_http(Path(id): Path<String>) -> (StatusCode, Json<Value>) {
    let Some(state_settings) = settings() else {
        return json_error(&ApiError::internal("app state not initialized"));
    };
    let Some(account) = state_settings.account(&id) else {
        return json_error(&ApiError::bad_request(format!("account '{id}' not found")));
    };
    if let Some(app_handle) = crate::APP_HANDLE.get() {
        let handle = app_handle.clone();
        std::thread::spawn(move || {
            if let Err(err) = crate::window::open_account_webview(&handle, &account) {
                eprintln!("zapdude: could not open account '{}': {}", account.id, err);
            }
        });
    }
    json_ok(json!({ "id": id, "opened": true }))
}

async fn hide_account_http(Path(id): Path<String>) -> (StatusCode, Json<Value>) {
    if let Some(app_handle) = crate::APP_HANDLE.get() {
        if let Err(err) = crate::window::hide_account_content(app_handle, &id) {
            return json_error(&ApiError::internal(err.to_string()));
        }
        json_ok(json!({ "id": id, "hidden": true }))
    } else {
        json_error(&ApiError::internal("app handle not available"))
    }
}

// -------------------------------------------------------------------------
// Automation handlers (require API key + enabled)
// -------------------------------------------------------------------------

async fn status(
    State(state): State<ApiState>,
    headers: HeaderMap,
    RawQuery(raw): RawQuery,
) -> (StatusCode, Json<Value>) {
    if let Err(err) = authorize_call(&headers) {
        return json_error(&err);
    }
    if let Err(err) = require_api_enabled(&state) {
        return json_error(&err);
    }
    let params = parse_query_to_map(raw.as_deref());
    let account_id = match resolve_account_id(&state, params.get("account").map(String::as_str)) {
        Ok(id) => id,
        Err(err) => return json_error(&err),
    };
    let app_settings = state.settings.current();
    let account_entries: Vec<Value> = state
        .settings
        .current_accounts()
        .iter()
        .map(|account| {
            json!({
                "id": account.id,
                "label": account.label,
                "enabled": account.enabled,
                "window_present": page_for(Some(&account.id)).is_some(),
            })
        })
        .collect();
    let mut body = json!({
        "name": "Zapdude",
        "version": env!("CARGO_PKG_VERSION"),
        "api_version": LOCAL_API_VERSION,
        "api": {
            "enabled": app_settings.enabled,
            "port": app_settings.port,
            "expose_network": app_settings.expose_network,
            "webhook_configured": !app_settings.webhook_url.is_empty(),
        },
        "accounts": account_entries,
    });
    if let Some(bridge) = page_for(Some(account_id.as_str())) {
        match bridge.status_snapshot().await {
            Ok(snapshot) => {
                // Three-state heuristic: the page cannot prove a session just
                // because no QR is rendered yet (slow first load). Agents
                // should retry while "detecting".
                let composer = snapshot
                    .get("composer_visible")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let chat_list = snapshot
                    .get("chat_list")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let qrcode = snapshot
                    .get("qrcode_visible")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let state = if composer || chat_list {
                    "logged_in"
                } else if qrcode {
                    "scan_qr"
                } else {
                    "detecting"
                };
                body["page"] = snapshot;
                body["session_state"] = json!(state);
            }
            Err(err) => body["page_error"] = json!(err.message),
        }
    }
    (StatusCode::OK, Json(body))
}

async fn open_chat(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    if let Err(err) = authorize_call(&headers) {
        return json_error(&err);
    }
    if let Err(err) = require_api_enabled(&state) {
        return json_error(&err);
    }
    let phone = body.get("phone").and_then(Value::as_str).unwrap_or("");
    if phone.is_empty() {
        return json_error(&ApiError::bad_request("phone is required"));
    }
    let text = body.get("text").and_then(Value::as_str);
    let account_id = match resolve_account_id(&state, body.get("account").and_then(Value::as_str)) {
        Ok(id) => id,
        Err(err) => return json_error(&err),
    };
    let Some(bridge) = page_for(Some(account_id.as_str())) else {
        return json_error(&ApiError::unavailable("account window is not ready"));
    };
    match bridge.open_chat(phone, text).await {
        Ok(detail) => (StatusCode::OK, Json(detail)),
        Err(err) => json_error(&err),
    }
}

async fn message(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    if let Err(err) = authorize_call(&headers) {
        return json_error(&err);
    }
    if let Err(err) = require_api_enabled(&state) {
        return json_error(&err);
    }
    let phone = body.get("phone").and_then(Value::as_str).unwrap_or("");
    let text = body.get("message").and_then(Value::as_str).unwrap_or("");
    if phone.is_empty() || text.is_empty() {
        return json_error(&ApiError::bad_request("phone and message are required"));
    }
    let account_id = match resolve_account_id(&state, body.get("account").and_then(Value::as_str)) {
        Ok(id) => id,
        Err(err) => return json_error(&err),
    };
    let Some(bridge) = page_for(Some(account_id.as_str())) else {
        return json_error(&ApiError::unavailable("account window is not ready"));
    };
    match bridge.send_message(phone, text).await {
        Ok(outcome) => (StatusCode::OK, Json(outcome)),
        Err(err) => json_error(&err),
    }
}

async fn chats(
    State(state): State<ApiState>,
    headers: HeaderMap,
    RawQuery(raw): RawQuery,
) -> (StatusCode, Json<Value>) {
    if let Err(err) = authorize_call(&headers) {
        return json_error(&err);
    }
    if let Err(err) = require_api_enabled(&state) {
        return json_error(&err);
    }
    let params = parse_query_to_map(raw.as_deref());
    let limit: u32 = params
        .get("limit")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(20);
    let account_id = match resolve_account_id(&state, params.get("account").map(String::as_str)) {
        Ok(id) => id,
        Err(err) => return json_error(&err),
    };
    let Some(bridge) = page_for(Some(account_id.as_str())) else {
        return json_error(&ApiError::unavailable("account window is not ready"));
    };
    match bridge.list_chats(limit).await {
        Ok(list) => (StatusCode::OK, Json(list)),
        Err(err) => json_error(&err),
    }
}

/// Start a call through the chat UI: opens the chat (creating the account
/// webview when it does not exist yet - automation callers should not need
/// to pre-open an account), then clicks the voice/video call button in the
/// conversation header. The call surface is WhatsApp's own (in-page panel
/// or popup window); this endpoint only drives the entry point.
async fn call_start(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    if let Err(err) = authorize_call(&headers) {
        return json_error(&err);
    }
    if let Err(err) = require_api_enabled(&state) {
        return json_error(&err);
    }
    let phone = body.get("phone").and_then(Value::as_str).unwrap_or("");
    if phone.is_empty() {
        return json_error(&ApiError::bad_request("phone is required"));
    }
    let call_type = body.get("type").and_then(Value::as_str).unwrap_or("voice");
    if call_type != "voice" && call_type != "video" {
        return json_error(&ApiError::bad_request("type must be 'voice' or 'video'"));
    }
    let account_id = match resolve_account_id(&state, body.get("account").and_then(Value::as_str)) {
        Ok(id) => id,
        Err(err) => return json_error(&err),
    };
    // The webview is lazy; a parked/unopened account gets opened first. The
    // open flow is fast (view shell); the page itself keeps loading and
    // open_chat below waits for the composer/bridge within its own budget.
    if page_for(Some(account_id.as_str())).is_none() {
        let Some(account) = state.settings.account(account_id.as_str()) else {
            return json_error(&ApiError::unavailable("account window is not ready"));
        };
        let Some(app_handle) = crate::APP_HANDLE.get() else {
            return json_error(&ApiError::internal("app handle not available"));
        };
        let handle = app_handle.clone();
        let requested = account.clone();
        match crate::window::open_account_webview(&handle, &requested) {
            Ok(()) => {}
            Err(err) => {
                return json_error(&ApiError::unavailable(format!(
                    "account window could not be opened: {err}"
                )));
            }
        }
    }
    let Some(bridge) = page_for(Some(account_id.as_str())) else {
        return json_error(&ApiError::unavailable("account window is not ready"));
    };
    // Open the target chat first (the call button lives in the chat header).
    if let Err(err) = bridge.open_chat(phone, None).await {
        return json_error(&err);
    }
    let arguments = serde_json::to_string(call_type)
        .unwrap_or_else(|_| "\"voice\"".to_string());
    let script = format!(
        "JSON.stringify(function(){{ try {{ if (!window.__zapdudeApiV1 || typeof window.__zapdudeApiV1.startCall !== 'function') {{ return null; }} return window.__zapdudeApiV1.startCall({arguments}); }} catch (error) {{ return {{ ok:false, error:'unexpected_error', detail: String(error && error.message || error) }}; }} }}())",
        arguments = arguments
    );
    match bridge.evaluate(script).await {
        Ok(outcome) => {
            let mut detail = json!({
                "account": account_id,
                "phone": phone,
                "type": call_type,
                "clicked": outcome.get("clicked").cloned().unwrap_or(json!(null)),
                "ok": outcome.get("ok").cloned().unwrap_or(json!(false)),
            });
            if let Some(error) = outcome.get("error").and_then(Value::as_str) {
                detail["error"] = json!(error);
            }
            (StatusCode::OK, Json(detail))
        }
        Err(err) => json_error(&err),
    }
}

/// End the active call for an account (clicks WhatsApp's hang-up control).
async fn call_end(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    if let Err(err) = authorize_call(&headers) {
        return json_error(&err);
    }
    if let Err(err) = require_api_enabled(&state) {
        return json_error(&err);
    }
    let account_id = match resolve_account_id(&state, body.get("account").and_then(Value::as_str)) {
        Ok(id) => id,
        Err(err) => return json_error(&err),
    };
    let Some(bridge) = page_for(Some(account_id.as_str())) else {
        return json_error(&ApiError::unavailable("account window is not ready"));
    };
    let script = String::from(
        "JSON.stringify(function(){ try { if (!window.__zapdudeApiV1 || typeof window.__zapdudeApiV1.endCall !== 'function') { return null; } return window.__zapdudeApiV1.endCall(); } catch (error) { return { ok:false, error:'unexpected_error', detail: String(error && error.message || error) }; } }())",
    );
    match bridge.evaluate(script).await {
        Ok(outcome) => (
            StatusCode::OK,
            Json(json!({
                "account": account_id,
                "ok": outcome.get("ok").cloned().unwrap_or(json!(false)),
                "clicked": outcome.get("clicked").cloned().unwrap_or(json!(null)),
                "error": outcome.get("error").cloned().unwrap_or(json!(null)),
            })),
        ),
        Err(err) => json_error(&err),
    }
}

async fn events(
    State(state): State<ApiState>,
    headers: HeaderMap,
    RawQuery(raw): RawQuery,
) -> (StatusCode, Json<Value>) {
    if let Err(err) = authorize_call(&headers) {
        return json_error(&err);
    }
    if let Err(err) = require_api_enabled(&state) {
        return json_error(&err);
    }
    let params = parse_query_to_map(raw.as_deref());
    let since = params.get("since").cloned();
    let history = state.bus.since(since.as_deref());
    json_ok(json!({ "events": history }))
}

async fn events_stream(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<
    Sse<impl Stream<Item = Result<Event, std::io::Error>>>,
    (StatusCode, Json<Value>),
> {
    if let Err(err) = authorize_call(&headers) {
        return Err(json_error(&err));
    }
    if let Err(err) = require_api_enabled(&state) {
        return Err(json_error(&err));
    }
    let receiver = state.bus.subscribe();
    let stream = BroadcastStream::new(receiver).filter_map(|packed| match packed {
        Ok(event) => {
            let id = event.get("id").and_then(Value::as_str).unwrap_or("");
            let data = serde_json::to_string(&event).ok()?;
            Some(Ok(Event::default().id(id).event("message").data(data)))
        }
        Err(BroadcastStreamRecvError::Lagged(_skipped)) => {
            // Lagged consumers just receive the next fresh event.
            None
        }
    });
    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("ping"),
    ))
}

async fn webhook_get(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> (StatusCode, Json<Value>) {
    if let Err(err) = authorize_call(&headers) {
        return json_error(&err);
    }
    if let Err(err) = require_api_enabled(&state) {
        return json_error(&err);
    }
    let (url, _secret) = state.bus.webhook_config();
    json_ok(json!({ "url": url, "configured": !url.is_empty() }))
}

async fn webhook_post(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> (StatusCode, Json<Value>) {
    if let Err(err) = authorize_call(&headers) {
        return json_error(&err);
    }
    if let Err(err) = require_api_enabled(&state) {
        return json_error(&err);
    }
    let url = body.get("url").and_then(Value::as_str).unwrap_or_default();
    let url = if url.trim().is_empty() {
        String::new()
    } else if url.starts_with("http://") || url.starts_with("https://") {
        url.trim().to_string()
    } else {
        return json_error(&ApiError::bad_request("webhook url must be http(s)"));
    };
    let secret = body
        .get("secret")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    state.bus.set_webhook(&url, &secret);
    // Persist so restarts keep the receiver.
    let mut app_settings = state.settings.current();
    app_settings.webhook_url = url.clone();
    app_settings.webhook_secret = secret;
    let _ = state.settings.update_api(app_settings);
    json_ok(json!({ "url": url, "configured": !url.is_empty() }))
}

// -------------------------------------------------------------------------
// Debug harness (inactive unless ZAPDUDE_DEBUG_EVAL=1: every handler gates
// on the env var AND the API key AND the API-enabled preference, and the
// whole router only listens on loopback).
// -------------------------------------------------------------------------

#[derive(Deserialize)]
struct DebugEvalBody {
    account: Option<String>,
    script: String,
    all: Option<bool>,
}

#[derive(Deserialize)]
struct DebugEmitBody {
    kind: String,
    #[serde(default)]
    data: Value,
}

fn debug_harness_enabled() -> bool {
    matches!(std::env::var("ZAPDUDE_DEBUG_EVAL").as_deref(), Ok("1"))
}

/// Debug harness auth: the same key verification as automation calls but
/// NOT rate limited - debug monitors poll fast during live sessions and
/// must not exhaust the shared 60/min window. The env var + loopback
/// binding stay part of the gate.
fn debug_auth(headers: &HeaderMap) -> Result<(), ApiError> {
    let Some(state_settings) = settings() else {
        return Err(ApiError::internal("app state not initialized"));
    };
    let expected = state_settings.api_key();
    let provided = extract_api_key(
        headers.get("X-API-Key").and_then(|value| value.to_str().ok()),
        headers.get("Authorization").and_then(|value| value.to_str().ok()),
    );
    match provided {
        Some(candidate) if candidate == expected && !expected.is_empty() => Ok(()),
        _ => Err(ApiError::unauthorized()),
    }
}

/// Shared gate for every debug call: env flag, API key, API enabled.
fn debug_gate(state: &ApiState, headers: &HeaderMap) -> Result<(), ApiError> {
    if !debug_harness_enabled() {
        return Err(ApiError::unavailable(
            "debug harness disabled (set ZAPDUDE_DEBUG_EVAL=1)",
        ));
    }
    debug_auth(headers)?;
    require_api_enabled(state)
}

/// Evaluate a JS EXPRESSION inside an account webview (or all of them) and
/// return the JSON-serialized result. This is the live inspection channel:
/// WhatsApp DOM probes, page diagnostics and popup lifecycle experiments
/// run through it while developing. It never ships enabled: the env var
/// must be set at process start.
async fn debug_eval(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(body): Json<DebugEvalBody>,
) -> (StatusCode, Json<Value>) {
    if let Err(err) = debug_gate(&state, &headers) {
        return json_error(&err);
    }
    if body.script.trim().is_empty() {
        return json_error(&ApiError::bad_request("script is required"));
    }
    // The expression rides a defensive wrapper so page side exceptions come
    // back as data instead of transport errors; the stringified result goes
    // through the same double-unwrap transport every bridge call uses.
    let wrapped = format!(
        "JSON.stringify(function(){{ try {{ return ({}); }} catch (error) {{ return {{ eval_error: String(error && error.message || error) }}; }} }}())",
        body.script
    );
    let mut results = serde_json::Map::new();
    if body.all == Some(true) {
        for account in state.settings.current_accounts() {
            let value = match page_for(Some(account.id.as_str())) {
                Some(bridge) => debug_eval_once(&bridge, &wrapped).await,
                None => json!({ "eval_error": "no bridge installed for this account" }),
            };
            results.insert(account.id.clone(), value);
        }
    } else {
        let account_id = match resolve_account_id(&state, body.account.as_deref()) {
            Ok(id) => id,
            Err(err) => return json_error(&err),
        };
        let Some(bridge) = page_for(Some(account_id.as_str())) else {
            return json_error(&ApiError::unavailable("account window is not ready"));
        };
        let value = debug_eval_once(&bridge, &wrapped).await;
        results.insert(account_id.clone(), value);
    }
    json_ok(Value::Object(results))
}

async fn debug_eval_once(
    bridge: &std::sync::Arc<super::page_ops::PageBridge>,
    script: &str,
) -> Value {
    match bridge.evaluate(script.to_string()).await {
        Ok(value) => value,
        Err(err) => json!({ "eval_error": err.message }),
    }
}

/// Enumerate every window and webview the shell knows about: labels, URLs
/// and visibility. This is how live sessions answer "what does the runtime
/// actually look like" (popup call windows land here with popup-* labels).
async fn debug_windows(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> (StatusCode, Json<Value>) {
    if let Err(err) = debug_gate(&state, &headers) {
        return json_error(&err);
    }
    let Some(app_handle) = crate::APP_HANDLE.get() else {
        return json_error(&ApiError::internal("app handle not available"));
    };
    let mut targets: Vec<Value> = Vec::new();
    for (label, window) in app_handle.windows() {
        targets.push(json!({
            "kind": "window",
            "label": label,
            "visible": window.is_visible().unwrap_or(false),
            "minimized": window.is_minimized().unwrap_or(false),
            "title": window.title().unwrap_or_default(),
        }));
    }
    for (label, webview) in app_handle.webviews() {
        targets.push(json!({
            "kind": "webview",
            "label": label,
            "url": webview.url().map(|url| url.to_string()).unwrap_or_default(),
        }));
    }
    json_ok(json!({ "targets": targets }))
}

/// Close one window or webview by label (shell core surfaces are refused):
/// the programmatic way to exercise the popup close path end to end.
async fn debug_close_window(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(label): Path<String>,
) -> (StatusCode, Json<Value>) {
    if let Err(err) = debug_gate(&state, &headers) {
        return json_error(&err);
    }
    if label == "main" || label == "rail" || label == "welcome" {
        return json_error(&ApiError::bad_request("refusing to close a core shell surface"));
    }
    let Some(app_handle) = crate::APP_HANDLE.get() else {
        return json_error(&ApiError::internal("app handle not available"));
    };
    // Account webviews carry page bridges: cancel the bridge before the
    // target goes away (the poller holds a Webview handle).
    if let Some(account_id) = label.strip_prefix("account-") {
        crate::api::page_ops::remove(account_id);
    }
    if let Some(window) = app_handle.get_window(&label) {
        let closed = window.close();
        return json_ok(json!({ "kind": "window", "label": label, "closed": closed.is_ok() }));
    }
    if let Some(webview) = app_handle.get_webview(&label) {
        let closed = webview.close();
        return json_ok(json!({ "kind": "webview", "label": label, "closed": closed.is_ok() }));
    }
    json_error(&ApiError::bad_request(format!(
        "no window or webview labeled '{label}'"
    )))
}

/// Inject a synthetic event through the API bus (SSE, webhook and polling
/// consumers all see it). Used to exercise event consumers without waiting
/// for a real WhatsApp message.
async fn debug_emit(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(body): Json<DebugEmitBody>,
) -> (StatusCode, Json<Value>) {
    if let Err(err) = debug_gate(&state, &headers) {
        return json_error(&err);
    }
    let event = bus().emit(&body.kind, body.data);
    json_ok(json!({ "event": event }))
}
