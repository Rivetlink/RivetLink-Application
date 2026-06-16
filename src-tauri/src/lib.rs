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
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{Emitter, Manager, State};
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
    /// The task pumping frames for the active LAN live-stream, if any. A sync
    /// mutex so the viewer window's close handler can stop it without awaiting.
    stream: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

/// A saved LAN device the frontend asks to connect to.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LanTarget {
    name: String,
    address: String,
    port: u16,
    device_id: String,
    public_key: Option<String>,
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

/// Update this device's name and enabled roles after onboarding.
#[tauri::command]
async fn update_device(
    state: State<'_, AppState>,
    device_name: String,
    roles: Vec<String>,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().await;
    settings.device_name = device_name.trim().to_string();
    settings.roles = roles;
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

/// This app's version (from Cargo.toml), shown in the About dialog.
#[tauri::command]
fn app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

/// Whether the app is running from an AppImage bundle. The AppImage runtime
/// sets `$APPIMAGE` to the bundle path; the updater can replace that file in
/// place. A `.deb`/`.rpm` install lives in `/usr` (root-owned) and can't, so
/// the frontend falls back to notify-only there. Always false off Linux.
#[tauri::command]
fn is_appimage() -> bool {
    cfg!(target_os = "linux") && std::env::var_os("APPIMAGE").is_some()
}

/// Build the native menu bar: a "RivetLink" menu (version + check for updates +
/// quit) and a standard "Edit" menu so clipboard shortcuts work everywhere.
fn install_menu(app: &tauri::App) -> tauri::Result<()> {
    let version = app.package_info().version.to_string();

    let version_item = MenuItemBuilder::new(format!("Version {version}"))
        .id("version")
        .enabled(false)
        .build(app)?;
    let check_updates = MenuItemBuilder::new("Check for Updates…")
        .id("check_updates")
        .build(app)?;

    let rivetlink_menu = SubmenuBuilder::new(app, "RivetLink")
        .item(&version_item)
        .item(&check_updates)
        .separator()
        .quit()
        .build()?;

    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let menu = MenuBuilder::new(app)
        .item(&rivetlink_menu)
        .item(&edit_menu)
        .build()?;

    app.set_menu(menu)?;
    Ok(())
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

// ---- Direct-LAN: discover, remember, connect -------------------------------

/// Browse the local network for RivetLink hosts (~3s), excluding this device's
/// own advertisement (matched by identity public key).
#[tauri::command]
async fn discover_lan(
    state: State<'_, AppState>,
) -> Result<Vec<rivetlink_sdk::LanDevice>, String> {
    let own_key = Identity::load_or_create(&state.identity_path())
        .map(|id| id.public_key_b64())
        .ok();

    let mut found = rivetlink_sdk::lan::discover(std::time::Duration::from_secs(3))
        .await
        .map_err(|e| e.to_string())?;

    // Drop our own host advert so we never list ourselves.
    found.retain(|d| match (&d.public_key, &own_key) {
        (Some(pk), Some(own)) => pk != own,
        _ => true,
    });
    Ok(found)
}

/// Remember a discovered LAN host so it shows up without re-scanning.
#[tauri::command]
async fn add_lan_device(
    state: State<'_, AppState>,
    name: String,
    address: String,
    port: u16,
    public_key: Option<String>,
) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().await;
    // De-duplicate on address:port; refresh the stored entry if it exists.
    settings
        .lan_devices
        .retain(|d| !(d.address == address && d.port == port));
    settings.lan_devices.push(settings::SavedLanDevice {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.trim().to_string(),
        address,
        port,
        public_key,
    });
    settings.save(&state.data_dir)?;
    Ok(settings.clone())
}

/// Forget a remembered LAN host.
#[tauri::command]
async fn remove_lan_device(state: State<'_, AppState>, id: String) -> Result<AppSettings, String> {
    let mut settings = state.settings.lock().await;
    settings.lan_devices.retain(|d| d.id != id);
    settings.save(&state.data_dir)?;
    Ok(settings.clone())
}

/// Connect to a LAN host and capture one screenshot, returned as a PNG data URL.
///
/// With a `pin`, authenticates via PAKE. Without one, uses this device's
/// identity (TOFU); if the host's public key is known it is pinned to defeat a
/// man-in-the-middle.
#[tauri::command]
async fn lan_screenshot(
    state: State<'_, AppState>,
    address: String,
    port: u16,
    pin: Option<String>,
    host_public_key: Option<String>,
) -> Result<String, String> {
    let addr: std::net::SocketAddr = format!("{address}:{port}")
        .parse()
        .map_err(|_| format!("bad address: {address}:{port}"))?;

    let png = match pin {
        Some(pin) if !pin.trim().is_empty() => {
            rivetlink_sdk::lan::screenshot_password(addr, pin.trim())
                .await
                .map_err(|e| e.to_string())?
        },
        _ => {
            let identity =
                Identity::load_or_create(&state.identity_path()).map_err(|e| e.to_string())?;
            rivetlink_sdk::lan::screenshot_key_pinned(addr, &identity, host_public_key.as_deref())
                .await
                .map_err(|e| e.to_string())?
        },
    };

    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
    Ok(format!("data:image/png;base64,{b64}"))
}

/// Open (or focus) the standalone viewer window that renders the live stream.
/// Closing the window stops the stream.
fn open_viewer(app: &tauri::AppHandle, title: &str) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("viewer") {
        let _ = win.set_focus();
        return Ok(());
    }
    let window = tauri::WebviewWindowBuilder::new(
        app,
        "viewer",
        tauri::WebviewUrl::App("index.html#/viewer".into()),
    )
    .title(title)
    .inner_size(1280.0, 800.0)
    .min_inner_size(640.0, 400.0)
    .build()
    .map_err(|e| e.to_string())?;

    // Closing the viewer disconnects the live stream.
    let handle = app.clone();
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
            if let Some(state) = handle.try_state::<AppState>() {
                stop_stream(&state);
            }
            let _ = handle.emit("lan://disconnected", ());
        }
    });
    Ok(())
}

/// Connect to a LAN host and open a live screen stream in its own window.
///
/// Frames are emitted to the frontend as `lan://frame` (a JPEG data URL); the
/// viewer window renders them. `lan://connected` / `lan://disconnected` drive
/// the "connected" badge in the main window.
#[tauri::command]
async fn lan_connect(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    target: LanTarget,
    pin: Option<String>,
) -> Result<(), String> {
    let addr: std::net::SocketAddr = format!("{}:{}", target.address, target.port)
        .parse()
        .map_err(|_| format!("bad address: {}:{}", target.address, target.port))?;

    // Establish the encrypted channel (PIN/PAKE, or key/TOFU with host pinning).
    let (mut stream, channel) = match pin {
        Some(pin) if !pin.trim().is_empty() => {
            rivetlink_sdk::lan::connect_password(addr, pin.trim())
                .await
                .map_err(|e| e.to_string())?
        },
        _ => {
            let identity =
                Identity::load_or_create(&state.identity_path()).map_err(|e| e.to_string())?;
            rivetlink_sdk::lan::connect_key_pinned(addr, &identity, target.public_key.as_deref())
                .await
                .map_err(|e| e.to_string())?
        },
    };

    // Stop any previous session before starting a new one.
    stop_stream(&state);
    open_viewer(&app, &format!("RivetLink — {}", target.name))?;

    let app_for_task = app.clone();
    let task = tokio::spawn(async move {
        let result = rivetlink_sdk::lan::stream_frames(&mut stream, &channel, 20, |delta| {
            // Forward the delta frame to the viewer window. If emit fails the
            // window is gone — stop the stream.
            app_for_task.emit("lan://frame", delta).is_ok()
        })
        .await;
        if let Err(e) = result {
            let _ = app_for_task.emit("lan://error", e.to_string());
        }
        let _ = app_for_task.emit("lan://disconnected", ());
    });

    if let Ok(mut guard) = state.stream.lock() {
        *guard = Some(task);
    }
    let _ = app.emit("lan://connected", target.device_id);
    Ok(())
}

/// Stop the active stream (if any) and abort its task. Sync so it can run from
/// a window-event handler.
fn stop_stream(state: &AppState) {
    if let Ok(mut guard) = state.stream.lock() {
        if let Some(task) = guard.take() {
            task.abort();
        }
    }
}

/// Disconnect the current LAN stream and close the viewer window.
#[tauri::command]
async fn lan_disconnect(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    stop_stream(&state);
    if let Some(win) = app.get_webview_window("viewer") {
        let _ = win.close();
    }
    let _ = app.emit("lan://disconnected", ());
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // Resolve the data dir, load settings, and seed the shared state.
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let settings = AppSettings::load(&data_dir);
            app.manage(AppState {
                data_dir,
                settings: Mutex::new(settings),
                client: Arc::new(Mutex::new(None)),
                stream: std::sync::Mutex::new(None),
            });

            // Native menu bar (RivetLink + Edit). The "Check for Updates" item
            // forwards to the frontend, which does the actual version check.
            install_menu(app)?;
            app.on_menu_event(|app, event| {
                if event.id() == "check_updates" {
                    let _ = app.emit("menu://check-updates", ());
                }
            });

            // Open the web inspector from the Rust side when RIVETLINK_DEVTOOLS
            // is set. This works even if the frontend never mounts (white
            // screen), unlike the in-app keyboard shortcut.
            if std::env::var("RIVETLINK_DEVTOOLS").is_ok() {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            complete_setup,
            update_device,
            public_key,
            app_version,
            is_appimage,
            toggle_devtools,
            add_relay,
            remove_relay,
            set_active_relay,
            connect,
            login,
            list_devices,
            capture_screenshot,
            discover_lan,
            add_lan_device,
            remove_lan_device,
            lan_screenshot,
            lan_connect,
            lan_disconnect
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
