<template>
	<VContainer style="max-width: 880px">
		<VTabs v-model="tab" class="mb-4">
			<VTab value="general">
				{{ t("settings.tabs.general") }}
			</VTab>
			<VTab value="security">
				{{ t("settings.tabs.security") }}
			</VTab>
			<VTab value="about">
				{{ t("settings.tabs.about") }}
			</VTab>
		</VTabs>

		<VWindow v-model="tab">
			<VWindowItem value="general">
				<VCard variant="tonal" class="mb-4">
					<VCardTitle class="d-flex align-center">
						{{ t("settings.thisDevice") }}
						<VSpacer />
						<VBtn
							variant="text"
							size="small"
							prepend-icon="mdi-pencil"
							@click="editOpen = true"
						>
							{{ t("common.edit") }}
						</VBtn>
					</VCardTitle>
					<VCardText>
						<div class="text-overline text-medium-emphasis">
							{{ t("settings.name") }}
						</div>
						<div class="text-body-1 mb-4">
							{{ store.settings.device_name || "—" }}
						</div>
						<div class="text-overline text-medium-emphasis">
							{{ t("settings.roles") }}
						</div>
						<div class="mt-1 d-flex flex-wrap ga-2">
							<VChip v-if="isHost()" size="small" prepend-icon="mdi-monitor-share">
								{{ t("common.host") }}
							</VChip>
							<VChip v-if="isClient()" size="small" prepend-icon="mdi-account-arrow-right">
								{{ t("common.client") }}
							</VChip>
							<span v-if="!isHost() && !isClient()">—</span>
						</div>
					</VCardText>
				</VCard>

				<VCard variant="tonal" class="mb-4">
					<VCardTitle>{{ t("settings.language") }}</VCardTitle>
					<VCardText>
						<VSelect
							:model-value="locale"
							:items="SUPPORTED"
							item-title="label"
							item-value="code"
							density="comfortable"
							hide-details
							prepend-inner-icon="mdi-translate"
							@update:model-value="onLocaleChange"
						/>
					</VCardText>
				</VCard>

				<VCard variant="tonal" class="mb-4">
					<VCardTitle>{{ t("settings.scrollTitle") }}</VCardTitle>
					<VCardText>
						<VSelect
							:model-value="store.settings.scroll_speed"
							:items="scrollSpeedOptions"
							item-title="label"
							item-value="value"
							:label="t('settings.scrollSpeed')"
							:loading="scrollSpeedBusy"
							:disabled="scrollSpeedBusy"
							density="comfortable"
							hide-details
							prepend-inner-icon="mdi-mouse-scroll-wheel"
							@update:model-value="onScrollSpeedChange"
						/>
						<p class="text-caption text-medium-emphasis mt-2 mb-0">
							{{ t("settings.scrollHint") }}
						</p>
					</VCardText>
				</VCard>

				<VCard variant="tonal">
					<VCardTitle>{{ t("settings.startupTitle") }}</VCardTitle>
					<VCardText>
						<VSwitch
							:model-value="autostart"
							:label="t('settings.startupToggle')"
							color="primary"
							density="comfortable"
							hide-details
							:loading="autostartBusy"
							@update:model-value="onAutostartChange"
						/>
						<p class="text-caption text-medium-emphasis mt-1 mb-0">
							{{ t("settings.startupHint") }}
						</p>
					</VCardText>
				</VCard>

				<VCard v-if="physicalConsole.supported" variant="tonal" class="mt-4">
					<VCardTitle class="d-flex align-center">
						{{ t("physicalConsole.title") }}
						<VSpacer />
						<VChip v-if="physicalConsole.configured" :color="physicalConsole.brokerActive && physicalConsole.nativeServiceAgentCurrent ? 'success' : 'warning'" size="small">
							{{ physicalConsole.brokerActive && physicalConsole.nativeServiceAgentCurrent ? t("physicalConsole.running") : t("physicalConsole.needsAttention") }}
						</VChip>
					</VCardTitle>
					<VCardText>
						<p class="text-body-2 mb-3">
							{{ physicalConsole.configured ? t("physicalConsole.configuredHint") : t("physicalConsole.intro") }}
						</p>
						<VAlert
							v-if="physicalConsole.lightdmLoginConfigured && !physicalConsole.lightdmLoginEnabled && physicalConsole.loginManager === 'lightdm' && physicalConsole.gdmAvailable"
							type="warning"
							variant="tonal"
							icon="mdi-restart-alert"
							class="mb-3"
						>
							<strong>{{ t("physicalConsole.lightdmRebootRequiredTitle") }}</strong>
							<div>{{ t("physicalConsole.lightdmRebootRequiredHint") }}</div>
						</VAlert>
						<VList v-if="physicalConsole.configured" density="compact" class="bg-transparent mb-2">
							<VListItem :title="t('physicalConsole.bootService')">
								<template #append>
									<VChip size="x-small" :color="physicalConsole.bootServiceEnabled ? 'success' : 'warning'">
										{{ physicalConsole.bootServiceEnabled ? t('physicalConsole.active') : t('physicalConsole.inactive') }}
									</VChip>
								</template>
							</VListItem>
							<VListItem :title="t('physicalConsole.serviceAgent')">
								<template #append>
									<VChip size="x-small" :color="physicalConsole.nativeServiceAgentCurrent ? 'success' : 'warning'">
										{{ physicalConsole.nativeServiceAgentCurrent ? t('physicalConsole.native') : t('physicalConsole.updateRequired') }}
									</VChip>
								</template>
							</VListItem>
							<VListItem :title="t('physicalConsole.gdm')">
								<template #append>
									<VChip size="x-small" :color="physicalConsole.gdmAvailable ? 'success' : 'warning'">
										{{ physicalConsole.gdmAvailable ? t('physicalConsole.available') : t('physicalConsole.unavailable') }}
									</VChip>
								</template>
							</VListItem>
							<VListItem :title="t('physicalConsole.loginScreen')">
								<template #append>
									<VChip
										size="x-small"
										:color="physicalConsole.lightdmLoginEnabled ? 'success' : 'warning'"
									>
										{{
											physicalConsole.lightdmLoginEnabled
												? t('physicalConsole.lightdmEnabled')
												: physicalConsole.lightdmLoginConfigured
													? t('physicalConsole.lightdmPending')
													: t('physicalConsole.gdmProtected')
										}}
									</VChip>
								</template>
							</VListItem>
							<VListItem v-if="physicalConsole.lanEnabled" :title="t('physicalConsole.localNetwork')">
								<template #append>
									<VChip size="x-small" :color="physicalConsole.lanListening ? 'success' : 'warning'">
										{{ physicalConsole.lanListening ? t('physicalConsole.listening', { port: physicalConsole.lanPort || '—' }) : t('physicalConsole.notListening') }}
									</VChip>
								</template>
							</VListItem>
							<VListItem v-if="physicalConsole.relayEnabled" :title="t('physicalConsole.relay')">
								<template #append>
									<VChip size="x-small" color="primary">
										{{ t('physicalConsole.configured') }}
									</VChip>
								</template>
							</VListItem>
						</VList>
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
								variant="text"
								prepend-icon="mdi-restore"
								:loading="physicalConsoleBusy"
								@click="restorePhysicalConsoleGdm"
							>
								{{ t("physicalConsole.restoreGdm") }}
							</VBtn>
							<VBtn
								v-if="physicalConsole.lightdmLoginConfigured && !physicalConsole.lightdmLoginEnabled"
								color="primary"
								variant="text"
								prepend-icon="mdi-refresh"
								:loading="physicalConsoleBusy"
								@click="retryPhysicalConsoleLightdm"
							>
								{{ t("physicalConsole.retryLightdm") }}
							</VBtn>
							<VBtn
								v-if="!physicalConsole.nativeServiceAgentCurrent"
								color="primary"
								variant="text"
								prepend-icon="mdi-update"
								:loading="physicalConsoleBusy"
								@click="updatePhysicalConsoleAgent"
							>
								{{ t("physicalConsole.updateAgent") }}
							</VBtn>
							<VBtn
								color="primary"
								variant="text"
								prepend-icon="mdi-cog-outline"
								:disabled="physicalConsoleBusy"
								@click="physicalConsoleDialog = true"
							>
								{{ t("physicalConsole.update") }}
							</VBtn>
							<VBtn
								v-if="physicalConsole.bootServiceEnabled"
								color="error"
								variant="text"
								prepend-icon="mdi-stop-circle-outline"
								:loading="physicalConsoleBusy"
								@click="setPhysicalConsoleService('disable')"
							>
								{{ t("physicalConsole.disable") }}
							</VBtn>
							<VBtn
								v-else
								color="success"
								variant="text"
								prepend-icon="mdi-play-circle-outline"
								:loading="physicalConsoleBusy"
								@click="setPhysicalConsoleService('enable')"
							>
								{{ t("physicalConsole.enable") }}
							</VBtn>
							<VBtn
								variant="text"
								prepend-icon="mdi-refresh"
								@click="refreshPhysicalConsoleStatus"
							>
								{{ t("physicalConsole.refresh") }}
							</VBtn>
						</template>
						<VAlert
							v-if="physicalConsoleServiceError"
							type="error"
							variant="tonal"
							class="mt-3"
						>
							{{ physicalConsoleServiceError }}
						</VAlert>
					</VCardText>
				</VCard>
			</VWindowItem>

			<VWindowItem value="security">
				<VCard variant="tonal" class="mb-4">
					<VCardTitle>{{ t("settings.identityTitle") }}</VCardTitle>
					<VCardText>
						<VTextField
							:model-value="store.publicKey"
							:label="t('settings.publicKey')"
							readonly
							density="comfortable"
							hide-details
						/>
						<p class="text-caption text-medium-emphasis mt-2">
							{{ t("settings.privateNote") }}
						</p>
					</VCardText>
				</VCard>

				<VCard v-if="isHost()" variant="tonal">
					<VCardTitle class="d-flex align-center">
						{{ t("access.title") }}
						<VSpacer />
						<VBtn
							variant="text"
							size="small"
							prepend-icon="mdi-plus"
							@click="openAdd"
						>
							{{ t("access.addBtn") }}
						</VBtn>
					</VCardTitle>
					<VCardSubtitle class="text-wrap">
						{{ t("access.subtitle") }}
					</VCardSubtitle>
					<VCardText>
						<p
							v-if="store.settings.trusted_keys.length === 0"
							class="text-body-2 text-medium-emphasis mb-0"
						>
							{{ t("access.empty") }}
						</p>
						<VList v-else class="bg-transparent">
							<VListItem v-for="k in store.settings.trusted_keys" :key="k.id">
								<template #prepend>
									<VIcon icon="mdi-key-chain" />
								</template>
								<VListItemTitle>{{ k.name || "—" }}</VListItemTitle>
								<VListItemSubtitle class="text-truncate">
									{{ k.public_key }}
								</VListItemSubtitle>
								<template #append>
									<VBtn
										size="small"
										variant="text"
										icon="mdi-delete-outline"
										@click="openRemove(k)"
									/>
								</template>
							</VListItem>
						</VList>
					</VCardText>
				</VCard>
			</VWindowItem>

			<VWindowItem value="about">
				<VCard variant="tonal">
					<VCardTitle>{{ t("settings.about") }}</VCardTitle>
					<VList class="bg-transparent">
						<VListItem title="RivetLink" :subtitle="t('settings.tagline')" />
						<VListItem :title="t('settings.version')" :subtitle="version || '—'" />
					</VList>
				</VCard>
			</VWindowItem>
		</VWindow>

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
						:disabled="physicalConsoleBusy"
						@click="closePhysicalConsoleDialog"
					>
						{{ t("common.cancel") }}
					</VBtn>
					<VBtn
						color="primary"
						:loading="physicalConsoleBusy"
						:disabled="!canInstallPhysicalConsole()"
						@click="installPhysicalConsole"
					>
						{{ t("physicalConsole.confirm") }}
					</VBtn>
				</VCardActions>
			</VCard>
		</VDialog>
	</VContainer>
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
	import {
		isClient, isHost, loadPublicKey, setScrollSpeed, store, type ScrollSpeed, type TrustedKey,
	} from "../store";
	import {
		SUPPORTED, setLocale,
	} from "../i18n";
	import EditDeviceModal from "../components/EditDeviceModal.vue";
	import TrustedKeyModal from "../components/TrustedKeyModal.vue";

	const {
		t, locale,
	} = useI18n();

	const tab = ref("general");
	const editOpen = ref(false);
	const version = ref("");
	const accessOpen = ref(false);
	const accessTarget = ref<TrustedKey | null>(null);
	const autostart = ref(false);
	const autostartBusy = ref(false);
	const scrollSpeedBusy = ref(false);
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

	async function setPhysicalConsoleService(action: "enable" | "disable") {
		physicalConsoleBusy.value = true;
		physicalConsoleServiceError.value = "";
		try {
			physicalConsole.value = await invoke<typeof physicalConsole.value>("physical_console_service_action", { action });
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
			physicalConsole.value = await invoke<typeof physicalConsole.value>("restore_physical_console_gdm");
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
