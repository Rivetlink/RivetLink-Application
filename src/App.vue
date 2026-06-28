<template>
	<!-- The host "being viewed" badge renders its component DIRECTLY off the
	     window label — no Vuetify shell, no router/store wait — so the window
	     stays transparent and never flashes the opaque loading spinner. -->
	<OverlayPanel v-if="overlayKind === 'panel'" />

	<VApp v-else>
		<!-- Standalone windows (e.g. the live viewer) render bare, no shell. -->
		<RouterView v-if="route.meta.bare" />

		<template v-else>
			<!-- Loading -->
			<VMain v-if="!store.loaded">
				<VContainer class="fill-height d-flex align-center justify-center">
					<VProgressCircular indeterminate color="primary" size="40" />
				</VContainer>
			</VMain>

			<!-- First-run onboarding -->
			<VMain v-else-if="!store.settings.setup_complete">
				<Onboarding @done="router.replace('/')" />
			</VMain>

			<!-- Main shell -->
			<template v-else>
				<VNavigationDrawer v-model="drawer" color="grey-darken-4">
					<div class="pa-4 d-flex align-center">
						<VIcon icon="mdi-shield-lock-outline" color="primary" class="mr-2" />
						<span class="text-h6">RivetLink</span>
					</div>
					<VDivider />
					<VList nav density="comfortable">
						<VListItem
							v-for="item in navItems"
							:key="item.path"
							:to="item.path"
							:prepend-icon="item.icon"
							:title="t(item.titleKey)"
						/>
					</VList>
					<template #append>
						<div class="pa-3 text-caption text-medium-emphasis">
							{{ store.settings.device_name || t("app.unnamedDevice") }}
						</div>
					</template>
				</VNavigationDrawer>

				<VAppBar flat color="grey-darken-4" density="comfortable">
					<VAppBarNavIcon @click="drawer = !drawer" />
					<VAppBarTitle>{{ currentTitle }}</VAppBarTitle>
				</VAppBar>

				<VMain>
					<RouterView />
				</VMain>
			</template>

			<!-- Check for updates (from the RivetLink native menu) -->
			<UpdateModal />

			<!-- Host consent: approve an incoming connection -->
			<ConsentModal />
		</template>
	</VApp>
</template>

<script setup lang="ts">
	import {
		computed, onMounted, onUnmounted, ref,
	} from "vue";
	import { useRoute } from "vue-router";
	import { useI18n } from "vue-i18n";
	import { invoke } from "@tauri-apps/api/core";
	import {
		listen, type UnlistenFn,
	} from "@tauri-apps/api/event";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import { router } from "./router";
	import {
		isClient, isHost, loadSettings, refreshHostState, startHost, store,
	} from "./store";
	import {
		checkForUpdates, checkForUpdatesOnStartup,
	} from "./updates";
	import Onboarding from "./views/Onboarding.vue";
	import UpdateModal from "./components/UpdateModal.vue";
	import ConsentModal from "./components/ConsentModal.vue";
	import OverlayPanel from "./views/OverlayPanel.vue";

	const route = useRoute();
	const { t } = useI18n();
	const drawer = ref(true);

	// The host badge window is dedicated to one component. Pick it synchronously
	// from the window label so it never goes through the router/store/loadSettings
	// path (which left the window stuck on the opaque loading shell).
	const overlayKind = ((): "panel" | null => {
		return getCurrentWindow().label === "hostpanel" ? "panel" : null;
	})();
	const unlistenMenu = ref<UnlistenFn | null>(null);
	const unlistenConnected = ref<UnlistenFn | null>(null);
	const unlistenDisconnected = ref<UnlistenFn | null>(null);

	// Developer console is off by default; toggle it with Ctrl/Cmd+Shift+I or F12.
	function onKeydown(e: KeyboardEvent) {
		const toggleCombo = (e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === "i";
		if (toggleCombo || e.key === "F12") {
			e.preventDefault();
			invoke("toggle_devtools").catch(() => { /* devtools unavailable */ });
		}
	}

	onMounted(async () => {
		// Overlay windows render a self-contained component; skip the whole app
		// boot (settings load, update check, host autostart, global listeners).
		if (overlayKind) {
			return;
		}
		window.addEventListener("keydown", onKeydown);
		// The native "RivetLink -> Check for Updates" menu item fires this event.
		unlistenMenu.value = await listen("menu://check-updates", () => checkForUpdates());
		// Track the active LAN live session for the "connected" badge.
		unlistenConnected.value = await listen<string>("lan://connected", (e) => {
			store.connectedLanId = e.payload;
		});
		unlistenDisconnected.value = await listen("lan://disconnected", () => {
			store.connectedLanId = null;
		});
		await loadSettings();
		if (getCurrentWindow().label === "main" && store.settings.setup_complete) {
			// Check for updates once on launch. Silent unless something's waiting:
			// a normal release pops a dismissable dialog, a forced one a locked
			// dialog. Fire-and-forget so a slow network never delays the app.
			void checkForUpdatesOnStartup();
			// A host-only device has no use for the client "Connect" page — open
			// the host page instead of the default /connect landing.
			if (isHost() && !isClient() && route.path === "/connect") {
				await router.replace("/device");
			}
			// Host role: behave like a daemon — start advertising + serving as
			// soon as the app is open so a client can always connect, no "start
			// sharing" needed. A trusted client skips the code entirely.
			if (isHost()) {
				await refreshHostState();
				if (!store.hosting) {
					try {
						await startHost();
					} catch {
						// Host backend unavailable (e.g. Windows) — ignore.
					}
				}
			}
		}
	});

	onUnmounted(() => {
		window.removeEventListener("keydown", onKeydown);
		unlistenMenu.value?.();
		unlistenConnected.value?.();
		unlistenDisconnected.value?.();
	});

	// Nav items, filtered by the roles the user enabled during onboarding.
	const navItems = computed(() => {
		return router.options.routes
			.filter((r) => r.meta && r.meta.titleKey)
			.filter((r) => {
				const role = r.meta?.role;
				if (role === "client") {return isClient();}
				if (role === "host") {return isHost();}
				return true;
			})
			.map((r) => ({
				path: r.path,
				titleKey: r.meta?.titleKey as string,
				icon: r.meta?.icon as string,
			}));
	});

	const currentTitle = computed(() => {
		const key = route.meta?.titleKey as string | undefined;
		return key ? t(key) : "RivetLink";
	});
</script>
