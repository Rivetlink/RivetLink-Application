<template>
	<VContainer style="max-width: 880px">
		<VCard variant="tonal" class="mb-4">
			<VCardText class="d-flex align-center">
				<VIcon icon="mdi-laptop" size="32" class="mr-3" />
				<div>
					<div class="text-h6">
						{{ store.settings.device_name || t("app.unnamedDevice") }}
					</div>
					<div class="text-caption text-medium-emphasis">
						{{ t("device.thisDevice") }}
					</div>
					<div v-if="net && (net.ssid || net.ip)" class="text-caption text-medium-emphasis mt-1">
						<VIcon :icon="net.ssid ? 'mdi-wifi' : 'mdi-lan'" size="x-small" class="mr-1" />
						<span v-if="net.ssid">{{ t("connect.lanNetworkWifi", { ssid: net.ssid }) }} · </span>
						<span v-if="net.ip">{{ t("connect.lanNetworkIp", { ip: net.ip }) }}</span>
					</div>
				</div>
				<VSpacer />
				<VChip :color="statusColor" size="small" variant="flat">
					<VIcon start icon="mdi-circle" size="x-small" /> {{ t(statusKey) }}
				</VChip>
			</VCardText>
		</VCard>

		<VCard variant="tonal" class="mb-4">
			<VCardTitle>{{ t("device.receiveHelpTitle") }}</VCardTitle>
			<VCardText>
				<p class="text-body-2 text-medium-emphasis mb-4">
					{{ t("device.receiveHelpIntro") }}
				</p>

				<VBtn
					v-if="!store.hosting"
					color="primary"
					prepend-icon="mdi-monitor-share"
					:loading="busy"
					@click="onStart"
				>
					{{ t("device.startHosting") }}
				</VBtn>

				<template v-else>
					<div class="d-flex align-center ga-4 mb-4">
						<div>
							<div class="text-overline text-medium-emphasis">
								{{ t("device.code") }}
							</div>
							<div class="code-display">
								{{ store.hostPin }}
							</div>
						</div>
						<VBtn
							variant="text"
							icon="mdi-content-copy"
							:title="t('device.copied')"
							@click="copyCode"
						/>
						<VFadeTransition>
							<span v-if="copied" class="text-caption text-success">
								{{ t("device.copied") }}
							</span>
						</VFadeTransition>
					</div>

					<VAlert
						:type="store.hostPeer ? 'success' : 'info'"
						variant="tonal"
						density="comfortable"
						class="mb-4"
					>
						<div class="text-subtitle-2">
							{{ store.hostPeer ? t("device.connectedTitle") : t("device.waitingTitle") }}
						</div>
						<div class="text-caption">
							{{ store.hostPeer ? t("device.connectedHint") : t("device.waitingHint") }}
						</div>
					</VAlert>

					<VBtn
						color="error"
						variant="tonal"
						prepend-icon="mdi-stop"
						:loading="busy"
						@click="onStop"
					>
						{{ t("device.stopHosting") }}
					</VBtn>
				</template>

				<p class="text-caption text-medium-emphasis mt-4 mb-0">
					{{ t("device.pickScreenNote") }}
				</p>
			</VCardText>
		</VCard>

		<VCard variant="tonal">
			<VCardTitle>{{ t("device.identityTitle") }}</VCardTitle>
			<VCardSubtitle class="text-wrap">
				{{ t("device.identitySubtitle") }}
			</VCardSubtitle>
			<VCardText>
				<VTextField
					:model-value="store.publicKey"
					:label="t('device.publicKey')"
					readonly
					density="comfortable"
					append-inner-icon="mdi-content-copy"
					@click:append-inner="copyKey"
				/>
			</VCardText>
		</VCard>
	</VContainer>
</template>

<script setup lang="ts">
	import {
		computed, onMounted, onUnmounted, ref,
	} from "vue";
	import { useI18n } from "vue-i18n";
	import {
		listen, type UnlistenFn,
	} from "@tauri-apps/api/event";
	import {
		loadPublicKey, type NetworkInfo, networkInfo, refreshHostState, startHost, stopHost, store,
	} from "../store";

	const { t } = useI18n();
	const copied = ref(false);
	const busy = ref(false);
	const net = ref<NetworkInfo | null>(null);

	let unlistenConnected: UnlistenFn | null = null;
	let unlistenDisconnected: UnlistenFn | null = null;
	let unlistenStopped: UnlistenFn | null = null;
	let netTimer: ReturnType<typeof setInterval> | undefined;

	async function refreshNet() {
		try {
			net.value = await networkInfo();
		} catch {
			// Best-effort; the page works without it.
		}
	}

	const statusKey = computed(() => {
		if (!store.hosting) {
			return "device.statusIdle";
		}
		return store.hostPeer ? "device.statusConnected" : "device.statusWaiting";
	});

	const statusColor = computed(() => {
		if (!store.hosting) {
			return "grey";
		}
		return store.hostPeer ? "success" : "amber";
	});

	onMounted(async () => {
		if (!store.publicKey) {
			await loadPublicKey();
		}
		await refreshNet();
		// Re-check periodically so the shown network follows Wi-Fi switches.
		netTimer = setInterval(refreshNet, 5000);
		await refreshHostState();
		unlistenConnected = await listen<string>("host://connected", (e) => {
			store.hostPeer = e.payload;
		});
		unlistenDisconnected = await listen("host://disconnected", () => {
			store.hostPeer = null;
		});
		unlistenStopped = await listen("host://stopped", () => {
			store.hosting = false;
			store.hostPin = "";
			store.hostPeer = null;
		});
	});

	onUnmounted(() => {
		unlistenConnected?.();
		unlistenDisconnected?.();
		unlistenStopped?.();
		if (netTimer) {
			clearInterval(netTimer);
		}
	});

	async function onStart() {
		busy.value = true;
		try {
			await startHost();
		} finally {
			busy.value = false;
		}
	}

	async function onStop() {
		busy.value = true;
		try {
			await stopHost();
		} finally {
			busy.value = false;
		}
	}

	async function copyCode() {
		await copyText(store.hostPin);
	}

	async function copyKey() {
		await copyText(store.publicKey);
	}

	async function copyText(value: string) {
		try {
			await navigator.clipboard.writeText(value);
			copied.value = true;
			setTimeout(() => (copied.value = false), 1500);
		} catch {
			// Clipboard may be unavailable; fields stay selectable as a fallback.
		}
	}
</script>

<style scoped>
	.code-display {
		font-family: monospace;
		font-size: 2rem;
		font-weight: 700;
		letter-spacing: 0.4rem;
	}
</style>
