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

#[cfg(any(target_os = "linux", target_os = "macos"))]
use rivetlink_agent::lan::{serve_with_events, HostEvent, LanAuth};
use rivetlink_sdk::{ClientConfig, Device, Identity, RivetClient};

use settings::{AppSettings, Relay, TrustedKey};

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
    /// Sends display-switch requests to the active live-stream task. Set while a
    /// stream is running; the client uses it to switch which host screen it views
    /// (honoured by macOS hosts).
    switch_tx: std::sync::Mutex<Option<tokio::sync::mpsc::Sender<u32>>>,
    /// The active "receive help" host session (advertise + serve), if any.
    host: std::sync::Mutex<Option<HostSession>>,
    /// Trusted client identity keys (base64), shared live with the running host
    /// so add/remove takes effect without restarting it. Mirrors
    /// `settings.trusted_keys`.
    trusted_keys: Arc<std::sync::Mutex<Vec<String>>>,
    /// Reused `sysinfo` handle for the Resources page. Kept alive so each poll
    /// measures CPU use since the previous one (a single refresh can't).
    sys: std::sync::Mutex<sysinfo::System>,
}

/// A running host session: the accept-loop task and the task forwarding its
/// lifecycle events to the frontend. Aborting both stops hosting and (by
/// dropping the listener) unregisters the mDNS advertisement.
struct HostSession {
    serve: tokio::task::JoinHandle<()>,
    forward: tokio::task::JoinHandle<()>,
    /// The PIN the helper must enter; shown on the host's screen.
    pin: String,
    /// Label of the currently connected client (`None` = nobody). Kept in sync
    /// by the event-forwarding task so the UI can restore the right "connected /
    /// waiting" state after navigating away and back, when it missed the live
    /// `host://connected|disconnected` events.
    peer: Arc<std::sync::Mutex<Option<String>>>,
}

impl HostSession {
    fn stop(self) {
        self.serve.abort();
        self.forward.abort();
    }
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
    for d in &found {
        tracing::info!(name = %d.name, address = %d.address, port = d.port, "discover_lan: host");
    }
    tracing::info!(count = found.len(), "discover_lan: done");
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
    // Build the address from the parsed IP so IPv6 is handled correctly —
    // `format!("{}:{}", v6, port)` produces an unbracketed, unparseable string.
    let ip: std::net::IpAddr = target
        .address
        .parse()
        .map_err(|_| format!("bad address: {}", target.address))?;
    let addr = std::net::SocketAddr::new(ip, target.port);
    let has_pin = pin.as_deref().map(|p| !p.trim().is_empty()).unwrap_or(false);
    tracing::info!(
        name = %target.name,
        %addr,
        has_pin,
        pinned_host = target.public_key.is_some(),
        "lan_connect: connecting"
    );

    // Establish the encrypted channel (PIN/PAKE, or key/TOFU with host pinning),
    // bounded by a timeout so an unreachable host fails instead of hanging the
    // UI on "Connecting…" forever.
    let connect = async {
        match &pin {
            Some(p) if !p.trim().is_empty() => rivetlink_sdk::lan::connect_password(addr, p.trim())
                .await
                .map_err(|e| e.to_string()),
            _ => {
                let identity =
                    Identity::load_or_create(&state.identity_path()).map_err(|e| e.to_string())?;
                rivetlink_sdk::lan::connect_key_pinned(addr, &identity, target.public_key.as_deref())
                    .await
                    .map_err(|e| e.to_string())
            },
        }
    };
    let (mut stream, channel) =
        tokio::time::timeout(std::time::Duration::from_secs(15), connect)
            .await
            .map_err(|_| {
                tracing::warn!(%addr, "lan_connect: timed out after 15s");
                "connection timed out".to_string()
            })?
            .inspect_err(|e| tracing::warn!(%addr, error = %e, "lan_connect: handshake failed"))?;
    tracing::info!(%addr, "lan_connect: channel established, opening viewer");

    // Stop any previous session before starting a new one.
    stop_stream(&state);
    open_viewer(&app, &format!("RivetLink — {}", target.name))?;

    // Ask the host which screens it can share (empty on Linux hosts, where the
    // portal owns selection) and hand it to the viewer for a screen picker.
    // Bounded too: an unresponsive host shouldn't stall the stream from starting.
    let displays = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        rivetlink_sdk::lan::list_displays(&mut stream, &channel),
    )
    .await
    {
        Ok(Ok(d)) => d,
        _ => Vec::new(),
    };
    let _ = app.emit("lan://displays", &displays);

    // The viewer's screen picker pushes display ids here to switch mid-stream.
    let (switch_tx, switch_rx) = tokio::sync::mpsc::channel::<u32>(4);
    if let Ok(mut guard) = state.switch_tx.lock() {
        *guard = Some(switch_tx);
    }

    let app_for_task = app.clone();
    let task = tokio::spawn(async move {
        let result =
            rivetlink_sdk::lan::stream_frames(stream, channel, 20, None, switch_rx, |delta| {
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
    tracing::info!(displays = displays.len(), "lan_connect: streaming");
    Ok(())
}

/// Switch the live stream to another of the host's displays (by id). No-op if
/// nothing is streaming; honoured by macOS hosts (Linux can't switch).
#[tauri::command]
async fn lan_switch_display(state: State<'_, AppState>, display: u32) -> Result<(), String> {
    // Clone the sender out and drop the (sync) lock before awaiting the send.
    let sender = state.switch_tx.lock().ok().and_then(|g| g.as_ref().cloned());
    if let Some(tx) = sender {
        tx.send(display).await.map_err(|_| "stream is not running".to_string())?;
    }
    Ok(())
}

/// The app's own CPU + memory use, for the Resources page.
#[derive(serde::Serialize)]
struct ResourceUsage {
    /// CPU as a fraction of the whole machine (0–100). sysinfo reports per-core
    /// usage, so we divide by the core count — a busy single thread on an
    /// 8-core box reads ~12%, not 100%.
    cpu_percent: f32,
    /// Resident memory of the app process, in bytes.
    mem_bytes: u64,
    /// Total physical memory on the machine, in bytes.
    total_mem_bytes: u64,
    /// Logical CPU cores.
    cores: usize,
}

/// Sample the app process's current CPU + memory. CPU is measured since the
/// previous call (sysinfo needs two refreshes), so the first sample reads ~0.
#[tauri::command]
fn resource_usage(state: State<'_, AppState>) -> Result<ResourceUsage, String> {
    let pid = sysinfo::get_current_pid().map_err(|e| e.to_string())?;
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());

    let mut sys = state.sys.lock().map_err(|_| "resource lock poisoned".to_string())?;
    sys.refresh_process(pid);
    sys.refresh_memory();

    let (raw_cpu, mem_bytes) = sys
        .process(pid)
        .map_or((0.0, 0), |p| (p.cpu_usage(), p.memory()));

    Ok(ResourceUsage {
        cpu_percent: raw_cpu / cores as f32,
        mem_bytes,
        total_mem_bytes: sys.total_memory(),
        cores,
    })
}

/// Stop the active stream (if any) and abort its task. Sync so it can run from
/// a window-event handler.
fn stop_stream(state: &AppState) {
    if let Ok(mut guard) = state.stream.lock() {
        if let Some(task) = guard.take() {
            task.abort();
        }
    }
    // Drop the switch sender so a stale picker can't talk to a dead stream.
    if let Ok(mut guard) = state.switch_tx.lock() {
        *guard = None;
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

// ---- Receive help (host mode) ----------------------------------------------

/// Start hosting on the local network so a helper can connect and view this
/// screen. Advertises over mDNS and serves an encrypted session guarded by a
/// freshly generated PIN, which is returned and shown to the user to read out.
///
/// Session lifecycle is emitted to the frontend: `host://connected` (with the
/// peer label) and `host://disconnected`. Hosting runs until [`stop_host`].
///
/// Windows has no host backend yet (scap's Windows capture doesn't build), so
/// there it returns an error and the app stays client/viewer only.
#[cfg(target_os = "windows")]
#[tauri::command]
async fn start_host(_app: tauri::AppHandle, _state: State<'_, AppState>) -> Result<String, String> {
    Err("Sharing this screen isn't supported on Windows yet.".to_string())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[tauri::command]
async fn start_host(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    // A 6-digit PIN. SPAKE2 makes a wrong PIN fail the handshake, and resists
    // offline guessing, so 6 digits is enough for a short-lived LAN session.
    let pin = format!("{:06}", rand::Rng::gen_range(&mut rand::thread_rng(), 0..1_000_000));

    let device_name = {
        let s = state.settings.lock().await;
        let name = s.device_name.trim();
        if name.is_empty() {
            "RivetLink".to_string()
        } else {
            name.to_string()
        }
    };

    let signing_key = Identity::load_or_create(&state.identity_path())
        .map_err(|e| e.to_string())?
        .signing_key()
        .clone();
    let trusted_keys = Arc::clone(&state.trusted_keys);

    // Replace any previous session.
    stop_host_inner(&state);

    let (tx, mut rx) = tokio::sync::mpsc::channel::<HostEvent>(8);

    let app_for_serve = app.clone();
    let serve_pin = pin.clone();
    let serve = tokio::spawn(async move {
        // Accept either the session PIN or a trusted client's key (empty PIN).
        let auth = LanAuth::PinOrKey {
            pin: serve_pin,
            trusted_keys,
            auto_accept: false,
        };
        let port = rivetlink_sdk::lan::DEFAULT_LAN_PORT;
        tracing::info!(%device_name, port, "start_host: advertising + serving on LAN");
        if let Err(e) = serve_with_events(signing_key, device_name, port, auth, Some(tx)).await {
            tracing::warn!(error = %e, "start_host: serve loop ended with error");
            let _ = app_for_serve.emit("host://error", e.to_string());
        }
    });

    // Shared truth for "who is connected", read by `host_active` so the UI can
    // resync after missing a live event.
    let peer = Arc::new(std::sync::Mutex::new(None::<String>));

    let app_for_fwd = app.clone();
    let peer_for_fwd = Arc::clone(&peer);
    let forward = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                HostEvent::ClientConnected(label) => {
                    if let Ok(mut p) = peer_for_fwd.lock() {
                        *p = Some(label.clone());
                    }
                    let _ = app_for_fwd.emit("host://connected", label);
                },
                HostEvent::ClientDisconnected => {
                    if let Ok(mut p) = peer_for_fwd.lock() {
                        *p = None;
                    }
                    let _ = app_for_fwd.emit("host://disconnected", ());
                },
            }
        }
    });

    if let Ok(mut guard) = state.host.lock() {
        *guard = Some(HostSession {
            serve,
            forward,
            pin: pin.clone(),
            peer,
        });
    }
    Ok(pin)
}

/// Abort the active host session (if any). Sync helper so it can run from a
/// window/close handler.
fn stop_host_inner(state: &AppState) {
    if let Ok(mut guard) = state.host.lock() {
        if let Some(session) = guard.take() {
            session.stop();
        }
    }
}

/// Stop hosting: tear down the session and stop advertising.
#[tauri::command]
async fn stop_host(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    stop_host_inner(&state);
    let _ = app.emit("host://stopped", ());
    Ok(())
}

/// The host's live state: the active PIN (if hosting) and the currently
/// connected client label (if any). Lets the UI restore the correct
/// "connected / waiting / idle" state after navigating away and back, when it
/// missed the live `host://connected|disconnected` events.
#[derive(Serialize)]
struct HostState {
    pin: Option<String>,
    peer: Option<String>,
}

#[tauri::command]
async fn host_active(state: State<'_, AppState>) -> Result<HostState, String> {
    let guard = state.host.lock().map_err(|_| "host lock poisoned".to_string())?;
    let (pin, peer) = match guard.as_ref() {
        Some(session) => (
            Some(session.pin.clone()),
            session.peer.lock().ok().and_then(|p| p.clone()),
        ),
        None => (None, None),
    };
    Ok(HostState { pin, peer })
}

// ---- Network info ----------------------------------------------------------

/// The current Wi-Fi SSID (if on Wi-Fi) and this machine's LAN IP. Shown in the
/// LAN tab so the user can confirm both devices share a network/subnet — mDNS
/// discovery only works within one broadcast domain.
#[derive(Serialize)]
struct NetworkInfo {
    ssid: Option<String>,
    ip: Option<String>,
}

#[tauri::command]
async fn network_info() -> Result<NetworkInfo, String> {
    tokio::task::spawn_blocking(|| NetworkInfo {
        ssid: current_ssid(),
        ip: local_ip(),
    })
    .await
    .map_err(|e| e.to_string())
}

/// Reachability check for a saved LAN host: can we open a TCP connection to its
/// listener? `true` = online/connectable. A short timeout keeps the per-device
/// status snappy and never blocks the UI.
#[tauri::command]
async fn lan_ping(address: String, port: u16) -> Result<bool, String> {
    // Parse the IP first so IPv6 builds a valid SocketAddr (a bare
    // `format!("{v6}:{port}")` is unbracketed and won't parse).
    let Ok(ip) = address.parse::<std::net::IpAddr>() else {
        return Ok(false);
    };
    let addr = std::net::SocketAddr::new(ip, port);
    let connect = tokio::net::TcpStream::connect(addr);
    let ok = matches!(
        tokio::time::timeout(std::time::Duration::from_millis(1500), connect).await,
        Ok(Ok(_))
    );
    tracing::debug!(%addr, ok, "lan_ping");
    Ok(ok)
}

/// This machine's LAN IP, found by asking the OS which local address would
/// route to a public host. No packet is sent (UDP connect only sets the route).
fn local_ip() -> Option<String> {
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    sock.local_addr().ok().map(|a| a.ip().to_string())
}

#[cfg(target_os = "linux")]
fn current_ssid() -> Option<String> {
    // nmcli ships with NetworkManager (Ubuntu/GNOME default). `-t` gives a
    // terse "active:ssid" per known Wi-Fi; the connected one has active=yes.
    let out = std::process::Command::new("nmcli")
        .args(["-t", "-f", "active,ssid", "dev", "wifi"])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("yes:").map(str::trim))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

#[cfg(target_os = "macos")]
fn current_ssid() -> Option<String> {
    // The Wi-Fi interface is usually en0 (en1 on some Macs). The tool prints
    // "Current Wi-Fi Network: <ssid>" or "You are not associated...".
    ["en0", "en1"].iter().find_map(|dev| {
        let out = std::process::Command::new("networksetup")
            .args(["-getairportnetwork", dev])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        if text.contains("not associated") {
            return None;
        }
        text.split_once(": ")
            .map(|(_, ssid)| ssid.trim().to_string())
            .filter(|s| !s.is_empty())
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn current_ssid() -> Option<String> {
    None
}

// ---- Access control (trusted clients) --------------------------------------

/// Verify the machine owner's OS login password (Linux: PAM, service
/// `rivetlink`). Gates changes to who may remotely access this device — only
/// someone who can log in to the machine may edit the allow-list. The password
/// is handed straight to PAM and never logged or stored. Other platforms have
/// no host-side trust editing yet, so this returns false there.
#[cfg(target_os = "linux")]
fn os_password_ok(password: &str) -> bool {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_default();
    if user.is_empty() {
        return false;
    }
    match pam::Client::with_password("rivetlink") {
        Ok(mut client) => {
            client.conversation_mut().set_credentials(user, password);
            client.authenticate().is_ok()
        },
        Err(_) => false,
    }
}

#[cfg(not(target_os = "linux"))]
fn os_password_ok(_password: &str) -> bool {
    false
}

/// Add a trusted client (its identity key + a name) after the owner confirms
/// with their OS login password. Trusted clients may later connect without the
/// session code. Returns the updated settings.
#[tauri::command]
async fn add_trusted_key(
    state: State<'_, AppState>,
    name: String,
    public_key: String,
    os_password: String,
) -> Result<AppSettings, String> {
    if !os_password_ok(&os_password) {
        return Err("wrong-os-password".to_string());
    }
    let key = public_key.trim().to_string();
    if key.is_empty() {
        return Err("empty-key".to_string());
    }
    let mut settings = state.settings.lock().await;
    if settings.trusted_keys.iter().any(|k| k.public_key == key) {
        return Err("duplicate-key".to_string());
    }
    settings.trusted_keys.push(TrustedKey {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.trim().to_string(),
        public_key: key,
    });
    settings.save(&state.data_dir)?;
    sync_trusted_live(&state, &settings);
    Ok(settings.clone())
}

/// Mirror the persisted trusted keys into the live allow-list the running host
/// reads, so an add/remove applies without restarting the host.
fn sync_trusted_live(state: &AppState, settings: &AppSettings) {
    if let Ok(mut live) = state.trusted_keys.lock() {
        *live = settings.trusted_keys.iter().map(|k| k.public_key.clone()).collect();
    }
}

/// Remove a trusted client, after the owner confirms with their OS password.
#[tauri::command]
async fn remove_trusted_key(
    state: State<'_, AppState>,
    id: String,
    os_password: String,
) -> Result<AppSettings, String> {
    if !os_password_ok(&os_password) {
        return Err("wrong-os-password".to_string());
    }
    let mut settings = state.settings.lock().await;
    settings.trusted_keys.retain(|k| k.id != id);
    settings.save(&state.data_dir)?;
    sync_trusted_live(&state, &settings);
    Ok(settings.clone())
}

/// Install a `tracing` subscriber that writes to **stderr** (visible when the
/// app is launched from a terminal) and a **log file** in the app's log dir, so
/// a user can attach it to a bug report. The SDK + agent log the whole LAN
/// connect / handshake / capture path; the default filter keeps the SDK and app
/// at `debug` (handshake detail) without the per-frame agent spam. `RUST_LOG`
/// overrides it. Returns the log file path on success.
fn init_logging(log_dir: &std::path::Path) -> Option<PathBuf> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let _ = std::fs::create_dir_all(log_dir);
    let log_path = log_dir.join("rivetlink.log");
    // Truncate per launch so an attached log is just the current session, small.
    let _ = std::fs::remove_file(&log_path);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .ok()?;

    let (writer, guard) = tracing_appender::non_blocking(file);
    // Keep the appender's flush guard alive for the whole process.
    Box::leak(Box::new(guard));

    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new("info,rivetlink_sdk=debug,rivetlink_app_lib=debug")
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_ansi(false).with_writer(writer))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .try_init()
        .ok()?;

    Some(log_path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            // Logging first, so startup + the LAN path are captured from the off.
            if let Ok(log_dir) = app.path().app_log_dir() {
                if let Some(path) = init_logging(&log_dir) {
                    tracing::info!(
                        version = env!("CARGO_PKG_VERSION"),
                        log = %path.display(),
                        "RivetLink starting — logging to file + stderr"
                    );
                }
            }

            // Resolve the data dir, load settings, and seed the shared state.
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let settings = AppSettings::load(&data_dir);
            let trusted: Vec<String> =
                settings.trusted_keys.iter().map(|k| k.public_key.clone()).collect();
            app.manage(AppState {
                data_dir,
                settings: Mutex::new(settings),
                client: Arc::new(Mutex::new(None)),
                stream: std::sync::Mutex::new(None),
                switch_tx: std::sync::Mutex::new(None),
                host: std::sync::Mutex::new(None),
                trusted_keys: Arc::new(std::sync::Mutex::new(trusted)),
                sys: std::sync::Mutex::new(sysinfo::System::new()),
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
            lan_switch_display,
            lan_disconnect,
            resource_usage,
            start_host,
            stop_host,
            host_active,
            network_info,
            lan_ping,
            add_trusted_key,
            remove_trusted_key
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
