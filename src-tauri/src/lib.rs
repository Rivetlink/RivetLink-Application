//! RivetLink desktop backend.
//!
//! Thin Tauri command layer over [`rivetlink_sdk::RivetClient`]. The frontend
//! (Vue) drives Client mode: configure the relay, log in, list devices, and
//! capture a screenshot. All protocol/crypto logic lives in the SDK — this
//! file only marshals between the webview and the SDK.

use std::sync::Arc;

use base64::Engine;
use serde::Serialize;
use tauri::{Manager, State};
use tokio::sync::Mutex;

use rivetlink_sdk::{ClientConfig, Device, RivetClient};

/// Shared app state: the (optional) configured client behind an async lock.
#[derive(Default)]
struct AppState {
    client: Arc<Mutex<Option<RivetClient>>>,
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

/// Configure the client against a relay; loads/creates the identity in the
/// app data dir and returns this client's public key (TOFU pin).
#[tauri::command]
async fn init_client(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    relay_ws_url: String,
    relay_http_url: String,
) -> Result<String, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let identity_path = dir.join("client_identity.json");

    let config = ClientConfig {
        relay_ws_url,
        relay_http_url,
        identity_path,
    };
    let client = RivetClient::new(config).map_err(|e| e.to_string())?;
    let public_key = client.public_key();
    *state.client.lock().await = Some(client);
    Ok(public_key)
}

/// Authenticate against the relay.
#[tauri::command]
async fn login(
    state: State<'_, AppState>,
    email: String,
    password: String,
) -> Result<(), String> {
    let mut guard = state.client.lock().await;
    let client = guard.as_mut().ok_or("client not initialized")?;
    client.login(&email, &password).await.map_err(|e| e.to_string())
}

/// List the devices visible to the authenticated user.
#[tauri::command]
async fn list_devices(state: State<'_, AppState>) -> Result<Vec<DeviceDto>, String> {
    let guard = state.client.lock().await;
    let client = guard.as_ref().ok_or("client not initialized")?;
    let devices = client.list_devices().await.map_err(|e| e.to_string())?;
    Ok(devices.into_iter().map(DeviceDto::from).collect())
}

/// Capture one screenshot from `device_id` and return it as a PNG data URL the
/// webview can render directly.
#[tauri::command]
async fn capture_screenshot(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    device_id: String,
) -> Result<String, String> {
    let guard = state.client.lock().await;
    let client = guard.as_ref().ok_or("client not initialized")?;

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
        .manage(AppState::default())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Auto-open the web inspector on debug builds so the console is
            // right there. In release, right-click -> Inspect Element still
            // works thanks to the `devtools` feature.
            #[cfg(debug_assertions)]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }
            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            init_client,
            login,
            list_devices,
            capture_screenshot
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
