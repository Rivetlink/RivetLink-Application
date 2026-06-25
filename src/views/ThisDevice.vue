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

				<template v-if="store.hosting">
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
						class="mb-0"
					>
						<div class="d-flex align-center ga-3">
							<div class="flex-grow-1">
								<div class="text-subtitle-2">
									{{ store.hostPeer ? t("device.connectedTitle") : t("device.waitingTitle") }}
								</div>
								<div class="text-caption">
									{{ store.hostPeer
										? t("device.connectedHint", { name: store.hostPeer })
										: t("device.waitingHint") }}
								</div>
							</div>
							<VBtn
								v-if="store.hostPeer"
								color="error"
								variant="tonal"
								size="small"
								prepend-icon="mdi-close-circle-outline"
								:loading="disconnecting"
								@click="onDisconnect"
							>
								{{ t("device.disconnect") }}
							</VBtn>
						</div>
					</VAlert>

					<!-- Trust-on-connect: offer to remember an as-yet-untrusted
					     device that proved its identity, so it can reconnect
					     without the code. -->
					<VAlert
						v-if="store.hostPeer && store.hostClientKey && !store.hostClientTrusted"
						type="info"
						variant="tonal"
						density="comfortable"
						class="mt-3 mb-0"
						icon="mdi-shield-key-outline"
					>
						<div class="d-flex align-center ga-3">
							<div class="flex-grow-1">
								<div class="text-subtitle-2">
									{{ t("device.rememberTitle", { name: store.hostPeer }) }}
								</div>
								<div class="text-caption">
									{{ t("device.rememberHint") }}
									<code>{{ fingerprint }}</code>
								</div>
							</div>
							<VBtn
								color="primary"
								variant="flat"
								size="small"
								prepend-icon="mdi-check"
								:loading="remembering"
								@click="onRemember"
							>
								{{ t("device.remember") }}
							</VBtn>
						</div>
					</VAlert>

					<VFadeTransition>
						<div
							v-if="store.hostPeer && store.hostClientTrusted"
							class="text-caption text-success mt-2 d-flex align-center ga-1"
						>
							<VIcon icon="mdi-shield-check" size="small" />
							{{ t("device.remembered") }}
						</div>
					</VFadeTransition>
				</template>

				<template v-else>
					<VAlert
						type="warning"
						variant="tonal"
						density="comfortable"
						class="mb-4"
					>
						{{ t("device.unavailable") }}
					</VAlert>
					<VBtn
						color="primary"
						variant="tonal"
						prepend-icon="mdi-refresh"
						:loading="busy"
						@click="onStart"
					>
						{{ t("device.retry") }}
					</VBtn>
				</template>
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
		hostDisconnect, loadPublicKey, type NetworkInfo, networkInfo,
		refreshHostState, startHost, store, trustClient,
	} from "../store";

	type ClientIdentity = {
		key: string | null;
		name: string | null;
		trusted: boolean;
	};

	const { t } = useI18n();
	const copied = ref(false);
	const busy = ref(false);
	const disconnecting = ref(false);
	const remembering = ref(false);
	const net = ref<NetworkInfo | null>(null);

	// Short, human-comparable fingerprint of the connected client's key.
	const fingerprint = computed(() => (store.hostClientKey ?? "").slice(0, 12));

	let unlistenConnected: UnlistenFn | null = null;
	let unlistenDisconnected: UnlistenFn | null = null;
	let unlistenStopped: UnlistenFn | null = null;
	let unlistenIdentity: UnlistenFn | null = null;
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
		// The host runs as a daemon: if it isn't up yet (e.g. first open on this
		// page), start it so the device is always reachable — no manual toggle.
		if (!store.hosting) {
			try {
				await startHost();
			} catch {
				// Backend unavailable (e.g. Windows) — the page shows a retry.
			}
		}
		unlistenConnected = await listen<string>("host://connected", (e) => {
			store.hostPeer = e.payload;
		});
		unlistenDisconnected = await listen("host://disconnected", () => {
			store.hostPeer = null;
			store.hostClientKey = null;
			store.hostClientTrusted = false;
		});
		// The connected client's verified identity (for the "remember" prompt).
		unlistenIdentity = await listen<ClientIdentity>("host://client-identity", (e) => {
			store.hostClientKey = e.payload.key;
			store.hostClientTrusted = e.payload.trusted;
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
		unlistenIdentity?.();
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

	async function onDisconnect() {
		disconnecting.value = true;
		try {
			await hostDisconnect();
		} finally {
			disconnecting.value = false;
		}
	}

	async function onRemember() {
		if (!store.hostPeer) {
			return;
		}
		remembering.value = true;
		try {
			await trustClient(store.hostPeer);
		} catch {
			// Nobody connected anymore, or already trusted — the prompt just hides.
		} finally {
			remembering.value = false;
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
