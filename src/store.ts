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

export type AppSettings = {
	setup_complete: boolean;
	device_name: string;
	roles: string[];
	relays: Relay[];
	active_relay_id: string | null;
};

export type Device = {
	id: string;
	hostname: string | null;
	platform: string | null;
	last_seen: string | null;
};

const emptySettings: AppSettings = {
	setup_complete: false,
	device_name: "",
	roles: [],
	relays: [],
	active_relay_id: null,
};

export const store = reactive({
	loaded: false,
	settings: { ...emptySettings } as AppSettings,
	connected: false,
	loggedIn: false,
	publicKey: "",
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
