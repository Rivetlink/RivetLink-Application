<template>
	<VContainer style="max-width: 880px">
		<VTabs v-model="tab" class="mb-4">
			<VTab value="devices" prepend-icon="mdi-server-network">
				{{ t("connect.tabDevices") }}
			</VTab>
			<VTab value="lan" prepend-icon="mdi-lan">
				{{ t("connect.tabLan") }}
			</VTab>
			<VTab value="code" prepend-icon="mdi-numeric">
				{{ t("connect.tabCode") }}
			</VTab>
		</VTabs>

		<VWindow v-model="tab">
			<!-- Managed devices (via a relay) -->
			<VWindowItem value="devices">
				<VAlert
					v-if="!activeRelay()"
					type="info"
					variant="tonal"
				>
					{{ t("connect.noRelayBefore") }}
					<RouterLink to="/relays">
						{{ t("connect.noRelayLink") }}
					</RouterLink>{{ t("connect.noRelayAfter") }}
				</VAlert>

				<template v-else>
					<VCard variant="tonal" class="mb-4">
						<VCardText class="d-flex align-center">
							<VIcon icon="mdi-server-network" class="mr-2" />
							<div>
								<div class="text-body-1">
									{{ activeRelay()?.name }}
								</div>
								<div class="text-caption text-medium-emphasis">
									{{ activeRelay()?.http_url }}
								</div>
							</div>
							<VSpacer />
							<VChip
								:color="store.connected ? 'green' : 'grey'"
								size="small"
								variant="flat"
							>
								{{ store.connected ? t("connect.connected") : t("connect.notConnected") }}
							</VChip>
						</VCardText>
					</VCard>

					<!-- Sign in -->
					<VCard v-if="!store.loggedIn" variant="tonal">
						<VCardTitle>{{ t("connect.signInTitle") }}</VCardTitle>
						<VCardSubtitle>{{ t("connect.signInSubtitle") }}</VCardSubtitle>
						<VCardText>
							<VTextField
								v-model="email"
								:label="t('connect.email')"
								prepend-inner-icon="mdi-email-outline"
								density="comfortable"
							/>
							<VTextField
								v-model="password"
								:label="t('connect.password')"
								type="password"
								prepend-inner-icon="mdi-lock-outline"
								density="comfortable"
								hide-details
								@keyup.enter="doLogin"
							/>
						</VCardText>
						<VCardActions>
							<VSpacer />
							<VBtn
								color="primary"
								variant="flat"
								:loading="busy"
								:disabled="!store.connected"
								@click="doLogin"
							>
								{{ t("connect.signIn") }}
							</VBtn>
						</VCardActions>
					</VCard>

					<!-- Device list -->
					<template v-else>
						<VCard variant="tonal">
							<VCardTitle class="d-flex align-center">
								{{ t("connect.devices") }}
								<VSpacer />
								<VBtn
									size="small"
									variant="text"
									icon="mdi-refresh"
									:loading="busy"
									@click="refresh"
								/>
							</VCardTitle>
							<VCardText>
								<VAlert
									v-if="!busy && devices.length === 0"
									type="warning"
									variant="tonal"
									density="compact"
								>
									{{ t("connect.noDevices") }}
								</VAlert>
								<VList v-else lines="two" density="comfortable">
									<VListItem
										v-for="d in devices"
										:key="d.id"
										:active="selected === d.id"
										@click="selected = d.id"
									>
										<template #prepend>
											<VIcon icon="mdi-monitor" />
										</template>
										<VListItemTitle>{{ d.hostname || d.id }}</VListItemTitle>
										<VListItemSubtitle>{{ deviceMeta(d) }}</VListItemSubtitle>
										<template #append>
											<VIcon
												v-if="selected === d.id"
												icon="mdi-check-circle"
												color="primary"
											/>
										</template>
									</VListItem>
								</VList>
							</VCardText>
							<VCardActions>
								<VSpacer />
								<VBtn
									color="primary"
									variant="flat"
									prepend-icon="mdi-camera"
									:disabled="!selected"
									:loading="!!busyMsg"
									@click="capture"
								>
									{{ t("connect.capture") }}
								</VBtn>
							</VCardActions>
						</VCard>

						<VCard v-if="screenshot" class="mt-4">
							<VCardTitle>{{ t("connect.screenshot") }}</VCardTitle>
							<VImg :src="screenshot" />
						</VCard>
					</template>
				</template>
			</VWindowItem>

			<!-- Local network (direct, no relay) -->
			<VWindowItem value="lan">
				<VCard variant="tonal" class="mb-4">
					<VCardText class="d-flex align-center">
						<VIcon icon="mdi-lan" class="mr-3" />
						<div>
							<div class="text-body-1">
								{{ t("connect.lanTitle") }}
							</div>
							<div class="text-caption text-medium-emphasis">
								{{ t("connect.lanSubtitle") }}
							</div>
							<div v-if="net && (net.ssid || net.ip)" class="text-caption text-medium-emphasis mt-1">
								<VIcon :icon="net.ssid ? 'mdi-wifi' : 'mdi-lan'" size="x-small" class="mr-1" />
								<span v-if="net.ssid">{{ t("connect.lanNetworkWifi", { ssid: net.ssid }) }} · </span>
								<span v-if="net.ip">{{ t("connect.lanNetworkIp", { ip: net.ip }) }}</span>
							</div>
						</div>
						<VSpacer />
						<VBtn
							color="primary"
							variant="flat"
							prepend-icon="mdi-magnify"
							:loading="scanning"
							@click="scan"
						>
							{{ t("connect.lanScan") }}
						</VBtn>
					</VCardText>
				</VCard>

				<!-- Newly found, not yet remembered -->
				<VCard v-if="lanUnsaved.length > 0" variant="tonal" class="mb-4">
					<VCardTitle>{{ t("connect.lanFound") }}</VCardTitle>
					<VList class="bg-transparent" lines="two" density="comfortable">
						<VListItem v-for="d in lanUnsaved" :key="`${d.address}:${d.port}`">
							<template #prepend>
								<VIcon icon="mdi-monitor-share" />
							</template>
							<VListItemTitle>{{ d.name }}</VListItemTitle>
							<VListItemSubtitle>{{ d.address }}:{{ d.port }}</VListItemSubtitle>
							<template #append>
								<VBtn
									size="small"
									variant="tonal"
									prepend-icon="mdi-plus"
									@click="remember(d)"
								>
									{{ t("connect.lanAdd") }}
								</VBtn>
							</template>
						</VListItem>
					</VList>
				</VCard>

				<VAlert
					v-else-if="!scanning && lanFound.length === 0 && store.settings.lan_devices.length === 0"
					type="info"
					variant="tonal"
					class="mb-4"
				>
					{{ t("connect.lanNoneFound") }}
				</VAlert>

				<!-- Remembered hosts -->
				<VCard v-if="store.settings.lan_devices.length > 0" variant="tonal">
					<VCardTitle>{{ t("connect.lanSaved") }}</VCardTitle>
					<VList class="bg-transparent" lines="two" density="comfortable">
						<VListItem v-for="d in store.settings.lan_devices" :key="d.id">
							<template #prepend>
								<VIcon icon="mdi-monitor" />
							</template>
							<VListItemTitle>
								{{ d.name }}
								<VChip
									v-if="store.connectedLanId === d.id"
									size="x-small"
									color="green"
									variant="flat"
									class="ml-2"
								>
									{{ t("connect.lanConnected") }}
								</VChip>
							</VListItemTitle>
							<VListItemSubtitle>
								<VIcon
									icon="mdi-circle"
									:color="online[d.id] ? 'green' : 'grey'"
									size="8"
									class="mr-1"
								/>
								<span>{{ online[d.id] ? t("connect.lanOnline") : t("connect.lanOffline") }} · {{ d.address }}:{{ d.port }}</span>
							</VListItemSubtitle>
							<template #append>
								<!-- Setting up a connection: progress + a cancel -->
								<template v-if="connectingId === d.id">
									<VBtn
										size="small"
										color="primary"
										variant="flat"
										loading
										disabled
										class="mr-2"
									>
										{{ t("connect.lanConnecting") }}
									</VBtn>
									<VBtn
										size="small"
										color="error"
										variant="tonal"
										prepend-icon="mdi-close"
										class="mr-2"
										@click="onLanDisconnect"
									>
										{{ t("connect.lanDisconnect") }}
									</VBtn>
								</template>
								<VBtn
									v-else-if="store.connectedLanId === d.id"
									size="small"
									color="error"
									variant="tonal"
									prepend-icon="mdi-close"
									class="mr-2"
									@click="onLanDisconnect"
								>
									{{ t("connect.lanDisconnect") }}
								</VBtn>
								<VBtn
									v-else
									size="small"
									color="primary"
									variant="flat"
									prepend-icon="mdi-connection"
									class="mr-2"
									@click="openConnect(d)"
								>
									{{ t("connect.lanConnect") }}
								</VBtn>
								<VBtn
									size="small"
									variant="text"
									icon="mdi-delete-outline"
									@click="forget(d.id)"
								/>
							</template>
						</VListItem>
					</VList>
				</VCard>
			</VWindowItem>

			<!-- Session code (planned) -->
			<VWindowItem value="code">
				<VCard variant="tonal">
					<VCardTitle class="d-flex align-center">
						{{ t("connect.sessionCodeTitle") }}
						<VChip class="ml-2" size="x-small" color="amber">
							{{ t("common.soon") }}
						</VChip>
					</VCardTitle>
					<VCardSubtitle>{{ t("connect.sessionCodeSubtitle") }}</VCardSubtitle>
					<VCardText>
						<VOtpInput v-model="sessionCode" length="9" disabled />
						<p class="text-caption text-medium-emphasis">
							{{ t("connect.sessionCodeNote") }}
						</p>
					</VCardText>
				</VCard>
			</VWindowItem>
		</VWindow>

		<LanConnectModal
			v-model="connectOpen"
			:target="connectTarget"
			@connect="onLanConnect"
		/>

		<!-- Feedback -->
		<VSnackbar :model-value="!!busyMsg" color="grey-darken-3" timeout="-1">
			<VProgressCircular indeterminate size="18" class="mr-2" /> {{ busyMsg }}
		</VSnackbar>
		<VSnackbar
			:model-value="!!error"
			color="error"
			timeout="6000"
			@update:model-value="error = null"
		>
			{{ error }}
		</VSnackbar>
	</VContainer>
</template>

<script setup lang="ts">
	import {
		computed, onMounted, onUnmounted, ref,
	} from "vue";
	import { useI18n } from "vue-i18n";
	import {
		activeRelay,
		addLanDevice,
		captureScreenshot,
		connect,
		discoverLan,
		type Device,
		type LanDevice,
		lanConnect,
		lanDisconnect,
		lanPing,
		listDevices,
		login,
		type NetworkInfo,
		networkInfo,
		removeLanDevice,
		type SavedLanDevice,
		store,
	} from "../store";
	import LanConnectModal from "../components/LanConnectModal.vue";

	const { t } = useI18n();

	const tab = ref("devices");

	const email = ref("");
	const password = ref("");

	const devices = ref<Device[]>([]);
	const selected = ref<string | null>(null);
	const screenshot = ref<string | null>(null);

	const busy = ref(false);
	const busyMsg = ref<string | null>(null);
	const error = ref<string | null>(null);

	const sessionCode = ref("");

	// --- LAN state ---
	const scanning = ref(false);
	const lanFound = ref<LanDevice[]>([]);
	const connectOpen = ref(false);
	const connectTarget = ref<SavedLanDevice | null>(null);
	// Id of the saved device a connection is currently being set up for, so its
	// row can show "Connecting…" and disable its button.
	const connectingId = ref<string | null>(null);
	const net = ref<NetworkInfo | null>(null);
	// Per-saved-device reachability (id -> online), refreshed on a timer.
	const online = ref<Record<string, boolean>>({});

	// Hosts found by a scan that aren't already remembered. Match on identity
	// key first (the same machine can resolve on a different address between
	// scans — e.g. IPv4 one time, a routable IPv6 the next — and must not show
	// twice), then fall back to address:port for hosts that advertise no key.
	const lanUnsaved = computed(() =>
		lanFound.value.filter(
			(f) => !store.settings.lan_devices.some(
				(s) => (f.public_key && s.public_key && s.public_key === f.public_key)
					|| (s.address === f.address && s.port === f.port),
			),
		),
	);

	// Map a raw backend error string to a friendly, translated message. Unknown
	// errors fall through unchanged so nothing is silently swallowed.
	function friendlyError(raw: string): string {
		const m = raw.toLowerCase();
		if (m.includes("refused") || m.includes("os error 61") || m.includes("os error 111")) {
			return t("connect.errUnreachable");
		}
		if (m.includes("timed out") || m.includes("timeout") || m.includes("deadline")) {
			return t("connect.errTimeout");
		}
		if (m.includes("unreachable") || m.includes("no route") || m.includes("os error 65") || m.includes("os error 113")) {
			return t("connect.errUnreachable");
		}
		if (m.includes("not trusted") || m.includes("trust")) {
			return t("connect.errNotTrusted");
		}
		if (
			m.includes("handshake") || m.includes("decrypt") || m.includes("password")
			|| m.includes("pin") || m.includes("spake") || m.includes("signature")
			|| m.includes("reject")
		) {
			return t("connect.errAuth");
		}
		return raw;
	}

	function fail(e: unknown) {
		error.value = friendlyError(typeof e === "string" ? e : String(e));
	}

	function deviceMeta(d: Device): string {
		return t("connect.deviceMeta", {
			platform: d.platform || t("connect.unknown"),
			seen: d.last_seen || t("connect.never"),
		});
	}

	let netTimer: ReturnType<typeof setInterval> | undefined;

	async function refreshNet() {
		try {
			net.value = await networkInfo();
		} catch {
			// Network info is best-effort; the tab works without it.
		}
	}

	async function pingDevices() {
		const devices = store.settings.lan_devices;
		const results = await Promise.all(
			devices.map(async (d) => [d.id, await lanPing(d.address, d.port)] as const),
		);
		online.value = Object.fromEntries(results);
	}

	onMounted(async () => {
		if (activeRelay() && !store.connected) {
			doConnect();
		}
		await refreshNet();
		await pingDevices();
		// Re-check periodically so the network + device status stay current.
		netTimer = setInterval(() => {
			refreshNet();
			pingDevices();
		}, 5000);
	});

	onUnmounted(() => {
		if (netTimer) {
			clearInterval(netTimer);
		}
	});

	async function doConnect() {
		error.value = null;
		busy.value = true;
		try {
			await connect();
		} catch (e) {
			fail(e);
		} finally {
			busy.value = false;
		}
	}

	async function doLogin() {
		error.value = null;
		busy.value = true;
		try {
			await login(email.value, password.value);
			await refresh();
		} catch (e) {
			fail(e);
		} finally {
			busy.value = false;
		}
	}

	async function refresh() {
		error.value = null;
		busy.value = true;
		try {
			devices.value = await listDevices();
		} catch (e) {
			fail(e);
		} finally {
			busy.value = false;
		}
	}

	async function capture() {
		if (!selected.value) {return;}
		error.value = null;
		screenshot.value = null;
		busyMsg.value = t("connect.requestingSession");
		try {
			screenshot.value = await captureScreenshot(selected.value);
		} catch (e) {
			fail(e);
		} finally {
			busyMsg.value = null;
		}
	}

	// --- LAN actions ---
	async function scan() {
		error.value = null;
		scanning.value = true;
		try {
			lanFound.value = await discoverLan();
		} catch (e) {
			fail(e);
		} finally {
			scanning.value = false;
		}
	}

	async function remember(d: LanDevice) {
		error.value = null;
		try {
			await addLanDevice(d.name, d.address, d.port, d.public_key);
		} catch (e) {
			fail(e);
		}
	}

	async function forget(id: string) {
		error.value = null;
		try {
			await removeLanDevice(id);
		} catch (e) {
			fail(e);
		}
	}

	function openConnect(d: SavedLanDevice) {
		connectTarget.value = d;
		connectOpen.value = true;
	}

	async function onLanConnect(pin: string) {
		const target = connectTarget.value;
		if (!target) {return;}
		error.value = null;
		connectingId.value = target.id;
		try {
			await lanConnect(target, pin || null);
		} catch (e) {
			fail(e);
		} finally {
			connectingId.value = null;
		}
	}

	async function onLanDisconnect() {
		error.value = null;
		// Also clears any in-progress "Connecting…" so the row resets at once.
		connectingId.value = null;
		try {
			await lanDisconnect();
		} catch (e) {
			fail(e);
		}
	}
</script>
