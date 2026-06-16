// Small reactive store wrapping the Tauri backend commands. Keeps the
// persisted settings and the live session state in one place so every view
// reads from the same source of truth.

import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";

export type Relay = {
	id: string;
	name: string;
	http_url: string;
	ws_url: string;
};

export type SavedLanDevice = {
	id: string;
	name: string;
	address: string;
	port: number;
	public_key: string | null;
};

export type AppSettings = {
	setup_complete: boolean;
	device_name: string;
	roles: string[];
	relays: Relay[];
	active_relay_id: string | null;
	lan_devices: SavedLanDevice[];
};

/// A host found on the local network (not yet remembered).
export type LanDevice = {
	name: string;
	address: string;
	port: number;
	public_key: string | null;
	protocol_version: number | null;
};

export type Device = {
	id: string;
	hostname: string | null;
	platform: string | null;
	last_seen: string | null;
};

/// The machine's current network, for confirming both devices share a subnet.
export type NetworkInfo = {
	ssid: string | null;
	ip: string | null;
};

const emptySettings: AppSettings = {
	setup_complete: false,
	device_name: "",
	roles: [],
	relays: [],
	active_relay_id: null,
	lan_devices: [],
};

export const store = reactive({
	loaded: false,
	settings: { ...emptySettings } as AppSettings,
	connected: false,
	loggedIn: false,
	publicKey: "",
	/// Saved LAN device id of the active live session, or null.
	connectedLanId: null as string | null,
	/// Receive-help (host) state: whether we're advertising, the PIN to read
	/// out, and the label of the connected helper (null while waiting).
	hosting: false,
	hostPin: "",
	hostPeer: null as string | null,
});

export function isHost(): boolean {
	return store.settings.roles.includes("host");
}

export function isClient(): boolean {
	return store.settings.roles.includes("client");
}

export function activeRelay(): Relay | null {
	const id = store.settings.active_relay_id;
	return store.settings.relays.find((r) => r.id === id) ?? null;
}

export async function loadSettings(): Promise<void> {
	store.settings = await invoke<AppSettings>("get_settings");
	store.loaded = true;
}

export async function loadPublicKey(): Promise<void> {
	store.publicKey = await invoke<string>("public_key");
}

export async function completeSetup(deviceName: string, roles: string[]): Promise<void> {
	store.settings = await invoke<AppSettings>("complete_setup", {
		deviceName,
		roles,
	});
}

export async function updateDevice(deviceName: string, roles: string[]): Promise<void> {
	store.settings = await invoke<AppSettings>("update_device", {
		deviceName,
		roles,
	});
}

export async function addRelay(name: string, httpUrl: string): Promise<void> {
	// Only the HTTP URL is needed; the backend derives the WebSocket URL.
	store.settings = await invoke<AppSettings>("add_relay", {
		name,
		httpUrl,
	});
}

export async function removeRelay(id: string): Promise<void> {
	store.settings = await invoke<AppSettings>("remove_relay", { id });
	// Dropping the active relay invalidates the current session.
	store.connected = false;
	store.loggedIn = false;
}

export async function setActiveRelay(id: string): Promise<void> {
	store.settings = await invoke<AppSettings>("set_active_relay", { id });
	store.connected = false;
	store.loggedIn = false;
}

export async function connect(): Promise<void> {
	store.publicKey = await invoke<string>("connect");
	store.connected = true;
	store.loggedIn = false;
}

export async function login(email: string, password: string): Promise<void> {
	await invoke("login", {
		email,
		password,
	});
	store.loggedIn = true;
}

export async function listDevices(): Promise<Device[]> {
	return invoke<Device[]>("list_devices");
}

export async function captureScreenshot(deviceId: string): Promise<string> {
	return invoke<string>("capture_screenshot", { deviceId });
}

// ---- Direct-LAN ------------------------------------------------------------

export async function discoverLan(): Promise<LanDevice[]> {
	return invoke<LanDevice[]>("discover_lan");
}

export async function addLanDevice(
	name: string,
	address: string,
	port: number,
	publicKey: string | null,
): Promise<void> {
	store.settings = await invoke<AppSettings>("add_lan_device", {
		name,
		address,
		port,
		publicKey,
	});
}

export async function removeLanDevice(id: string): Promise<void> {
	store.settings = await invoke<AppSettings>("remove_lan_device", { id });
}

export async function lanScreenshot(
	address: string,
	port: number,
	pin: string | null,
	hostPublicKey: string | null,
): Promise<string> {
	return invoke<string>("lan_screenshot", {
		address,
		port,
		pin,
		hostPublicKey,
	});
}

/// Open a live screen stream to a saved LAN device (renders in its own window).
export async function lanConnect(
	device: SavedLanDevice,
	pin: string | null,
): Promise<void> {
	await invoke("lan_connect", {
		target: {
			name: device.name,
			address: device.address,
			port: device.port,
			deviceId: device.id,
			publicKey: device.public_key,
		},
		pin,
	});
}

/// Stop the active live stream and close the viewer window.
export async function lanDisconnect(): Promise<void> {
	await invoke("lan_disconnect");
}

// ---- Receive help (host mode) ----------------------------------------------

/// Start advertising + serving this screen; returns the PIN to read out to the
/// helper. The connected/disconnected state arrives via `host://` events.
export async function startHost(): Promise<string> {
	const pin = await invoke<string>("start_host");
	store.hosting = true;
	store.hostPin = pin;
	store.hostPeer = null;
	return pin;
}

/// Stop hosting (stop advertising + drop any session).
export async function stopHost(): Promise<void> {
	await invoke("stop_host");
	store.hosting = false;
	store.hostPin = "";
	store.hostPeer = null;
}

/// Re-sync host state from the backend (e.g. after navigating back).
export async function refreshHostState(): Promise<void> {
	const pin = await invoke<string | null>("host_active");
	store.hosting = pin !== null;
	store.hostPin = pin ?? "";
	if (!store.hosting) {
		store.hostPeer = null;
	}
}

/// This machine's current Wi-Fi name (if any) and LAN IP.
export async function networkInfo(): Promise<NetworkInfo> {
	return invoke<NetworkInfo>("network_info");
}

/// Whether a saved host is reachable right now (its listener accepts a TCP
/// connection). Used to show a live online/offline dot per device.
export async function lanPing(address: string, port: number): Promise<boolean> {
	return invoke<boolean>("lan_ping", {
		address,
		port,
	});
}
