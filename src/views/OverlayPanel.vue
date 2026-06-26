<template>
	<div class="badge" :class="{ collapsed }">
		<!-- Collapsed: expand chevron on the LEFT, live dot tucked to the right
		     (against the screen edge). Explicit element order — no row-reverse. -->
		<template v-if="collapsed">
			<button
				type="button"
				class="handle"
				:title="t('overlay.expand')"
				@click="toggle"
			>
				<i class="mdi mdi-chevron-left" />
			</button>
			<span class="dot" />
		</template>

		<!-- "Are you sure?" before kicking. -->
		<template v-else-if="confirming">
			<span class="dot" />
			<span class="label">{{ t("overlay.kickConfirm") }}</span>
			<button type="button" class="btn danger" @click="doKick">
				{{ t("overlay.kick") }}
			</button>
			<button type="button" class="btn" @click="confirming = false">
				{{ t("common.cancel") }}
			</button>
		</template>

		<!-- Normal: who's watching + grant control + kick + collapse. -->
		<template v-else>
			<span class="dot" />
			<span class="label">{{ t("overlay.watching", { name: peer || t("overlay.someone") }) }}</span>
			<button
				type="button"
				class="btn icon"
				:class="{ active: controlGranted }"
				:title="controlGranted ? t('overlay.controlOn') : t('overlay.controlOff')"
				@click="toggleControl"
			>
				<i class="mdi" :class="controlGranted ? 'mdi-mouse' : 'mdi-mouse-off'" />
			</button>
			<button
				type="button"
				class="btn danger icon"
				:title="t('overlay.kick')"
				@click="confirming = true"
			>
				<i class="mdi mdi-account-cancel" />
			</button>
			<button
				type="button"
				class="handle"
				:title="t('overlay.collapse')"
				@click="toggle"
			>
				<i class="mdi mdi-chevron-right" />
			</button>
		</template>
	</div>
</template>

<script setup lang="ts">
	import {
		nextTick, onMounted, onUnmounted, ref,
	} from "vue";
	import { useI18n } from "vue-i18n";
	import { invoke } from "@tauri-apps/api/core";
	import {
		listen, type UnlistenFn,
	} from "@tauri-apps/api/event";

	type HostState = {
		pin: string | null;
		peer: string | null;
		control: boolean;
	};

	const { t } = useI18n();
	const peer = ref<string | null>(null);
	const collapsed = ref(false);
	const confirming = ref(false);
	// Whether the helper is allowed to drive this device's mouse/keyboard. Off by
	// default — being viewed never implies being controlled.
	const controlGranted = ref(false);

	let unlistenConnected: UnlistenFn | null = null;
	let unlistenDisconnected: UnlistenFn | null = null;
	let unlistenControl: UnlistenFn | null = null;

	// Collapse is pure CSS now: the window stays at a fixed size/position (runtime
	// resize+reposition desync on GNOME/Wayland and flung the badge off-screen).
	// The pill hugs the window's right edge, so collapsing just swaps the content
	// to the "< dot" peek tab — the rest of the fixed window is transparent.
	async function toggle(): Promise<void> {
		collapsed.value = !collapsed.value;
		confirming.value = false;
		// The webview runs with accelerated compositing disabled (the backend sets
		// WEBKIT_DISABLE_COMPOSITING_MODE), so the software painter erases the shrunk
		// pill's vacated region on its own. Belt-and-suspenders: a body display
		// toggle forces a full repaint in case anything is still cached.
		await nextTick();
		const b = document.body;
		b.style.display = "none";
		void b.offsetHeight;
		b.style.display = "";
	}

	// Kick the helper. The backend drops the viewer and closes this window, so
	// there's nothing to clean up here.
	async function doKick(): Promise<void> {
		await invoke("host_disconnect").catch(() => { /* already gone */ });
	}

	// Grant or revoke the helper's mouse/keyboard control. Optimistic; the backend
	// echoes "host://control" to confirm and keep every host surface in sync.
	async function toggleControl(): Promise<void> {
		const next = !controlGranted.value;
		controlGranted.value = next;
		await invoke("host_set_control", { value: next }).catch(() => {
			controlGranted.value = !next; // revert on failure
		});
	}

	onMounted(async () => {
		const app = document.getElementById("app");
		for (const el of [document.documentElement, document.body, app]) {
			if (el) {
				el.style.background = "transparent";
				el.style.margin = "0";
				el.style.overflow = "hidden";
				el.style.width = "100%";
				el.style.height = "100%";
			}
		}
		// Right-hug the pill: the dark badge is only ever as wide as its content
		// (chevron+dot when collapsed), and any leftover window width stays
		// transparent on the LEFT — no dead dark space. Robust to the window not
		// shrinking on collapse (GNOME/Wayland sometimes ignores set_size).
		if (app) {
			app.style.display = "flex";
			app.style.justifyContent = "flex-end";
			app.style.alignItems = "center";
		}
		// Position the (fixed-size) window bottom-right. GNOME/Wayland ignores the
		// window builder's initial position, so do it here at runtime.
		invoke("place_badge").catch(() => { /* window gone */ });
		// The connect event can fire before this window mounts, so pull the live
		// peer up front, then track changes.
		try {
			const state = await invoke<HostState>("host_active");
			peer.value = state.peer;
			controlGranted.value = state.control;
		} catch {
			// Backend unavailable — the badge still shows the generic label.
		}
		unlistenConnected = await listen<string>("host://connected", (e) => {
			peer.value = e.payload;
			// The window is reused across sessions, so start each one expanded.
			collapsed.value = false;
			confirming.value = false;
		});
		unlistenDisconnected = await listen("host://disconnected", () => {
			peer.value = null;
		});
		unlistenControl = await listen<boolean>("host://control", (e) => {
			controlGranted.value = e.payload;
		});
	});

	onUnmounted(() => {
		unlistenConnected?.();
		unlistenDisconnected?.();
		unlistenControl?.();
	});
</script>

<style scoped>
	.badge {
		display: flex;
		flex: none; /* size to content, hug the right edge — never fill the window */
		align-items: center;
		gap: 10px;
		height: 64px;
		padding: 0 10px 0 16px;
		box-sizing: border-box;
		border-radius: 12px;
		background: rgba(20, 20, 22, 0.94);
		color: #fff;
		font-family: system-ui, sans-serif;
		font-size: 0.95rem;
		white-space: nowrap;
		overflow: hidden;
		user-select: none;
	}

	/* Collapsed: just the expand chevron (left) + live dot (right), in a tight
	   content-width pill. Explicit element order in the template — no row-reverse. */
	.badge.collapsed {
		gap: 8px;
		padding: 0 14px;
	}

	.dot {
		flex: none;
		width: 12px;
		height: 12px;
		border-radius: 50%;
		background: #ff1744;
		box-shadow: 0 0 7px #ff1744;
		animation: pulse 1.6s ease-in-out infinite;
	}

	@keyframes pulse {
		50% {
			opacity: 0.35;
		}
	}

	.label {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.btn {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: center;
		height: 34px;
		padding: 0 12px;
		color: #fff;
		font-size: 0.9rem;
		background: rgba(255, 255, 255, 0.1);
		border: none;
		border-radius: 8px;
		cursor: pointer;
	}

	.btn.icon {
		width: 38px;
		padding: 0;
	}

	/* Control granted — the mouse toggle goes green. */
	.btn.active {
		color: #69f0ae;
		background: rgba(105, 240, 174, 0.18);
	}

	.btn:hover {
		background: rgba(255, 255, 255, 0.2);
	}

	.btn.danger {
		color: #ff6b81;
		background: rgba(255, 23, 68, 0.16);
	}

	.btn.danger:hover {
		background: rgba(255, 23, 68, 0.32);
	}

	/* The collapse/expand handle — the chevron. */
	.handle {
		flex: none;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 34px;
		height: 34px;
		color: #fff;
		background: rgba(255, 255, 255, 0.08);
		border: none;
		border-radius: 8px;
		cursor: pointer;
	}

	.handle:hover {
		background: rgba(255, 255, 255, 0.18);
	}

	.handle .mdi,
	.btn .mdi {
		font-size: 1.3rem;
	}
</style>
