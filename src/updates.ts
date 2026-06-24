// "Check for Updates" logic.
//
// macOS + Windows + Linux AppImage: the Tauri updater plugin checks a signed
// manifest, then downloads and installs in place (then relaunches).
// Linux .deb/.rpm: the updater can't replace a root-owned system package, so we
// only *notify* — fetch the latest GitHub release, compare versions, and offer
// a Download link.

import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
	check, type Update,
} from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

const REPO = "Rivetlink/RivetLink-Application";
const RELEASES_URL = `https://github.com/${REPO}/releases`;

// A release whose notes contain this marker is *mandatory*: the update dialog
// can't be dismissed until the user updates. Put `[force-update]` anywhere in
// the GitHub release body to flag a security/breaking release as required.
const FORCE_MARKER = "[force-update]";

function isForced(body?: string | null): boolean {
	return (body ?? "").toLowerCase().includes(FORCE_MARKER);
}

// Tauri's updater can't replace a system package; deb/rpm Linux falls back to
// notify. An AppImage install *can* self-update, detected at runtime (Rust).
const isLinux = navigator.userAgent.includes("Linux")
    && !navigator.userAgent.includes("Android");

// True only for a .deb/.rpm install (notify-only). Resolved on first check via
// the `is_appimage` command; AppImage and non-Linux can install in place.
let notifyOnly = isLinux;

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
	canAutoInstall: !notifyOnly,
	// When the available release is flagged mandatory, the dialog is persistent
	// (no close button, can't click away) — the user must update to continue.
	forced: false,
});

let pending: Update | null = null;

/** Semver-ish compare: 1 if a > b, -1 if a < b, 0 if equal. */
function compareVersions(a: string, b: string): number {
	const pa = a.split(".").map((n) => parseInt(n, 10) || 0);
	const pb = b.split(".").map((n) => parseInt(n, 10) || 0);
	const len = Math.max(pa.length, pb.length);
	for (let i = 0; i < len; i++) {
		const diff = (pa[i] || 0) - (pb[i] || 0);
		if (diff !== 0) {return diff > 0 ? 1 : -1;}
	}
	return 0;
}

async function checkDesktop(): Promise<void> {
	const update = await check();
	if (update) {
		pending = update;
		updateState.latest = update.version;
		updateState.forced = isForced(update.body);
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
	const isNewer = compareVersions(tag, updateState.current) > 0;
	updateState.forced = isNewer && isForced(data.body);
	updateState.status = isNewer ? UpdateStatus.Available : UpdateStatus.UpToDate;
}

async function runCheck(): Promise<void> {
	updateState.checking = true;
	updateState.status = UpdateStatus.Idle;
	updateState.latest = "";
	updateState.forced = false;
	pending = null;
	try {
		updateState.current = await invoke<string>("app_version");
		if (isLinux) {
			// A .deb/.rpm install is notify-only; an AppImage can self-update.
			notifyOnly = !(await invoke<boolean>("is_appimage"));
		}
		updateState.canAutoInstall = !notifyOnly;
		if (notifyOnly) {
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

/** Manual check (RivetLink menu): always opens the dialog so the user sees a
 * result, even "you're up to date". */
export async function checkForUpdates(): Promise<void> {
	updateState.dialog = true;
	await runCheck();
}

/** Silent check on launch: runs in the background and only surfaces the dialog
 * when an update is actually waiting. A forced release opens a dialog the user
 * can't dismiss; a normal one they can close. Stays quiet when up to date or on
 * a transient network error, so startup is never interrupted needlessly. */
export async function checkForUpdatesOnStartup(): Promise<void> {
	await runCheck();
	if (updateState.status === UpdateStatus.Available) {
		updateState.dialog = true;
	}
}

/** Install in place (desktop/AppImage) or open the download page (deb/rpm). */
export async function installUpdate(): Promise<void> {
	if (notifyOnly || !pending) {
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
