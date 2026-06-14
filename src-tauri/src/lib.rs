//! RivetLink desktop backend.
//!
//! Tauri command layer over [`rivetlink_sdk`] plus a small persistence layer
//! for app settings (saved relays, this device's name, enabled roles). The
//! Vue frontend drives onboarding, then Client mode (connect to the active
//! relay, sign in, list devices, capture a screenshot). Host mode and
//! session-code pairing are wired in as the backend lands.

mod settings;

use std::path::PathBuf;
use std::sync::Arc;

use base64::Engine;
use serde::Serialize;
use tauri::{Manager, State};
use tokio::sync::Mutex;

use rivetlink_sdk::{ClientConfig, Device, Identity, RivetClient};

use settings::{AppSettings, Relay};

/// Shared app state.
struct AppState {
    /// Where settings + identity live (the OS app-data dir).
    data_dir: PathBuf,
    /// Persisted settings, loaded at startup.
    settings: Mutex<AppSettings>,
    /// The connected client (after `connect`).
    client: Arc<Mutex<Option<RivetClient>>>,
}

impl AppState {
    fn identity_path(&self) -> PathBuf {
        self.data_dir.join("client_identity.json")
    }
}

/// Device shape sent to the frontend (no key material).
#[derive(Serialize)]
struct DeviceDto {
    id: String,
    hostname: Option<String>,
    platform: Option<String>,
    last_seen: Option<String>,
}

impl From<Device> for DeviceDto {
    fn from(d: Device) -> Self {
        Self {
            id: d.id,
            hostname: d.hostname,
            platform: d.platform,
            last_seen: d.last_seen,
        }
    }
}

// ---- Settings / onboarding -------------------------------------------------

/// Read the persisted settings (drives the onboarding gate + every page).
#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.lock().await.clone())
}

/// Finish onboarding: store this device's name + enabled roles.
#[tauri::command]
async fn complete_setup(
    state: State<'_, AppState>,
    device_name: String,
    roles: Vec<String>,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().await;
    settings.device_name = device_name.trim().to_string();
    settings.roles = roles;
    settings.setup_complete = true;
    settings.save(&state.data_dir)?;
    Ok(settings.clone())
}

/// This client's identity public key (base64) — what a host trusts (TOFU).
#[tauri::command]
async fn public_key(state: State<'_, AppState>) -> Result<String, String> {
    let identity = Identity::load_or_create(&state.identity_path()).map_err(|e| e.to_string())?;
    Ok(identity.public_key_b64())
}

/// Toggle the web inspector. Bound to a keyboard shortcut in the frontend so
/// the console is off by default but one keystroke away.
#[tauri::command]
fn toggle_devtools(window: tauri::WebviewWindow) {
    if window.is_devtools_open() {
        window.close_devtools();
    } else {
        window.open_devtools();
    }
}

// ---- Relays ----------------------------------------------------------------

/// Derive the WebSocket URL from the server's HTTP URL: http -> ws, https ->
/// wss, same host/port, with the relay's `/ws` signaling path. The user only
/// enters the HTTP URL.
fn derive_ws_url(http_url: &str) -> Result<String, String> {
    let trimmed = http_url.trim().trim_end_matches('/');
    let ws_base = if let Some(rest) = trimmed.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        return Err("server-URL moet met http:// of https:// beginnen".to_string());
    };
    Ok(format!("{ws_base}/ws"))
}

/// Add a saved relay from just the HTTP URL (the WS URL is derived). Becomes
/// active if it's the first one saved.
#[tauri::command]
async fn add_relay(
    state: State<'_, AppState>,
    name: String,
    http_url: String,
) -> Result<AppSettings, String> {
    let ws_url = derive_ws_url(&http_url)?;

    // Validate via the SDK's config rules before saving.
    ClientConfig {
        relay_http_url: http_url.trim().to_string(),
        relay_ws_url: ws_url.clone(),
        identity_path: state.identity_path(),
    }
    .validate()
    .map_err(|e| e.to_string())?;

    let mut settings = state.settings.lock().await;
    let relay = Relay {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.trim().to_string(),
        http_url: http_url.trim().trim_end_matches('/').to_string(),
        ws_url,
    };
    if settings.active_relay_id.is_none() {
        settings.active_relay_id = Some(relay.id.clone());
    }
    settings.relays.push(relay);
    settings.save(&state.data_dir)?;
    Ok(settings.clone())
}

/// Remove a saved relay (clears the active selection if it was active).
#[tauri::command]
async fn remove_relay(state: State<'_, AppState>, id: String) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().await;
    settings.relays.retain(|r| r.id != id);
    if settings.active_relay_id.as_deref() == Some(id.as_str()) {
        settings.active_relay_id = settings.relays.first().map(|r| r.id.clone());
    }
    settings.save(&state.data_dir)?;
    Ok(settings.clone())
}

/// Select which saved relay is active.
#[tauri::command]
async fn set_active_relay(state: State<'_, AppState>, id: String) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().await;
    if !settings.relays.iter().any(|r| r.id == id) {
        return Err("unknown relay".to_string());
    }
    settings.active_relay_id = Some(id);
    settings.save(&state.data_dir)?;
    Ok(settings.clone())
}

// ---- Client session --------------------------------------------------------

/// Build a client bound to the active relay; returns this client's public key.
#[tauri::command]
async fn connect(state: State<'_, AppState>) -> Result<String, String> {
    let config = {
        let settings = state.settings.lock().await;
        let relay = settings
            .active_relay()
            .ok_or("no active relay — add one first")?;
        ClientConfig {
            relay_http_url: relay.http_url.clone(),
            relay_ws_url: relay.ws_url.clone(),
            identity_path: state.identity_path(),
        }
    };

    let client = RivetClient::new(config).map_err(|e| e.to_string())?;
    let public_key = client.public_key();
    *state.client.lock().await = Some(client);
    Ok(public_key)
}

/// Authenticate against the active relay.
#[tauri::command]
async fn login(state: State<'_, AppState>, email: String, password: String) -> Result<(), String> {
    let mut guard = state.client.lock().await;
    let client = guard.as_mut().ok_or("not connected — call connect first")?;
    client.login(&email, &password).await.map_err(|e| e.to_string())
}

/// List the devices visible to the authenticated user.
#[tauri::command]
async fn list_devices(state: State<'_, AppState>) -> Result<Vec<DeviceDto>, String> {
    let guard = state.client.lock().await;
    let client = guard.as_ref().ok_or("not connected")?;
    let devices = client.list_devices().await.map_err(|e| e.to_string())?;
    Ok(devices.into_iter().map(DeviceDto::from).collect())
}

/// Capture one screenshot from `device_id`, returned as a PNG data URL.
#[tauri::command]
async fn capture_screenshot(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    device_id: String,
) -> Result<String, String> {
    let guard = state.client.lock().await;
    let client = guard.as_ref().ok_or("not connected")?;

    let device = client.find_device(&device_id).await.map_err(|e| e.to_string())?;

    let dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let out = dir.join("last_capture.png");

    let path = client
        .capture_screenshot(&device, out)
        .await
        .map_err(|e| e.to_string())?;

    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:image/png;base64,{b64}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Resolve the data dir, load settings, and seed the shared state.
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let settings = AppSettings::load(&data_dir);
            app.manage(AppState {
                data_dir,
                settings: Mutex::new(settings),
                client: Arc::new(Mutex::new(None)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            complete_setup,
            public_key,
            toggle_devtools,
            add_relay,
            remove_relay,
            set_active_relay,
            connect,
            login,
            list_devices,
            capture_screenshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
