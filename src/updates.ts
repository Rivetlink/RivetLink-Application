// "Check for Updates" logic.
//
// macOS + Windows: the Tauri updater plugin checks a signed manifest, then
// downloads and installs in place (then relaunches).
// Linux: Tauri's updater can't install a .deb/.rpm, so we only *notify* — fetch
// the latest GitHub release, compare versions, and offer a Download link.

import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
	check, type Update,
} from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

const REPO = "Rivetlink/RivetLink-Application";
const RELEASES_URL = `https://github.com/${REPO}/releases`;

// Tauri's updater can't replace a system package; Linux falls back to notify.
const isLinux = navigator.userAgent.includes("Linux")
    && !navigator.userAgent.includes("Android");

export enum UpdateStatus {
	Idle = "idle",
	UpToDate = "uptodate",
	Available = "available",
	Error = "error",
}

export const updateState = reactive({
	dialog: false,
	checking: false,
	installing: false,
	current: "",
	latest: "",
	status: UpdateStatus.Idle,
	canAutoInstall: !isLinux,
});

let pending: Update | null = null;

/** Semver-ish compare: 1 if a > b, -1 if a < b, 0 if equal. */
function compareVersions(a: string, b: string): number {
	const pa = a.split(".").map((n) => parseInt(n, 10) || 0);
	const pb = b.split(".").map((n) => parseInt(n, 10) || 0);
	const len = Math.max(pa.length, pb.length);
	for (let i = 0; i < len; i++) {
		const diff = (pa[i] || 0) - (pb[i] || 0);
		if (diff !== 0) return diff > 0 ? 1 : -1;
	}
	return 0;
}

async function checkDesktop(): Promise<void> {
	const update = await check();
	if (update) {
		pending = update;
		updateState.latest = update.version;
		updateState.status = UpdateStatus.Available;
	} else {
		updateState.status = UpdateStatus.UpToDate;
	}
}

async function checkLinux(): Promise<void> {
	const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
		headers: { Accept: "application/vnd.github+json" },
	});
	if (!res.ok) {
		throw new Error(`HTTP ${res.status}`);
	}
	const data = await res.json();
	const tag = String(data.tag_name ?? "").replace(/^v/, "");
	updateState.latest = tag;
	updateState.status = compareVersions(tag, updateState.current) > 0
		? UpdateStatus.Available
		: UpdateStatus.UpToDate;
}

export async function checkForUpdates(): Promise<void> {
	updateState.dialog = true;
	updateState.checking = true;
	updateState.status = UpdateStatus.Idle;
	updateState.latest = "";
	pending = null;
	try {
		updateState.current = await invoke<string>("app_version");
		if (isLinux) {
			await checkLinux();
		} else {
			await checkDesktop();
		}
	} catch {
		updateState.status = UpdateStatus.Error;
	} finally {
		updateState.checking = false;
	}
}

/** Install (desktop) or open the download page (Linux). */
export async function installUpdate(): Promise<void> {
	if (isLinux || !pending) {
		await openUrl(RELEASES_URL);
		return;
	}
	updateState.installing = true;
	try {
		await pending.downloadAndInstall();
		await relaunch();
	} catch {
		updateState.status = UpdateStatus.Error;
		updateState.installing = false;
	}
}
