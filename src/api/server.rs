//! Loopback HTTP server for the automation API (axum).
//!
//! Endpoints mirror the original fork design (account-aware):
//!   GET  /api/healthz                - availability probe (no key required)
//!   GET  /api/config                 - read API config (no key, localhost only)
//!   POST  /api/config                - save API config (no key, localhost only)
//!   GET  /api/accounts               - configured accounts
//!   POST /api/accounts               - add an account
//!   DELETE /api/accounts/:id         - remove an account
//!   POST /api/accounts/:id/open      - open an account's WhatsApp Web window
//!   POST /api/accounts/:id/hide      - hide an account's WhatsApp Web window
//!   GET  /api/status?account=<id>    - app + page + accounts snapshot
//!   POST /api/open-chat              - {"account?":"work","phone":"55...","text?":"Hi"}
//!   POST /api/message                - {"account?":"work","phone":"55...","message":"Hi"}
//!   GET  /api/chats?limit=20&account= - best-effort visible chat list
//!   GET  /api/events?since=<id>      - polled history (all accounts)
//!   GET  /api/events/stream          - SSE push (all accounts)
//!   GET|POST /api/webhook            - set an HTTPS receiver for events
//!
//! Authentication: `X-API-Key: <key>` or `Authorization: Bearer <key>`.
//! Rate limit: 60 requests / rolling 60 s. Body cap: 1 MB.
//!
//! Management endpoints (/config, /accounts) are always available on
//! localhost so the built-in UI can manage accounts and settings even when
//! the automation API is disabled.

use std::sync::{Arc, OnceLock};

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

use super::events::ApiEventBus;
use super::page_ops::page_for;
use super::protocol::{
    error_payload, extract_api_key, ApiError, LOCAL_API_VERSION, MAX_BODY_BYTES,
};
use crate::config::{self, AccountConfig, SettingsState};
use crate::window::{account_window_label, create_account_with_bridge};
use tauri::Manager;

static SERVER: OnceLock<ServerHolder> = OnceLock::new();
static RATE: OnceLock<super::protocol::RateLimiter> = OnceLock::new();
static BUS: OnceLock<Arc<ApiEventBus>> = OnceLock::new();

pub fn bus() -> Arc<ApiEventBus> {
    BUS.get_or_init(ApiEventBus::new).clone()
}

pub fn rate_limiter() -> &'static super::protocol::RateLimiter {
    RATE.get_or_init(super::protocol::RateLimiter::new)
}

struct ServerHolder {
    shutdown: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Clone)]
pub struct ApiState {
    pub settings: SettingsState,
    pub bus: Arc<ApiEventBus>,
}

fn settings() -> Option<SettingsState> {
    crate::SHARED.get().map(|shared| shared.settings.clone())
}

/// Idempotent: start the loopback server on localhost so the built-in UI
/// can always manage accounts and settings, even when the automation API
/// is disabled by configuration.
pub fn start_loopback_server() {
    if SERVER.get().is_some() {
        return; // already running
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
    // Always bind to localhost for the built-in UI.
    let bind_addr = format!("127.0.0.1:{}", app_settings.port);

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
                match tokio::net::TcpListener::bind(&bind_addr).await {
                    Ok(listener) => {
                        if let Err(err) = axum::serve(listener, app).await {
                            eprintln!("zapdude: API server stopped: {}", err);
                        }
                    }
                    Err(err) => {
                        eprintln!("zapdude: API bind failed on {}: {}", bind_addr, err);
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
            let exit = runtime.block_on(async {
                tokio::select! {
                    _ = serve => 0u8,
                    _ = watcher => 1u8,
                }
            });
            let _ = exit;
            runtime.shutdown_timeout(std::time::Duration::from_secs(2));
        });
    if let Ok(_handle) = thread {
        eprintln!(
            "zapdude: API server on http://127.0.0.1:{} [v{}]",
            app_settings.port, LOCAL_API_VERSION
        );
        let _ = SERVER.set(ServerHolder { shutdown });
    }
}

pub fn shutdown_loopback_server() {
    if let Some(holder) = SERVER.get() {
        holder
            .shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Resolve a target account id: explicit request wins, then the first
/// enabled account.
fn resolve_account_id(
    state: &ApiState,
    requested: Option<&str>,
) -> Result<String, ApiError> {
    let target = requested
        .map(str::trim)
        .filter(|raw| !raw.is_empty());
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

fn router(state: ApiState) -> Router {
    // Flat paths under /api: axum 0.8 removed nesting at "/" (the previous
    // internal `.nest("/")` panicked at runtime: "merge instead").
    Router::new()
        // --- Management endpoints (no auth, localhost only) ---
        .route("/api/healthz", get(healthz))
        .route("/api/config", get(get_config).post(post_config))
        .route("/api/accounts", get(list_accounts).post(add_account_http))
        .route(
            "/api/accounts/{id}",
            delete(remove_account_http),
        )
        .route(
            "/api/accounts/{id}/open",
            post(open_account_http),
        )
        .route(
            "/api/accounts/{id}/hide",
            post(hide_account_http),
        )
        // --- Automation endpoints (require X-API-Key) ---
        .route("/api/status", get(status))
        .route("/api/message", post(message))
        .route("/api/open-chat", post(open_chat))
        .route("/api/chats", get(chats))
        .route("/api/events", get(events))
        .route("/api/events/stream", get(events_stream))
        .route("/api/webhook", get(webhook_get).post(webhook_post))
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
// Management handlers (no auth)
// -------------------------------------------------------------------------

async fn healthz() -> impl IntoResponse {
    let running = SERVER.get().is_some();
    json_ok(json!({
        "service": "zapdude-api",
        "api_version": LOCAL_API_VERSION,
        "server_running": running,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn get_config() -> (StatusCode, Json<Value>) {
    let settings = match settings() {
        Some(s) => s,
        None => {
            return json_error(&ApiError::internal("app state not initialized"));
        }
    };
    let api = settings.current();
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
    // Siempre shutdown primero para que cambios de puerto surtan efecto inmediato.
    shutdown_loopback_server();
    if api.enabled {
        start_loopback_server();
    }
    json_ok(json!({
        "enabled": api.enabled,
        "port": api.port,
        "api_key": api.api_key,
        "webhook_url": api.webhook_url,
        "webhook_secret": api.webhook_secret,
    }))
}

async fn list_accounts() -> (StatusCode, Json<Value>) {
    let settings = match settings() {
        Some(s) => s,
        None => {
            return json_error(&ApiError::internal("app state not initialized"));
        }
    };
    let account_entries: Vec<Value> = settings
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
    json_ok(json!({ "accounts": account_entries }))
}

#[derive(Deserialize)]
struct AddAccountBody {
    id: String,
    label: String,
}

async fn add_account_http(
    State(state): State<ApiState>,
    Json(body): Json<AddAccountBody>,
) -> (StatusCode, Json<Value>) {
    let id = body.id.trim().to_string();
    let label = body.label.trim().to_string();
    if id.is_empty() {
        return json_error(&ApiError::bad_request("account id is required"));
    }
    let accounts: Vec<AccountConfig> = state
        .settings
        .current_accounts()
        .iter()
        .filter(|a| a.id != id)
        .cloned()
        .collect();
    if accounts.iter().any(|a| a.id == id) {
        return json_error(&ApiError::bad_request(format!("account '{id}' already exists")));
    }
    let mut new_accounts = accounts;
    new_accounts.push(AccountConfig {
        id: id.clone(),
        label: label.clone(),
        enabled: true,
    });
    if let Err(err) = state.settings.update_api(config::ApiConfig {
        enabled: state.settings.current().enabled,
        port: state.settings.current().port,
        api_key: state.settings.current().api_key,
        expose_network: false,
        webhook_url: state.settings.current().webhook_url,
        webhook_secret: state.settings.current().webhook_secret,
    }) {
        return json_error(&ApiError::internal(err));
    }
    if let Err(err) = state.settings.update_accounts(new_accounts.clone()) {
        return json_error(&ApiError::internal(err));
    }
    // Create the hidden window for the new account.
    if let Some(app_handle) = crate::APP_HANDLE.get() {
        let account = AccountConfig {
            id: id.clone(),
            label: label.clone(),
            enabled: true,
        };
        if let Err(err) = create_account_with_bridge(app_handle, &account, false) {
            eprintln!(
                "zapdude: could not create hidden window for account '{}': {}",
                account.id, err
            );
        }
    }
    json_ok(json!({ "id": id, "label": label }))
}

async fn remove_account_http(
    Path(id): Path<String>,
) -> (StatusCode, Json<Value>) {
    let settings = match settings() {
        Some(s) => s,
        None => {
            return json_error(&ApiError::internal("app state not initialized"));
        }
    };
    let before = settings.current_accounts().len();
    let accounts: Vec<AccountConfig> = settings
        .current_accounts()
        .into_iter()
        .filter(|a| a.id != id)
        .collect();
    if accounts.len() == before {
        return json_error(&ApiError::bad_request(format!("account '{id}' not found")));
    }
    if let Err(err) = settings.update_accounts(accounts) {
        return json_error(&ApiError::internal(err));
    }
    // Hide the window if it exists.
    if let Some(app_handle) = crate::APP_HANDLE.get() {
        if let Some(window) = app_handle.get_webview_window(&account_window_label(&id)) {
            let _ = window.hide();
        }
    }
    json_ok(json!({ "id": id }))
}

async fn open_account_http(
    Path(id): Path<String>,
) -> (StatusCode, Json<Value>) {
    let settings = match settings() {
        Some(s) => s,
        None => {
            return json_error(&ApiError::internal("app state not initialized"));
        }
    };
    let account = match settings.account(&id) {
        Some(a) => a,
        None => {
            return json_error(&ApiError::bad_request(format!("account '{id}' not found")));
        }
    };
    let app_handle = match crate::APP_HANDLE.get() {
        Some(h) => h.clone(),
        None => {
            return json_error(&ApiError::internal("app handle not available"));
        }
    };
    // If window exists, show it; otherwise create it.
    if let Some(window) = app_handle.get_webview_window(&account_window_label(&id)) {
        if let Err(err) = window.show() {
            return json_error(&ApiError::internal(err.to_string()));
        }
        if let Err(err) = window.set_focus() {
            return json_error(&ApiError::internal(err.to_string()));
        }
    } else {
        if let Err(err) = create_account_with_bridge(&app_handle, &account, true) {
            return json_error(&ApiError::internal(err.to_string()));
        }
    }
    json_ok(json!({ "id": id, "opened": true }))
}

async fn hide_account_http(
    Path(id): Path<String>,
) -> (StatusCode, Json<Value>) {
    let app_handle = match crate::APP_HANDLE.get() {
        Some(h) => h.clone(),
        None => {
            return json_error(&ApiError::internal("app handle not available"));
        }
    };
    if let Some(window) = app_handle.get_webview_window(&account_window_label(&id)) {
        if let Err(err) = window.hide() {
            return json_error(&ApiError::internal(err.to_string()));
        }
        json_ok(json!({ "id": id, "hidden": true }))
    } else {
        json_error(&ApiError::bad_request(format!("window for account '{id}' not found")))
    }
}

// -------------------------------------------------------------------------
// Automation handlers (require auth)
// -------------------------------------------------------------------------

async fn status(
    State(state): State<ApiState>,
    headers: HeaderMap,
    RawQuery(raw): RawQuery,
) -> (StatusCode, Json<Value>) {
    if let Err(err) = authorize_call(&headers) {
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
        "name": "ZapZap",
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
    let phone = body.get("phone").and_then(Value::as_str).unwrap_or("");
    if phone.is_empty() {
        return json_error(&ApiError::bad_request("phone is required"));
    }
    let text = body.get("text").and_then(Value::as_str);
    let account_id = match resolve_account_id(
        &state,
        body.get("account").and_then(Value::as_str),
    ) {
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
    let phone = body.get("phone").and_then(Value::as_str).unwrap_or("");
    let text = body.get("message").and_then(Value::as_str).unwrap_or("");
    if phone.is_empty() || text.is_empty() {
        return json_error(&ApiError::bad_request("phone and message are required"));
    }
    let account_id = match resolve_account_id(
        &state,
        body.get("account").and_then(Value::as_str),
    ) {
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

async fn events(
    State(state): State<ApiState>,
    headers: HeaderMap,
    RawQuery(raw): RawQuery,
) -> (StatusCode, Json<Value>) {
    if let Err(err) = authorize_call(&headers) {
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
    let url = body
        .get("url")
        .and_then(Value::as_str)
        .unwrap_or_default();
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
