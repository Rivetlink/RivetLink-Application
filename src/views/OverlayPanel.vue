<template>
	<div class="badge" :class="{ collapsed, blocked }">
		<!-- Shield feedback: flashes over the badge when an injected (client) click
		     on these buttons is swallowed, so the host can tell the shield worked. -->
		<div v-if="blocked" class="shield-flash">
			<i class="mdi mdi-shield-lock" />
			<span>{{ t("overlay.hostOnly") }}</span>
		</div>
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
				@click="startKick"
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
	// Briefly true right after the shield swallows an injected (client) click, so
	// the badge can flash "host only" — otherwise a blocked click looks like
	// nothing happened and you can't tell the shield is even working.
	const blocked = ref(false);
	const blockedTimer = ref<ReturnType<typeof setTimeout>>();
	// Whether the helper is allowed to drive this device's mouse/keyboard. The host
	// grants it by default (set from host_active on mount); the client's own
	// takeover still starts off, so viewing never silently becomes controlling.
	const controlGranted = ref(false);

	const unlistenConnected = ref<UnlistenFn | null>(null);
	const unlistenDisconnected = ref<UnlistenFn | null>(null);
	const unlistenControl = ref<UnlistenFn | null>(null);

	// Collapse is pure CSS now: the window stays at a fixed size/position (runtime
	// resize+reposition desync on GNOME/Wayland and flung the badge off-screen).
	// The pill hugs the window's right edge, so collapsing just swaps the content
	// to the "< dot" peek tab — the rest of the fixed window is transparent.
	async function toggle(): Promise<void> {
		collapsed.value = !collapsed.value;
		confirming.value = false;
		// WebKitGTK render-throttles this unfocused, transparent, always-on-top
		// window to ~2fps and won't present the just-collapsed frame for 10-20s. DOM
		// nudges (opacity / display toggle / paint) don't help — it's a surface-
		// present stall, not a paint one. Once the DOM has reflowed, ask the native
		// side to nudge the window geometry, which forces an immediate compositor
		// commit of the new frame. See `overlay_poke` in lib.rs.
		await nextTick();
		void invoke("overlay_poke").catch(() => { /* window gone */ });
	}

	// True when this click is one the controlling client injected onto the badge
	// (it drives the host cursor, so it can reach these buttons). The agent stamps
	// every press it injects; the backend decides + logs the verdict (so it always
	// lands in the app log) and we ignore an injected click. A physical host click
	// has no recent injection. So only the real host can kick / flip control.
	async function fromRemote(button: string): Promise<boolean> {
		const block = await invoke<boolean>("host_overlay_block_click", { button }).catch(() => false);
		if (block) {
			// Flash the badge so the host sees the client's click was rejected.
			blocked.value = true;
			if (blockedTimer.value !== undefined) {
				clearTimeout(blockedTimer.value);
			}
			blockedTimer.value = setTimeout(() => {
				blocked.value = false;
			}, 900);
		}
		return block;
	}

	// Start the kick confirmation — host-physical only.
	async function startKick(): Promise<void> {
		if (await fromRemote("kick")) {
			return;
		}
		confirming.value = true;
	}

	// Kick the helper. The backend drops the viewer and closes this window, so
	// there's nothing to clean up here. Host-physical only.
	async function doKick(): Promise<void> {
		if (await fromRemote("kick-confirm")) {
			return;
		}
		await invoke("host_disconnect").catch(() => { /* already gone */ });
	}

	// Grant or revoke the helper's mouse/keyboard control. Optimistic; the backend
	// echoes "host://control" to confirm and keep every host surface in sync.
	// Host-physical only — the client can't flip its own control off/on.
	async function toggleControl(): Promise<void> {
		if (await fromRemote("control")) {
			return;
		}
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
		unlistenConnected.value = await listen<string>("host://connected", (e) => {
			peer.value = e.payload;
			// The window is reused across sessions, so start each one expanded.
			collapsed.value = false;
			confirming.value = false;
		});
		unlistenDisconnected.value = await listen("host://disconnected", () => {
			peer.value = null;
		});
		unlistenControl.value = await listen<boolean>("host://control", (e) => {
			controlGranted.value = e.payload;
		});
	});

	onUnmounted(() => {
		unlistenConnected.value?.();
		unlistenDisconnected.value?.();
		unlistenControl.value?.();
		if (blockedTimer.value !== undefined) {
			clearTimeout(blockedTimer.value);
		}
	});
</script>

<style scoped>
	.badge {
		position: relative; /* anchor the shield-flash overlay */
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

	/* Shield rejected an injected client click — flash red + shake briefly. */
	.badge.blocked {
		animation: shake 0.4s ease-in-out;
		box-shadow: 0 0 0 2px #ff1744, 0 0 14px rgba(255, 23, 68, 0.7);
	}

	@keyframes shake {
		0%, 100% {
			transform: translateX(0);
		}
		25% {
			transform: translateX(-5px);
		}
		75% {
			transform: translateX(5px);
		}
	}

	/* The "host only" overlay covering the badge content during the flash. */
	.shield-flash {
		position: absolute;
		inset: 0;
		z-index: 2;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		border-radius: 12px;
		background: rgba(20, 20, 22, 0.96);
		color: #ff6b81;
		font-size: 0.9rem;
		font-weight: 600;
		white-space: nowrap;
	}

	.shield-flash .mdi {
		font-size: 1.3rem;
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
