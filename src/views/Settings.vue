<template>
	<div class="rl-page">
		<h1 class="rl-h1">
			{{ t("nav.settings") }}
		</h1>

		<div class="settings-layout">
			<nav class="settings-nav" :aria-label="t('nav.settings')">
				<button
					v-for="s in sections"
					:key="s.id"
					type="button"
					class="settings-nav-item"
					:class="{ active: section === s.id }"
					:aria-current="section === s.id ? 'page' : undefined"
					@click="section = s.id"
				>
					{{ s.label }}
				</button>
			</nav>

			<div class="settings-body">
				<!-- General -->
				<template v-if="section === 'general'">
					<h2 class="rl-h2">
						{{ t("settings.tabs.general") }}
					</h2>
					<section>
						<h3 class="rl-section-title">
							{{ t("settings.thisDevice") }}
						</h3>
						<div class="rl-group">
							<div class="rl-row">
								<div class="rl-row-text">
									<span class="rl-row-title">{{ t("settings.name") }}</span>
									<span class="rl-row-sub">{{ store.settings.device_name || "—" }}</span>
								</div>
								<VBtn variant="outlined" prepend-icon="mdi-pencil-outline" @click="editOpen = true">
									{{ t("common.edit") }}
								</VBtn>
							</div>
							<div class="rl-row">
								<div class="rl-row-text">
									<span class="rl-row-title">{{ t("settings.roles") }}</span>
								</div>
								<div class="d-flex flex-wrap ga-2">
									<VChip
										v-if="isHost()"
										label
										size="small"
										prepend-icon="mdi-lifebuoy"
									>
										{{ t("common.host") }}
									</VChip>
									<VChip
										v-if="isClient()"
										label
										size="small"
										prepend-icon="mdi-monitor"
									>
										{{ t("common.client") }}
									</VChip>
									<span v-if="!isHost() && !isClient()">—</span>
								</div>
							</div>
						</div>
					</section>
					<section>
						<h3 class="rl-section-title">
							{{ t("settings.preferences") }}
						</h3>
						<div class="rl-group">
							<div class="rl-row">
								<div class="rl-row-text">
									<span class="rl-row-title">{{ t("settings.language") }}</span>
								</div>
								<VSelect
									:model-value="locale"
									:items="SUPPORTED"
									item-title="label"
									item-value="code"
									density="compact"
									hide-details
									class="row-control"
									@update:model-value="onLocaleChange"
								/>
							</div>
							<div class="rl-row">
								<div class="rl-row-text">
									<span class="rl-row-title">{{ t("settings.scrollSpeed") }}</span>
									<span class="rl-row-sub">{{ t("settings.scrollHint") }}</span>
								</div>
								<VSelect
									:model-value="store.settings.scroll_speed"
									:items="scrollSpeedOptions"
									item-title="label"
									item-value="value"
									:loading="scrollSpeedBusy"
									:disabled="scrollSpeedBusy"
									density="compact"
									hide-details
									class="row-control"
									@update:model-value="onScrollSpeedChange"
								/>
							</div>
							<div class="rl-row">
								<div class="rl-row-text">
									<span class="rl-row-title">{{ t("settings.videoToggle") }}</span>
									<span class="rl-row-sub">{{ t("settings.videoHint") }}</span>
								</div>
								<VSwitch
									:model-value="store.settings.video_h264"
									:aria-label="t('settings.videoToggle')"
									density="compact"
									hide-details
									class="flex-grow-0"
									:loading="videoBusy"
									:disabled="videoBusy"
									@update:model-value="onVideoChange"
								/>
							</div>
							<div class="rl-row">
								<div class="rl-row-text">
									<span class="rl-row-title">{{ t("settings.startupToggle") }}</span>
									<span class="rl-row-sub">{{ t("settings.startupHint") }}</span>
								</div>
								<VSwitch
									:model-value="autostart"
									:aria-label="t('settings.startupToggle')"
									density="compact"
									hide-details
									class="flex-grow-0"
									:loading="autostartBusy"
									@update:model-value="onAutostartChange"
								/>
							</div>
						</div>
					</section>
				</template>

				<!-- Security -->
				<template v-else-if="section === 'security'">
					<h2 class="rl-h2">
						{{ t("settings.tabs.security") }}
					</h2>
					<section>
						<h3 class="rl-section-title">
							{{ t("settings.identityTitle") }}
						</h3>
						<div class="rl-group">
							<div class="rl-row">
								<div class="rl-row-text">
									<span class="rl-row-title">{{ t("settings.publicKey") }}</span>
									<span class="rl-mono key-text">{{ store.publicKey || "—" }}</span>
									<span class="rl-row-sub">{{ t("settings.privateNote") }}</span>
								</div>
								<VBtn
									variant="outlined"
									:prepend-icon="keyCopied ? 'mdi-check' : 'mdi-content-copy'"
									:disabled="!store.publicKey"
									@click="copyKey"
								>
									{{ keyCopied ? t("settings.copied") : t("settings.copyKey") }}
								</VBtn>
							</div>
						</div>
					</section>
					<section v-if="isHost()">
						<div class="d-flex align-center justify-space-between flex-wrap ga-2 mb-2">
							<h3 class="rl-section-title mb-0">
								{{ t("access.title") }}
							</h3>
							<VBtn variant="outlined" prepend-icon="mdi-plus" @click="openAdd">
								{{ t("access.addBtn") }}
							</VBtn>
						</div>
						<p class="rl-row-sub mt-0 mb-3">
							{{ t("access.subtitle") }}
						</p>
						<div class="rl-group">
							<div v-if="store.settings.trusted_keys.length === 0" class="rl-row rl-row-sub">
								{{ t("access.empty") }}
							</div>
							<div v-for="k in store.settings.trusted_keys" :key="k.id" class="rl-row">
								<VIcon icon="mdi-shield-check-outline" color="success" size="20" />
								<div class="rl-row-text">
									<span class="rl-row-title">{{ k.name || "—" }}</span>
									<span class="rl-mono key-text">{{ k.public_key }}</span>
								</div>
								<VBtn
									variant="text"
									icon="mdi-delete-outline"
									:aria-label="t('access.removeTitle')"
									@click="openRemove(k)"
								/>
							</div>
						</div>
					</section>
				</template>

				<!-- Servers -->
				<Relays v-else-if="section === 'servers'" />

				<!-- Ubuntu physical console -->
				<template v-else-if="section === 'console'">
					<div>
						<div class="d-flex align-center flex-wrap ga-3">
							<h2 class="rl-h2">
								{{ t("physicalConsole.title") }}
							</h2>
							<span v-if="physicalConsole.configured" class="rl-status" :class="consoleHealthy ? 'ok' : 'warn'">
								{{ consoleHealthy ? t("physicalConsole.running") : t("physicalConsole.needsAttention") }}
							</span>
						</div>
						<p class="rl-lead">
							{{ physicalConsole.configured ? t("physicalConsole.configuredHint") : t("physicalConsole.intro") }}
						</p>
					</div>
					<VAlert
						v-if="physicalConsole.lightdmLoginConfigured && !physicalConsole.lightdmLoginEnabled && physicalConsole.loginManager === 'lightdm' && physicalConsole.gdmAvailable"
						type="warning"
						variant="tonal"
						icon="mdi-restart-alert"
					>
						<strong>{{ t("physicalConsole.lightdmRebootRequiredTitle") }}</strong>
						<div>{{ t("physicalConsole.lightdmRebootRequiredHint") }}</div>
					</VAlert>
					<VAlert
						v-if="gdmRestorePending"
						type="info"
						variant="tonal"
						icon="mdi-restart"
					>
						{{ t("physicalConsole.gdmRestoredHint") }}
					</VAlert>
					<template v-if="physicalConsole.configured">
						<section v-if="physicalConsole.lanEnabled || physicalConsole.relayEnabled">
							<h3 class="rl-section-title">
								{{ t("physicalConsole.reachableVia") }}
							</h3>
							<div class="rl-group">
								<div v-if="physicalConsole.lanEnabled" class="rl-row">
									<div class="rl-row-text">
										<span class="rl-row-title">{{ t("physicalConsole.localNetwork") }}</span>
									</div>
									<span class="rl-status" :class="physicalConsole.lanListening ? 'ok' : 'warn'">
										{{ physicalConsole.lanListening ? t('physicalConsole.listening', { port: physicalConsole.lanPort || '—' }) : t('physicalConsole.notListening') }}
									</span>
								</div>
								<div v-if="physicalConsole.relayEnabled" class="rl-row">
									<div class="rl-row-text">
										<span class="rl-row-title">{{ t("physicalConsole.relay") }}</span>
									</div>
									<span class="rl-status ok">{{ t('physicalConsole.configured') }}</span>
								</div>
							</div>
						</section>
						<section>
							<h3 class="rl-section-title">
								{{ t("physicalConsole.statusTitle") }}
							</h3>
							<div class="rl-group">
								<div class="rl-row">
									<div class="rl-row-text">
										<span class="rl-row-title">{{ t("physicalConsole.bootService") }}</span>
									</div>
									<span class="rl-status" :class="physicalConsole.bootServiceEnabled ? 'ok' : 'warn'">
										{{ physicalConsole.bootServiceEnabled ? t('physicalConsole.active') : t('physicalConsole.inactive') }}
									</span>
								</div>
								<div class="rl-row">
									<div class="rl-row-text">
										<span class="rl-row-title">{{ t("physicalConsole.serviceAgent") }}</span>
									</div>
									<span class="rl-status" :class="physicalConsole.nativeServiceAgentCurrent ? 'ok' : 'warn'">
										{{ physicalConsole.nativeServiceAgentCurrent ? t('physicalConsole.native') : t('physicalConsole.updateRequired') }}
									</span>
								</div>
								<div class="rl-row">
									<div class="rl-row-text">
										<span class="rl-row-title">{{ t("physicalConsole.gdm") }}</span>
									</div>
									<span class="rl-status" :class="physicalConsole.gdmAvailable ? 'ok' : 'warn'">
										{{ physicalConsole.gdmAvailable ? t('physicalConsole.available') : t('physicalConsole.unavailable') }}
									</span>
								</div>
								<div class="rl-row">
									<div class="rl-row-text">
										<span class="rl-row-title">{{ t("physicalConsole.loginScreen") }}</span>
									</div>
									<span class="rl-status" :class="physicalConsole.lightdmLoginEnabled ? 'ok' : 'warn'">
										{{
											physicalConsole.lightdmLoginEnabled
												? t('physicalConsole.lightdmEnabled')
												: physicalConsole.lightdmLoginConfigured
													? t('physicalConsole.lightdmPending')
													: t('physicalConsole.gdmProtected')
										}}
									</span>
								</div>
							</div>
						</section>
					</template>
					<div class="d-flex flex-wrap ga-2">
						<VBtn
							v-if="!physicalConsole.configured"
							color="primary"
							prepend-icon="mdi-monitor-lock"
							@click="physicalConsoleDialog = true"
						>
							{{ t("physicalConsole.setup") }}
						</VBtn>
						<template v-else>
							<VBtn
								v-if="physicalConsole.lightdmLoginConfigured"
								color="warning"
								variant="tonal"
								prepend-icon="mdi-restore"
								:loading="physicalConsoleBusy"
								@click="restorePhysicalConsoleGdm"
							>
								{{ t("physicalConsole.restoreGdm") }}
							</VBtn>
							<VBtn
								v-if="physicalConsole.lightdmLoginConfigured && !physicalConsole.lightdmLoginEnabled"
								variant="outlined"
								prepend-icon="mdi-refresh"
								:loading="physicalConsoleBusy"
								@click="retryPhysicalConsoleLightdm"
							>
								{{ t("physicalConsole.retryLightdm") }}
							</VBtn>
							<VBtn
								v-if="!physicalConsole.nativeServiceAgentCurrent"
								color="primary"
								prepend-icon="mdi-update"
								:loading="physicalConsoleBusy"
								@click="updatePhysicalConsoleAgent"
							>
								{{ t("physicalConsole.updateAgent") }}
							</VBtn>
							<VBtn
								variant="outlined"
								prepend-icon="mdi-cog-outline"
								:disabled="physicalConsoleBusy"
								@click="physicalConsoleDialog = true"
							>
								{{ t("physicalConsole.update") }}
							</VBtn>
							<VBtn
								variant="outlined"
								prepend-icon="mdi-refresh"
								@click="refreshPhysicalConsoleStatus"
							>
								{{ t("physicalConsole.refresh") }}
							</VBtn>
							<VSpacer />
							<VBtn
								v-if="physicalConsole.bootServiceEnabled"
								color="error"
								variant="tonal"
								prepend-icon="mdi-stop-circle-outline"
								:loading="physicalConsoleBusy"
								@click="setPhysicalConsoleService('disable')"
							>
								{{ t("physicalConsole.disable") }}
							</VBtn>
							<VBtn
								v-else
								color="success"
								variant="tonal"
								prepend-icon="mdi-play-circle-outline"
								:loading="physicalConsoleBusy"
								@click="setPhysicalConsoleService('enable')"
							>
								{{ t("physicalConsole.enable") }}
							</VBtn>
						</template>
					</div>
					<VAlert
						v-if="physicalConsoleServiceError"
						type="error"
						variant="tonal"
					>
						{{ physicalConsoleServiceError }}
					</VAlert>
				</template>

				<!-- About & updates -->
				<template v-else-if="section === 'about'">
					<h2 class="rl-h2">
						{{ t("settings.tabs.about") }}
					</h2>
					<div class="rl-group">
						<div class="rl-row">
							<div class="rl-row-text">
								<span class="rl-row-title">RivetLink {{ version || "—" }}</span>
								<span class="rl-row-sub">{{ t("settings.tagline") }}</span>
							</div>
							<VBtn variant="outlined" prepend-icon="mdi-update" @click="checkForUpdates">
								{{ t("settings.checkUpdates") }}
							</VBtn>
						</div>
					</div>
					<Resources />
				</template>
			</div>
		</div>

		<EditDeviceModal v-model="editOpen" />
		<TrustedKeyModal v-model="accessOpen" :target="accessTarget" />
		<VDialog v-model="physicalConsoleDialog" max-width="580" persistent>
			<VCard>
				<VCardTitle>{{ t("physicalConsole.dialogTitle") }}</VCardTitle>
				<VCardText>
					<VAlert type="info" variant="tonal" class="mb-4">
						{{ t("physicalConsole.warning") }}
					</VAlert>
					<p class="text-body-2 mb-4">
						{{ t("physicalConsole.scope") }}
					</p>
					<VTextField
						v-model="physicalConsoleName"
						:label="t('physicalConsole.name')"
						density="comfortable"
						class="mb-2"
					/>
					<VSelect
						v-if="store.settings.trusted_keys.length > 0"
						v-model="physicalConsoleControllerKeys"
						:items="store.settings.trusted_keys"
						item-title="name"
						item-value="public_key"
						:label="t('physicalConsole.savedControllers')"
						:hint="t('physicalConsole.savedControllersHint')"
						multiple
						chips
						closable-chips
						persistent-hint
						density="comfortable"
						class="mb-2"
					/>
					<VTextField
						v-model="physicalConsoleControllerKey"
						:label="t('physicalConsole.additionalControllerKey')"
						:hint="t('physicalConsole.controllerKeyHint')"
						persistent-hint
						density="comfortable"
						class="mb-2"
					/>
					<VCheckbox
						v-model="physicalConsoleLan"
						:label="t('physicalConsole.localNetwork')"
						:hint="t('physicalConsole.localNetworkHint')"
						persistent-hint
						density="comfortable"
					/>
					<VCheckbox
						v-model="physicalConsoleRelay"
						:label="t('physicalConsole.relay')"
						:hint="t('physicalConsole.relayHint')"
						persistent-hint
						density="comfortable"
					/>
					<VCheckbox
						v-model="physicalConsoleLightdmLogin"
						:label="t('physicalConsole.lightdmOptIn')"
						:hint="t('physicalConsole.lightdmOptInHint')"
						persistent-hint
						density="comfortable"
					/>
					<VAlert
						v-if="physicalConsoleError"
						type="error"
						variant="tonal"
						class="mt-4"
					>
						{{ physicalConsoleError }}
					</VAlert>
				</VCardText>
				<VCardActions>
					<VSpacer />
					<VBtn
						variant="text"
						:disabled="physicalConsoleBusy"
						@click="closePhysicalConsoleDialog"
					>
						{{ t("common.cancel") }}
					</VBtn>
					<VBtn
						color="primary"
						variant="flat"
						:loading="physicalConsoleBusy"
						:disabled="!canInstallPhysicalConsole()"
						@click="installPhysicalConsole"
					>
						{{ t("physicalConsole.confirm") }}
					</VBtn>
				</VCardActions>
			</VCard>
		</VDialog>
	</div>
</template>

<script setup lang="ts">
	import {
		computed, onMounted, ref,
	} from "vue";
	import { invoke } from "@tauri-apps/api/core";
	import {
		disable, enable, isEnabled,
	} from "@tauri-apps/plugin-autostart";
	import { useI18n } from "vue-i18n";
	import { useRoute } from "vue-router";
	import {
		isClient, isHost, loadPublicKey, setScrollSpeed, setVideoH264, store, type ScrollSpeed, type TrustedKey,
	} from "../store";
	import {
		SUPPORTED, setLocale,
	} from "../i18n";
	import { checkForUpdates } from "../updates";
	import EditDeviceModal from "../components/EditDeviceModal.vue";
	import TrustedKeyModal from "../components/TrustedKeyModal.vue";
	import Relays from "../components/Relays.vue";
	import Resources from "../components/Resources.vue";

	const {
		t, locale,
	} = useI18n();

	// `?s=servers` (e.g. from the Devices page) opens that section directly.
	const section = ref(String(useRoute().query.s ?? "general"));
	const sections = computed(() => [
		{
			id: "general",
			label: t("settings.tabs.general"),
		},
		{
			id: "security",
			label: t("settings.tabs.security"),
		},
		{
			id: "servers",
			label: t("settings.tabs.servers"),
		},
		...(physicalConsole.value.supported
			? [{
				id: "console",
				label: t("settings.tabs.console"),
			}]
			: []),
		{
			id: "about",
			label: t("settings.tabs.about"),
		},
	]);
	const keyCopied = ref(false);
	async function copyKey() {
		await navigator.clipboard.writeText(store.publicKey);
		keyCopied.value = true;
		setTimeout(() => (keyCopied.value = false), 1500);
	}
	const editOpen = ref(false);
	const version = ref("");
	const accessOpen = ref(false);
	const accessTarget = ref<TrustedKey | null>(null);
	const autostart = ref(false);
	const autostartBusy = ref(false);
	const scrollSpeedBusy = ref(false);
	const videoBusy = ref(false);
	const scrollSpeedOptions = computed(() => [
		{
			label: t("settings.scrollSlow"),
			value: "slow" as ScrollSpeed,
		},
		{
			label: t("settings.scrollNormal"),
			value: "normal" as ScrollSpeed,
		},
		{
			label: t("settings.scrollFast"),
			value: "fast" as ScrollSpeed,
		},
	]);
	const physicalConsoleDialog = ref(false);
	const physicalConsoleBusy = ref(false);
	const physicalConsoleError = ref("");
	const physicalConsoleServiceError = ref("");
	const physicalConsoleName = ref("");
	const physicalConsoleControllerKey = ref("");
	const physicalConsoleControllerKeys = ref<string[]>([]);
	const physicalConsoleLan = ref(true);
	const physicalConsoleRelay = ref(true);
	const physicalConsoleLightdmLogin = ref(false);
	const physicalConsole = ref({
		supported: false,
		configured: false,
		brokerActive: false,
		bootServiceEnabled: false,
		nativeServiceAgent: false,
		nativeServiceAgentCurrent: false,
		gdmAvailable: false,
		lightdmLoginEnabled: false,
		lightdmLoginConfigured: false,
		loginManager: "unknown",
		lanListening: false,
		lanPort: null as number | null,
		lanEnabled: false,
		relayEnabled: false,
	});
	const consoleHealthy = computed(() =>
		physicalConsole.value.brokerActive && physicalConsole.value.nativeServiceAgentCurrent);

	async function refreshPhysicalConsoleStatus() {
		try {
			physicalConsole.value = await invoke<typeof physicalConsole.value>("physical_console_status");
		} catch {
			// The setup card is optional; leave it hidden if the OS cannot report.
		}
	}

	function closePhysicalConsoleDialog() {
		physicalConsoleDialog.value = false;
		physicalConsoleError.value = "";
	}

	async function installPhysicalConsole() {
		physicalConsoleBusy.value = true;
		physicalConsoleError.value = "";
		try {
			physicalConsole.value = await invoke<typeof physicalConsole.value>("setup_physical_console", {
				setup: {
					deviceName: physicalConsoleName.value,
					controllerPublicKey: physicalConsoleControllerKey.value,
					controllerPublicKeys: physicalConsoleControllerKeys.value,
					enableLan: physicalConsoleLan.value,
					enableRelay: physicalConsoleRelay.value,
					enableLightdmLogin: physicalConsoleLightdmLogin.value,
				},
			});
			closePhysicalConsoleDialog();
		} catch (error) {
			physicalConsoleError.value = String(error);
		} finally {
			physicalConsoleBusy.value = false;
		}
	}

	function canInstallPhysicalConsole(): boolean {
		if (!physicalConsoleName.value.trim() || (!physicalConsoleLan.value && !physicalConsoleRelay.value)) {
			return false;
		}
		// An already installed broker owns its existing root-managed allow-list;
		// reconfigure/update may safely reuse it without making the owner paste a
		// key again. A first installation must name at least one controller.
		return physicalConsole.value.configured
			|| physicalConsoleControllerKeys.value.length > 0
			|| physicalConsoleControllerKey.value.trim().length > 0;
	}

	async function retryPhysicalConsoleLightdm() {
		physicalConsoleBusy.value = true;
		physicalConsoleServiceError.value = "";
		try {
			physicalConsole.value = await invoke<typeof physicalConsole.value>("enable_physical_console_lightdm");
		} catch (error) {
			physicalConsoleServiceError.value = String(error);
		} finally {
			physicalConsoleBusy.value = false;
		}
	}

	// GDM returns only at the next boot; say so once LightDM was handed back.
	const gdmRestorePending = ref(false);
	function noteGdmRestore(before: boolean): void {
		if (before && !physicalConsole.value.lightdmLoginConfigured) {
			gdmRestorePending.value = true;
		}
	}

	async function setPhysicalConsoleService(action: "enable" | "disable") {
		physicalConsoleBusy.value = true;
		physicalConsoleServiceError.value = "";
		try {
			const before = physicalConsole.value.lightdmLoginConfigured;
			physicalConsole.value = await invoke<typeof physicalConsole.value>("physical_console_service_action", { action });
			noteGdmRestore(before);
		} catch (error) {
			physicalConsoleServiceError.value = String(error);
		} finally {
			physicalConsoleBusy.value = false;
		}
	}

	async function restorePhysicalConsoleGdm() {
		physicalConsoleBusy.value = true;
		physicalConsoleServiceError.value = "";
		try {
			const before = physicalConsole.value.lightdmLoginConfigured;
			physicalConsole.value = await invoke<typeof physicalConsole.value>("restore_physical_console_gdm");
			noteGdmRestore(before);
		} catch (error) {
			physicalConsoleServiceError.value = String(error);
		} finally {
			physicalConsoleBusy.value = false;
		}
	}

	async function updatePhysicalConsoleAgent() {
		physicalConsoleBusy.value = true;
		physicalConsoleServiceError.value = "";
		try {
			await invoke<boolean>("update_physical_console_agent_if_needed");
			await refreshPhysicalConsoleStatus();
		} catch (error) {
			physicalConsoleServiceError.value = String(error);
		} finally {
			physicalConsoleBusy.value = false;
		}
	}

	async function onAutostartChange(value: boolean | null) {
		autostartBusy.value = true;
		try {
			if (value) {
				await enable();
			} else {
				await disable();
			}
		} catch {
			// Ignore; we re-read the real state below.
		}
		try {
			autostart.value = await isEnabled();
		} catch {
			// Keep the previous value if the platform can't report it.
		}
		autostartBusy.value = false;
	}

	async function onScrollSpeedChange(value: ScrollSpeed | null) {
		if (value === null) {
			return;
		}
		scrollSpeedBusy.value = true;
		try {
			await setScrollSpeed(value);
		} finally {
			scrollSpeedBusy.value = false;
		}
	}

	async function onVideoChange(value: boolean | null) {
		videoBusy.value = true;
		try {
			await setVideoH264(Boolean(value));
		} finally {
			videoBusy.value = false;
		}
	}

	function openAdd() {
		accessTarget.value = null;
		accessOpen.value = true;
	}

	function openRemove(key: TrustedKey) {
		accessTarget.value = key;
		accessOpen.value = true;
	}

	onMounted(async () => {
		version.value = await invoke<string>("app_version");
		if (!store.publicKey) {
			await loadPublicKey();
		}
		try {
			autostart.value = await isEnabled();
		} catch {
			// Autostart unsupported on this platform — leave the toggle off.
		}
		physicalConsoleName.value = store.settings.device_name || "RivetLink Home Node";
		// This dialog runs on the Ubuntu host. Its own key must never be used
		// as the remote controller key: doing so would leave the actual laptop
		// untrusted and make the console handshake fail after setup.
		physicalConsoleControllerKey.value = "";
		physicalConsoleControllerKeys.value = store.settings.trusted_keys.map((key) => key.public_key);
		await refreshPhysicalConsoleStatus();
	});

	function onLocaleChange(code: string) {
		setLocale(code);
	}
</script>

<style scoped>
	.settings-layout {
		display: flex;
		flex-wrap: wrap;
		gap: 28px;
		align-items: flex-start;
	}

	.settings-nav {
		flex: 1 1 180px;
		max-width: 220px;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.settings-nav-item {
		min-height: 40px;
		padding: 0 12px;
		border-radius: 8px;
		text-align: left;
		color: var(--rl-soft);
		font: inherit;
		cursor: pointer;
	}

	.settings-nav-item:hover {
		background: rgba(255, 255, 255, 0.04);
	}

	.settings-nav-item.active {
		background: var(--rl-raised);
		color: rgb(var(--v-theme-on-surface));
		font-weight: 600;
	}

	.settings-nav-item:focus-visible {
		outline: 2px solid var(--rl-accent);
		outline-offset: 2px;
	}

	.settings-body {
		flex: 999 1 420px;
		min-width: 0;
		max-width: 760px;
		display: flex;
		flex-direction: column;
		gap: 24px;
	}

	.row-control {
		flex: 0 0 200px;
	}

	.key-text {
		color: var(--rl-soft);
		overflow-wrap: anywhere;
	}
</style>
