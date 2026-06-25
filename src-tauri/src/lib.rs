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
use rivetlink_agent::lan::{serve_with_events, ConsentRequest, HostEvent, LanAuth};
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
    /// In-flight host-consent prompts, keyed by a per-request id. The agent
    /// awaits the host's accept/reject on each entry's `reply`; `respond_consent`
    /// looks it up by id and answers.
    pending_consents: std::sync::Mutex<std::collections::HashMap<u64, ConsentPending>>,
    /// Monotonic id source for consent prompts.
    consent_seq: std::sync::atomic::AtomicU64,
}

/// A pending host-consent prompt awaiting the user's accept/reject.
struct ConsentPending {
    reply: tokio::sync::oneshot::Sender<bool>,
    /// The client's verified identity key (base64), if any — to remember it.
    key: Option<String>,
    name: String,
}

/// A running host session: the accept-loop task and the task forwarding its
/// lifecycle events to the frontend. Aborting both stops hosting and (by
/// dropping the listener) unregisters the mDNS advertisement.
struct HostSession {
    serve: tokio::task::JoinHandle<()>,
    forward: tokio::task::JoinHandle<()>,
    /// Task draining host-consent requests from the agent to the UI.
    consent_fwd: tokio::task::JoinHandle<()>,
    /// The PIN the helper must enter; shown on the host's screen.
    pin: String,
    /// Label of the currently connected client (`None` = nobody). Kept in sync
    /// by the event-forwarding task so the UI can restore the right "connected /
    /// waiting" state after navigating away and back, when it missed the live
    /// `host://connected|disconnected` events.
    peer: Arc<std::sync::Mutex<Option<String>>>,
    /// Bumping this hangs up on the *active* viewer without stopping the
    /// listener (so the PIN/advertisement stay live). Drives "Disconnect helper".
    kick: tokio::sync::watch::Sender<u64>,
    /// "Share all screens": `true` lets the helper list/switch every display,
    /// `false` pins it to the primary. Toggled live from the host UI; the agent
    /// reads it per request and pushes the client a fresh display list on change.
    share_all: tokio::sync::watch::Sender<bool>,
    /// The connected client's verified identity `(key_b64, label)` when it
    /// announced one — what `trust_client` remembers (trust-on-connect). `None`
    /// for nobody, or a client that connected without proving an identity.
    current_client: Arc<std::sync::Mutex<Option<(String, String)>>>,
}

/// What the host UI needs to offer "remember this device": the connected
/// client's identity key + name, and whether it's already trusted.
#[derive(Serialize, Clone)]
struct ClientIdentityDto {
    key: Option<String>,
    name: Option<String>,
    trusted: bool,
}

impl HostSession {
    fn stop(self) {
        self.serve.abort();
        self.forward.abort();
        self.consent_fwd.abort();
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

/// Register the running AppImage in the desktop menu so it shows up in GNOME /
/// app-search. A bare AppImage is just an executable file — unlike the .deb/.rpm
/// install it ships no system `.desktop` entry, so nothing indexes it. We write a
/// user-level entry (`~/.local/share/applications`) + copy the icon out of the
/// (ephemeral) AppImage mount. Best effort; re-run each launch so `Exec` tracks
/// the AppImage if the user moves it. No-op when not running from an AppImage.
#[cfg(target_os = "linux")]
fn integrate_appimage_desktop() {
    use std::path::PathBuf;

    let (Some(appimage), Some(appdir), Some(home)) = (
        std::env::var_os("APPIMAGE"),
        std::env::var_os("APPDIR"),
        std::env::var_os("HOME"),
    ) else {
        return; // not an AppImage run (or no mount) — nothing to integrate
    };
    let appimage = PathBuf::from(appimage);
    let data = PathBuf::from(home).join(".local/share");

    // Copy the icon out of the temporary AppImage mount to a stable path so the
    // entry keeps an icon after the app exits (the mount disappears).
    let icon = "rivetlink-app";
    for size in ["256x256@2", "128x128", "32x32"] {
        let src = PathBuf::from(&appdir)
            .join(format!("usr/share/icons/hicolor/{size}/apps/{icon}.png"));
        let dst_dir = data.join(format!("icons/hicolor/{size}/apps"));
        if src.exists() && std::fs::create_dir_all(&dst_dir).is_ok() {
            let _ = std::fs::copy(&src, dst_dir.join(format!("{icon}.png")));
        }
    }

    let apps_dir = data.join("applications");
    if std::fs::create_dir_all(&apps_dir).is_err() {
        return;
    }
    // Quote the Exec path so a directory with spaces still launches.
    let entry = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=RivetLink\n\
         Comment=Zero-trust remote control\n\
         Exec=\"{}\" %U\n\
         Icon={icon}\n\
         Terminal=false\n\
         Categories=Network;RemoteAccess;Utility;\n\
         StartupWMClass=rivetlink-app\n",
        appimage.display(),
    );
    let path = apps_dir.join("rivetlink-app.desktop");
    if std::fs::write(&path, entry).is_ok() {
        tracing::info!(path = %path.display(), "registered AppImage in the app menu");
    }
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

// ---- Host "being viewed" overlay -------------------------------------------

/// The host's primary screen as a *logical* rect (x, y, width, height) — what
/// Tauri's window builder expects. `None` if no monitor is reported (headless).
fn primary_logical_rect(app: &tauri::AppHandle) -> Option<(f64, f64, f64, f64)> {
    let monitor = app.primary_monitor().ok().flatten()?;
    let scale = monitor.scale_factor();
    let pos = monitor.position().to_logical::<f64>(scale);
    let size = monitor.size().to_logical::<f64>(scale);
    Some((pos.x, pos.y, size.width, size.height))
}

/// Show the "you're being viewed" badge: a small, collapsible, transparent,
/// always-on-top window bottom-right of the host's primary screen, shown only
/// while a helper is actually viewing. Idempotent.
///
/// On Wayland the compositor owns window placement, so the badge lands on the
/// current/primary output and can't be repositioned — by design (the user opted
/// for "panel stays on screen 1 always").
fn show_host_overlay(app: &tauri::AppHandle) {
    let rect = primary_logical_rect(app);

    if app.get_webview_window("hostpanel").is_none() {
        let mut builder = tauri::WebviewWindowBuilder::new(
            app,
            "hostpanel",
            tauri::WebviewUrl::App("index.html#/overlay-panel".into()),
        )
        .title("RivetLink")
        .transparent(true)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .resizable(false)
        .inner_size(BADGE_EXPANDED_W, BADGE_H)
        .focused(false);
        if let Some((x, y, w, h)) = rect {
            let (px, py) = badge_origin(x, y, w, h, BADGE_EXPANDED_W);
            builder = builder.position(px, py);
        }
        match builder.build() {
            // Drop the inherited app menu bar ("RivetLink"/"Edit") — on Linux it
            // renders inside the window and would clip the badge row.
            Ok(win) => {
                let _ = win.remove_menu();
            },
            Err(e) => tracing::warn!(error = %e, "overlay: panel window failed"),
        }
    }
}

/// Badge window geometry. Fixed at the bottom-right of the primary screen,
/// raised 10% of the screen height so it clears the dock/taskbar. The window is
/// never resized/repositioned at runtime — collapse is pure CSS and the pill
/// hugs the window's right edge — because GNOME/Wayland desyncs runtime
/// set_size/set_position and flung the badge off-screen.
const BADGE_EXPANDED_W: f64 = 340.0;
const BADGE_H: f64 = 64.0;
const BADGE_MARGIN: f64 = 16.0;

/// Top-left for a badge of `width` on the primary screen: bottom-right corner,
/// lifted 10% of the screen height clear of the dock.
fn badge_origin(mx: f64, my: f64, mw: f64, mh: f64, width: f64) -> (f64, f64) {
    (mx + mw - width - BADGE_MARGIN, my + mh - BADGE_H - mh * 0.10)
}

/// Tear down the host "being viewed" badge (the viewing session ended).
fn hide_host_overlay(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("hostpanel") {
        let _ = win.close();
    }
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

    // Ask the host which screens it can share and hand it to the viewer for a
    // screen picker. Bounded: an unresponsive host shouldn't stall the stream.
    let displays = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        rivetlink_sdk::lan::list_displays(&mut stream, &channel),
    )
    .await
    {
        Ok(Ok(d)) => d,
        _ => Vec::new(),
    };
    // First (optimistic) emit; the viewer window may not have registered its
    // listener yet, so the stream task re-emits on the first frame too.
    let _ = app.emit("lan://displays", &displays);

    // The viewer's screen picker pushes display ids here to switch mid-stream.
    let (switch_tx, switch_rx) = tokio::sync::mpsc::channel::<u32>(4);
    if let Ok(mut guard) = state.switch_tx.lock() {
        *guard = Some(switch_tx);
    }

    // Our own device name travels with the stream so the host can show *who* is
    // viewing instead of a bare IP.
    let my_name = {
        let s = state.settings.lock().await;
        let n = s.device_name.trim();
        if n.is_empty() { None } else { Some(n.to_string()) }
    };
    // Announce our identity (signed) with the stream so a host we reached by PIN
    // can offer to remember this device (trust-on-connect). Same identity the
    // key-mode connect uses, so a remembered device later connects code-free.
    let identity = Identity::load_or_create(&state.identity_path()).map_err(|e| e.to_string())?;

    let app_for_task = app.clone();
    let app_for_displays = app.clone();
    let displays_for_viewer = displays.clone();
    let task = tokio::spawn(async move {
        let mut announced = false;
        let result = rivetlink_sdk::lan::stream_frames(
            stream,
            channel,
            // 30 fps: a 33 ms frame interval (vs 50 ms at 20) directly trims the
            // capture-cadence term that dominates felt LAN latency, and is
            // smoother. Tile-delta means a static screen stays cheap regardless.
            30,
            None,
            my_name,
            Some(&identity),
            switch_rx,
            |delta| {
                // The first frame proves the viewer window is mounted + listening,
                // so re-send the display list now — the optimistic emit above can
                // beat the viewer's listener registration, leaving the picker empty.
                if !announced {
                    announced = true;
                    let _ = app_for_task.emit("lan://displays", &displays_for_viewer);
                }
                // Forward the delta frame to the viewer window. If emit fails the
                // window is gone — stop the stream.
                app_for_task.emit("lan://frame", delta).is_ok()
            },
            // The host pushes a fresh display list when it grants/revokes "share
            // all screens" mid-stream — relay it so the viewer's picker updates.
            move |displays| {
                let _ = app_for_displays.emit("lan://displays", displays);
            },
        )
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
    // Kick channel: bumping the value drops the active viewer (host pressed
    // "Disconnect") while keeping the listener up.
    let (kick_tx, kick_rx) = tokio::sync::watch::channel(0u64);
    // Share-all channel: a direct LAN helper sees every screen by default; the
    // host can flip this off to restrict it to the primary screen. (A future
    // internet/session-code transport would start this at `false`.)
    let (share_tx, share_rx) = tokio::sync::watch::channel(true);
    // Consent channel: the agent asks before letting a not-yet-trusted (PIN)
    // client view; the drain task below turns each request into a UI prompt.
    let (consent_tx, mut consent_rx) = tokio::sync::mpsc::channel::<ConsentRequest>(8);

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
        if let Err(e) = serve_with_events(
            signing_key,
            device_name,
            port,
            auth,
            Some(tx),
            Some(kick_rx),
            Some(share_rx),
            Some(consent_tx),
        )
        .await
        {
            tracing::warn!(error = %e, "start_host: serve loop ended with error");
            let _ = app_for_serve.emit("host://error", e.to_string());
        }
    });

    // Turn each agent consent request into a UI prompt: store the reply keyed by
    // an id, surface the main window, and emit the request to the frontend.
    let app_for_consent = app.clone();
    let consent_fwd = tokio::spawn(async move {
        while let Some(ConsentRequest { key, name, reply }) = consent_rx.recv().await {
            let st = app_for_consent.state::<AppState>();
            let id = st.consent_seq.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if let Ok(mut map) = st.pending_consents.lock() {
                map.insert(id, ConsentPending { reply, key: key.clone(), name: name.clone() });
            }
            // Bring the host's window forward so the prompt isn't missed.
            if let Some(win) = app_for_consent.get_webview_window("main") {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
            let _ = app_for_consent.emit("host://consent-request", ConsentRequestDto { id, key, name });
        }
    });

    // Shared truth for "who is connected", read by `host_active` so the UI can
    // resync after missing a live event.
    let peer = Arc::new(std::sync::Mutex::new(None::<String>));
    let current_client = Arc::new(std::sync::Mutex::new(None::<(String, String)>));

    let app_for_fwd = app.clone();
    let peer_for_fwd = Arc::clone(&peer);
    let client_for_fwd = Arc::clone(&current_client);
    let trusted_for_fwd = Arc::clone(&state.trusted_keys);
    let forward = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                HostEvent::ClientConnected { label, key } => {
                    if let Ok(mut p) = peer_for_fwd.lock() {
                        *p = Some(label.clone());
                    }
                    // Remember the verified identity so the host can offer to
                    // trust this device (trust-on-connect).
                    if let Ok(mut c) = client_for_fwd.lock() {
                        *c = key.as_ref().map(|k| (k.clone(), label.clone()));
                    }
                    // Raise the on-screen "you're being viewed" badge.
                    show_host_overlay(&app_for_fwd);
                    let _ = app_for_fwd.emit("host://connected", &label);
                    // Tell the UI whether this device can be remembered (it isn't
                    // already trusted, and it proved an identity).
                    let trusted = key
                        .as_ref()
                        .is_some_and(|k| trusted_for_fwd.lock().is_ok_and(|t| t.iter().any(|x| x == k)));
                    let _ = app_for_fwd.emit("host://client-identity", ClientIdentityDto {
                        key,
                        name: Some(label),
                        trusted,
                    });
                },
                HostEvent::ClientDisconnected => {
                    if let Ok(mut p) = peer_for_fwd.lock() {
                        *p = None;
                    }
                    if let Ok(mut c) = client_for_fwd.lock() {
                        *c = None;
                    }
                    hide_host_overlay(&app_for_fwd);
                    let _ = app_for_fwd.emit("host://disconnected", ());
                },
            }
        }
    });

    if let Ok(mut guard) = state.host.lock() {
        *guard = Some(HostSession {
            serve,
            forward,
            consent_fwd,
            pin: pin.clone(),
            peer,
            kick: kick_tx,
            share_all: share_tx,
            current_client,
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
    hide_host_overlay(&app);
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
    /// Whether the helper may see/switch every screen (vs. the primary only).
    share_all: bool,
    /// The connected client's verified identity key (base64), if it proved one —
    /// for restoring the "remember this device" prompt after a UI resync.
    client_key: Option<String>,
    /// Whether that client is already in the trusted list.
    client_trusted: bool,
}

#[tauri::command]
async fn host_active(state: State<'_, AppState>) -> Result<HostState, String> {
    let guard = state.host.lock().map_err(|_| "host lock poisoned".to_string())?;
    let (pin, peer, share_all, client_key) = match guard.as_ref() {
        Some(session) => (
            Some(session.pin.clone()),
            session.peer.lock().ok().and_then(|p| p.clone()),
            *session.share_all.borrow(),
            session.current_client.lock().ok().and_then(|c| c.as_ref().map(|(k, _)| k.clone())),
        ),
        None => (None, None, true, None),
    };
    let client_trusted = client_key.as_ref().is_some_and(|k| {
        state.trusted_keys.lock().is_ok_and(|t| t.iter().any(|x| x == k))
    });
    Ok(HostState { pin, peer, share_all, client_key, client_trusted })
}

/// Toggle "share all screens" on the live host session. The agent picks it up
/// per request and pushes the connected helper a fresh display list; on revoke
/// it snaps the helper back to the primary screen. No-op if not hosting.
#[tauri::command]
async fn host_set_share_all(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    value: bool,
) -> Result<(), String> {
    if let Ok(guard) = state.host.lock() {
        if let Some(session) = guard.as_ref() {
            let _ = session.share_all.send(value);
        }
    }
    // Mirror to any open host UI (the page + the overlay badge).
    let _ = app.emit("host://share-all", value);
    Ok(())
}

/// Current "share all screens" state (defaults to `true` when not hosting).
#[tauri::command]
async fn host_share_all(state: State<'_, AppState>) -> Result<bool, String> {
    let guard = state.host.lock().map_err(|_| "host lock poisoned".to_string())?;
    Ok(guard.as_ref().map_or(true, |s| *s.share_all.borrow()))
}

/// Hang up on the currently connected helper without stopping hosting: the
/// listener and PIN stay live so a new helper can connect. No-op if nobody is
/// connected. The host's "connected" badge clears immediately; the agent drops
/// the viewer's stream (it sees a disconnect).
#[tauri::command]
async fn host_disconnect(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if let Ok(guard) = state.host.lock() {
        if let Some(session) = guard.as_ref() {
            // Bump the watched value to wake every session's kick arm.
            session.kick.send_modify(|v| *v = v.wrapping_add(1));
            if let Ok(mut p) = session.peer.lock() {
                *p = None;
            }
            if let Ok(mut c) = session.current_client.lock() {
                *c = None;
            }
        }
    }
    hide_host_overlay(&app);
    let _ = app.emit("host://disconnected", ());
    Ok(())
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
    let mut settings = state.settings.lock().await;
    push_trusted_key(&mut settings, &name, &public_key)?;
    settings.save(&state.data_dir)?;
    sync_trusted_live(&state, &settings);
    Ok(settings.clone())
}

/// Add a trusted key to `settings` (validate non-empty, dedupe by key). Shared
/// by the manual add (OS-password gated) and trust-on-connect (gated by the live
/// PIN connection instead). Does NOT persist — the caller saves + syncs.
fn push_trusted_key(settings: &mut AppSettings, name: &str, public_key: &str) -> Result<(), String> {
    let key = public_key.trim().to_string();
    if key.is_empty() {
        return Err("empty-key".to_string());
    }
    if settings.trusted_keys.iter().any(|k| k.public_key == key) {
        return Err("duplicate-key".to_string());
    }
    settings.trusted_keys.push(TrustedKey {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.trim().to_string(),
        public_key: key,
    });
    Ok(())
}

/// Trust the *currently connected* client (trust-on-connect): remember its
/// verified identity key so it can reconnect without the session code. No OS
/// password — the correct PIN plus the host actively clicking "remember" on a
/// live connection is the gate. No-op error if nobody is connected (with a key).
#[tauri::command]
async fn trust_client(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> Result<AppSettings, String> {
    let key = state
        .host
        .lock()
        .ok()
        .and_then(|g| g.as_ref().and_then(|s| s.current_client.lock().ok().and_then(|c| c.clone())))
        .map(|(k, _label)| k)
        .ok_or_else(|| "no connected client to trust".to_string())?;

    let mut settings = state.settings.lock().await;
    // Already trusted (e.g. double-click) is success, not an error.
    if !settings.trusted_keys.iter().any(|k| k.public_key == key) {
        push_trusted_key(&mut settings, &name, &key)?;
        settings.save(&state.data_dir)?;
        sync_trusted_live(&state, &settings);
    }
    let result = settings.clone();
    drop(settings);
    // Reflect the new trusted state in the host UI.
    let _ = app.emit("host://client-identity", ClientIdentityDto {
        key: Some(key),
        name: Some(name),
        trusted: true,
    });
    Ok(result)
}

/// A pending consent prompt sent to the host UI.
#[derive(Serialize, Clone)]
struct ConsentRequestDto {
    id: u64,
    key: Option<String>,
    name: String,
}

/// The host answered a consent prompt. Forwards the decision to the waiting
/// agent session; on accept + remember, trusts the client's key (no OS password
/// — the correct PIN plus this explicit accept is the gate). No-op for an
/// unknown id (e.g. the request already timed out).
#[tauri::command]
async fn respond_consent(
    state: State<'_, AppState>,
    id: u64,
    accept: bool,
    remember: bool,
) -> Result<(), String> {
    let pending = state.pending_consents.lock().ok().and_then(|mut m| m.remove(&id));
    let Some(ConsentPending { reply, key, name }) = pending else {
        return Ok(());
    };
    if accept && remember {
        if let Some(key) = key {
            let mut settings = state.settings.lock().await;
            if !settings.trusted_keys.iter().any(|k| k.public_key == key) {
                // Best-effort: ignore empty/dup since the host explicitly accepted.
                let _ = push_trusted_key(&mut settings, &name, &key);
                settings.save(&state.data_dir)?;
                sync_trusted_live(&state, &settings);
            }
        }
    }
    let _ = reply.send(accept);
    Ok(())
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

            // Make an AppImage install discoverable in the app menu / search
            // (the .deb/.rpm already register a system .desktop entry).
            #[cfg(target_os = "linux")]
            integrate_appimage_desktop();

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
                pending_consents: std::sync::Mutex::new(std::collections::HashMap::new()),
                consent_seq: std::sync::atomic::AtomicU64::new(0),
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

            // Dev convenience: in a debug build (`npm run tauri dev`) pop the host
            // "being viewed" badge on launch so its layout can be iterated without
            // a real incoming connection. Release builds only show it while a
            // helper is actually viewing.
            #[cfg(debug_assertions)]
            show_host_overlay(app.handle());

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
            host_disconnect,
            host_set_share_all,
            host_share_all,
            trust_client,
            respond_consent,
            network_info,
            lan_ping,
            add_trusted_key,
            remove_trusted_key
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
