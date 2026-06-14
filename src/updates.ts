// "Check for updates" logic. Compares this build's version against the latest
// published GitHub release and reports the result in a dialog. (A full
// auto-updater that downloads + installs is a later milestone; this is the
// lightweight check.)

import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

const REPO = "Rivetlink/RivetLink-Application";
const RELEASES_URL = `https://github.com/${REPO}/releases`;

export type UpdateStatus = "idle" | "uptodate" | "available" | "error";

export const updateState = reactive({
    dialog: false,
    checking: false,
    current: "",
    latest: "",
    status: "idle" as UpdateStatus,
});

/** Semver-ish compare: returns 1 if a > b, -1 if a < b, 0 if equal. */
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

export async function checkForUpdates(): Promise<void> {
    updateState.dialog = true;
    updateState.checking = true;
    updateState.status = "idle";
    updateState.latest = "";
    try {
        updateState.current = await invoke<string>("app_version");
        const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
            headers: { Accept: "application/vnd.github+json" },
        });
        if (!res.ok) {
            throw new Error(`HTTP ${res.status}`);
        }
        const data = await res.json();
        const tag = String(data.tag_name ?? "").replace(/^v/, "");
        updateState.latest = tag;
        updateState.status = compareVersions(tag, updateState.current) > 0 ? "available" : "uptodate";
    } catch {
        updateState.status = "error";
    } finally {
        updateState.checking = false;
    }
}

export async function openReleasesPage(): Promise<void> {
    await openUrl(RELEASES_URL);
}
