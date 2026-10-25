<template>
	<div class="rl-page">
		<header class="rl-header">
			<div>
				<h1 class="rl-h1">
					{{ t("nav.connect") }}
				</h1>
				<p class="rl-lead">
					{{ t("connect.summary", { saved: savedCards.length, online: onlineCount }) }}
					{{ t("connect.autoRoute") }}
				</p>
			</div>
			<div class="d-flex flex-wrap align-center ga-2">
				<VTextField
					v-model="query"
					:placeholder="t('connect.search')"
					:aria-label="t('connect.search')"
					prepend-inner-icon="mdi-magnify"
					density="compact"
					hide-details
					clearable
					class="search"
				/>
				<VBtn
					variant="outlined"
					prepend-icon="mdi-wifi"
					:loading="scanning"
					@click="refreshAll"
				>
					{{ t("connect.lanScan") }}
				</VBtn>
			</div>
		</header>

		<!-- Relay: not set up, or not signed in -->
		<div v-if="!activeRelay()" class="rl-row-sub">
			{{ t("connect.noRelayBefore") }}
			<RouterLink to="/settings?s=servers">
				{{ t("connect.noRelayLink") }}
			</RouterLink>{{ t("connect.noRelayAfter") }}
		</div>
		<section v-else-if="!store.loggedIn" class="rl-group pa-4">
			<p class="mt-0 mb-3">
				{{ t("connect.signInRelay", { name: activeRelay()?.name }) }}
			</p>
			<div class="d-flex flex-wrap ga-2 align-center">
				<VTextField
					v-model="email"
					:label="t('connect.email')"
					density="compact"
					hide-details
					class="login-field"
				/>
				<VTextField
					v-model="password"
					:label="t('connect.password')"
					type="password"
					density="compact"
					hide-details
					class="login-field"
					@keyup.enter="doLogin"
				/>
				<VBtn
					color="primary"
					:loading="busy"
					:disabled="!store.connected"
					@click="doLogin"
				>
					{{ t("connect.signIn") }}
				</VBtn>
			</div>
		</section>

		<!-- Saved hosts -->
		<section v-if="savedCards.length > 0">
			<h2 class="rl-section-title">
				{{ t("connect.lanSaved") }}
			</h2>
			<div class="cards">
				<article v-for="c in savedCards" :key="c.device.id" class="card">
					<div class="thumb">
						<VIcon
							:icon="c.device.physical_console ? 'mdi-monitor-lock' : 'mdi-monitor'"
							size="56"
							:class="{ dim: !c.online }"
						/>
						<span class="pill rl-status" :class="c.online ? 'ok' : ''">
							{{ c.online ? t("connect.lanOnline") : t("connect.lanOffline") }}
						</span>
					</div>
					<div class="card-body">
						<div>
							<h3 class="card-title">
								{{ c.device.name }}
							</h3>
							<div class="rl-row-sub rl-mono">
								{{ c.device.address }}:{{ c.device.port }}
							</div>
						</div>
						<div class="d-flex flex-wrap ga-2">
							<span v-if="c.route === 'lan'" class="tag lan">{{ t("connect.routeLan") }}</span>
							<span v-else-if="c.route === 'relay'" class="tag relay">{{ t("connect.routeRelay") }}</span>
							<span v-if="c.device.physical_console" class="tag">{{ t("connect.loginScreen") }}</span>
							<span v-if="store.connectedLanId === c.device.id" class="tag ok">{{ t("connect.lanConnected") }}</span>
						</div>
						<div class="d-flex ga-2 align-center card-actions">
							<template v-if="connectingId === c.device.id">
								<VBtn
									color="primary"
									loading
									disabled
									class="flex-grow-1"
								>
									{{ t("connect.lanConnecting") }}
								</VBtn>
								<VBtn color="error" variant="tonal" @click="onLanDisconnect">
									{{ t("connect.lanDisconnect") }}
								</VBtn>
							</template>
							<VBtn
								v-else-if="store.connectedLanId === c.device.id"
								color="error"
								variant="tonal"
								class="flex-grow-1"
								@click="onLanDisconnect"
							>
								{{ t("connect.lanDisconnect") }}
							</VBtn>
							<VBtn
								v-else
								:color="c.online ? 'primary' : undefined"
								:variant="c.online ? 'flat' : 'outlined'"
								:loading="lanCaptureId === c.device.id"
								class="flex-grow-1"
								@click="c.device.physical_console ? smartConnect(c.device) : openConnect(c.device)"
							>
								{{ t("connect.lanConnect") }}
							</VBtn>
							<VMenu location="bottom end">
								<template #activator="{ props: menu }">
									<VBtn
										v-bind="menu"
										variant="outlined"
										icon="mdi-dots-horizontal"
										size="40"
										rounded="lg"
										:aria-label="t('connect.more', { name: c.device.name })"
									/>
								</template>
								<VList density="compact">
									<VListItem
										v-if="!c.device.physical_console"
										prepend-icon="mdi-camera-outline"
										:title="t('connect.capture')"
										@click="captureLan(c.device)"
									/>
									<VListItem
										prepend-icon="mdi-delete-outline"
										:title="t('connect.forget')"
										@click="forget(c.device.id)"
									/>
								</VList>
							</VMenu>
						</div>
					</div>
				</article>
			</div>
		</section>

		<!-- Relay devices that are not saved here -->
		<section v-if="relayOnly.length > 0">
			<h2 class="rl-section-title">
				{{ t("connect.viaRelayTitle") }}
			</h2>
			<div class="rl-group">
				<div v-for="d in relayOnly" :key="d.id" class="rl-row">
					<VIcon icon="mdi-monitor" color="medium-emphasis" />
					<div class="rl-row-text">
						<span class="rl-row-title">{{ d.hostname || d.id }}</span>
						<span class="rl-row-sub">{{ deviceMeta(d) }}</span>
					</div>
					<span class="tag relay">{{ t("connect.routeRelay") }}</span>
					<VBtn
						variant="outlined"
						prepend-icon="mdi-camera-outline"
						:disabled="!d.online"
						:loading="selected === d.id && !!busyMsg"
						@click="captureRelay(d.id)"
					>
						{{ t("connect.capture") }}
					</VBtn>
				</div>
			</div>
		</section>

		<!-- Found by a scan, not saved yet -->
		<section v-if="lanUnsaved.length > 0">
			<h2 class="rl-section-title">
				{{ t("connect.lanFound") }}
			</h2>
			<div class="rl-group">
				<div v-for="d in lanUnsaved" :key="`${d.address}:${d.port}`" class="rl-row">
					<VIcon :icon="d.physical_console ? 'mdi-monitor-lock' : 'mdi-monitor'" color="medium-emphasis" />
					<div class="rl-row-text">
						<span class="rl-row-title">{{ d.name }}</span>
						<span class="rl-row-sub rl-mono">{{ d.address }}:{{ d.port }}</span>
					</div>
					<VBtn variant="outlined" prepend-icon="mdi-plus" @click="remember(d)">
						{{ t("connect.lanAdd") }}
					</VBtn>
				</div>
			</div>
		</section>

		<div
			v-if="!scanning && lanUnsaved.length === 0 && store.settings.lan_devices.length === 0 && relayOnly.length === 0"
			class="rl-group rl-row rl-row-sub"
		>
			{{ t("connect.lanNoneFound") }}
		</div>

		<!-- One-shot screenshots and the legacy console view -->
		<section v-if="screenshot" class="rl-group pa-4">
			<div class="d-flex flex-wrap align-center ga-2 mb-2">
				<h2 class="rl-h2 mr-auto">
					{{ t("connect.screenshot") }}
				</h2>
				<span v-if="consoleState" class="tag">{{ consoleStateLabel }}</span>
				<span class="tag relay">{{ t("connect.routeRelay") }}</span>
			</div>
			<p class="rl-row-sub mt-0">
				{{ t("connect.consoleInputHint") }}
			</p>
			<VImg
				:src="screenshot"
				tabindex="0"
				class="console-screen"
				@contextmenu.prevent
				@click="consoleClick"
				@keydown.prevent="consoleKey($event, true)"
				@keyup.prevent="consoleKey($event, false)"
			/>
		</section>
		<section v-if="lanScreenshotImage" class="rl-group pa-4">
			<h2 class="rl-h2 mb-2">
				{{ t("connect.screenshot") }}
			</h2>
			<VImg :src="lanScreenshotImage" />
		</section>
		<section v-if="lanConsoleImage" class="rl-group pa-4">
			<div class="d-flex flex-wrap align-center ga-2 mb-2">
				<h2 class="rl-h2 mr-auto">
					{{ t("connect.loginScreen") }}
				</h2>
				<span v-if="lanConsoleState" class="tag">{{ consoleStateText(lanConsoleState) }}</span>
				<span class="tag lan">{{ t("connect.routeLan") }}</span>
			</div>
			<p class="rl-row-sub mt-0">
				{{ t("connect.consoleInputHint") }}
			</p>
			<VImg
				:src="lanConsoleImage"
				tabindex="0"
				class="console-screen"
				@contextmenu.prevent
				@click="lanConsoleClick"
				@keydown.prevent="lanConsoleKey($event, true)"
				@keyup.prevent="lanConsoleKey($event, false)"
			/>
		</section>

		<LanConnectModal
			v-model="connectOpen"
			:target="connectTarget"
			@connect="onLanConnect"
		/>

		<!-- Feedback -->
		<VSnackbar :model-value="!!busyMsg" color="surface-bright" timeout="-1">
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
	</div>
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
		consoleInputAndCapture,
		connect,
		discoverLan,
		type Device,
		type LanDevice,
		lanConnect,
		lanDisconnect,
		lanPing,
		lanScreenshot,
		lanConsoleCapture,
		lanConsoleAutoConnect,
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

	const query = ref<string | null>("");

	const email = ref("");
	const password = ref("");

	const devices = ref<Device[]>([]);
	const selected = ref<string | null>(null);
	const screenshot = ref<string | null>(null);
	const consoleState = ref<string | null>(null);
	const consoleStateLabel = computed(() => {
		const labels: Record<string, string> = {
			GdmLogin: "connect.consoleStateLogin",
			DesktopReady: "connect.consoleStateDesktop",
			SessionLocked: "connect.consoleStateLocked",
			SessionStarting: "connect.consoleStateStarting",
			SessionSwitching: "connect.consoleStateSwitching",
			Booting: "connect.consoleStateBooting",
			Offline: "connect.consoleStateOffline",
		};
		return consoleState.value ? t(labels[consoleState.value] || "connect.consoleStateBooting") : "";
	});

	const busy = ref(false);
	const busyMsg = ref<string | null>(null);
	const error = ref<string | null>(null);

	// --- LAN state ---
	const scanning = ref(false);
	const lanFound = ref<LanDevice[]>([]);
	const connectOpen = ref(false);
	const connectTarget = ref<SavedLanDevice | null>(null);
	// Id of the saved device a connection is currently being set up for, so its
	// row can show "Connecting…" and disable its button.
	const connectingId = ref<string | null>(null);
	const lanCaptureId = ref<string | null>(null);
	const lanScreenshotImage = ref<string | null>(null);
	const lanConsoleImage = ref<string | null>(null);
	const lanConsoleState = ref<string | null>(null);
	const lanConsoleTarget = ref<SavedLanDevice | null>(null);
	const net = ref<NetworkInfo | null>(null);
	// Per-saved-device reachability (id -> online), refreshed on a timer.
	const online = ref<Record<string, boolean>>({});

	// Hosts found by a scan that aren't already remembered. A host identity is
	// authoritative: the same IP:port may legitimately move from the ordinary
	// user-session host to the boot-time physical-console broker, whose key is
	// intentionally different. Do not hide that key transition behind a stale
	// saved entry — show it so the owner can explicitly replace the old entry.
	// Only keyless legacy advertisements fall back to address:port deduping.
	const lanUnsaved = computed(() =>
		lanFound.value.filter(
			(f) => !store.settings.lan_devices.some(
				(s) => (f.public_key && s.public_key)
					? s.public_key === f.public_key
					: s.address === f.address && s.port === f.port,
			),
		),
	);

	// One card per saved host. A host is matched to its relay record by public
	// key (as smartConnect does), so the card can say which route is up.
	const savedCards = computed(() => {
		const q = (query.value ?? "").trim().toLowerCase();
		return store.settings.lan_devices
			.filter((d) => !q || d.name.toLowerCase().includes(q))
			.map((device) => {
				const lan = Boolean(online.value[device.id]);
				const relay = !lan && devices.value.some((r) =>
					r.online && device.public_key && r.public_key === device.public_key);
				return {
					device,
					online: lan || relay,
					route: lan ? "lan" : relay ? "relay" : null,
				};
			});
	});
	const onlineCount = computed(() => savedCards.value.filter((c) => c.online).length);

	// Relay devices with no saved host of the same key: the legacy screenshot
	// relay is their only route.
	const relayOnly = computed(() => {
		const q = (query.value ?? "").trim().toLowerCase();
		return devices.value.filter((r) =>
			!store.settings.lan_devices.some((s) => s.public_key && s.public_key === r.public_key)
			&& (!q || (r.hostname ?? r.id).toLowerCase().includes(q)));
	});

	function captureRelay(id: string) {
		selected.value = id;
		void capture();
	}

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
		// A key-authenticated host that doesn't list this device's identity.
		if (m.includes("host rejected connection")) {
			return t("connect.errKeyRejected");
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
		const platform = d.platform || t("connect.unknown");
		return d.online
			? t("connect.deviceMetaOnline", { platform })
			: t("connect.deviceMetaOffline", {
				platform,
				seen: d.last_seen || t("connect.never"),
			});
	}

	const netTimer = ref<ReturnType<typeof setInterval>>();

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
		if (store.loggedIn) {
			void refresh();
		}
		await refreshNet();
		await pingDevices();
		// Re-check periodically so the network + device status stay current.
		netTimer.value = setInterval(() => {
			refreshNet();
			pingDevices();
		}, 5000);
	});

	onUnmounted(() => {
		if (netTimer.value) {
			clearInterval(netTimer.value);
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
			const capture = await captureScreenshot(selected.value);
			screenshot.value = capture.image;
			consoleState.value = capture.consoleState;
		} catch (e) {
			fail(e);
		} finally {
			busyMsg.value = null;
		}
	}

	async function sendConsoleInput(event: Parameters<typeof consoleInputAndCapture>[1]) {
		if (!selected.value || busyMsg.value) {return;}
		error.value = null;
		busyMsg.value = t("connect.requestingSession");
		try {
			const capture = await consoleInputAndCapture(selected.value, event);
			screenshot.value = capture.image;
			consoleState.value = capture.consoleState;
		} catch (e) {
			fail(e);
		} finally {
			busyMsg.value = null;
		}
	}

	function consoleKey(event: KeyboardEvent, down: boolean) {
		// `code` is a physical key identifier. The key value (and therefore a
		// password character) is never inspected or recorded by RivetLink.
		void sendConsoleInput({
			type: "key",
			code: event.code,
			down,
		});
	}

	function consoleClick(event: MouseEvent) {
		const target = event.currentTarget as HTMLElement;
		const rect = target.getBoundingClientRect();
		if (!rect.width || !rect.height) {return;}
		const x = Math.max(0, Math.min(10000, Math.round((event.clientX - rect.left) / rect.width * 10000)));
		const y = Math.max(0, Math.min(10000, Math.round((event.clientY - rect.top) / rect.height * 10000)));
		void (async () => {
			await sendConsoleInput({
				type: "pointer_move",
				x,
				y,
			});
			await sendConsoleInput({
				type: "pointer_button",
				button: "left",
				down: true,
			});
			await sendConsoleInput({
				type: "pointer_button",
				button: "left",
				down: false,
			});
		})();
	}

	// --- LAN actions ---
	// The header button: look on the LAN, re-ping saved hosts and reload the
	// relay's device list in one go.
	async function refreshAll() {
		await Promise.all([
			scan(),
			pingDevices(),
			store.loggedIn ? refresh() : Promise.resolve(),
		]);
	}

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
			await addLanDevice(d.name, d.address, d.port, d.public_key, d.physical_console);
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

	async function captureLan(device: SavedLanDevice) {
		error.value = null;
		lanCaptureId.value = device.id;
		lanScreenshotImage.value = null;
		try {
			lanScreenshotImage.value = await lanScreenshot(
				device.address,
				device.port,
				null,
				device.public_key,
			);
		} catch (e) {
			fail(e);
		} finally {
			lanCaptureId.value = null;
		}
	}

	function consoleStateText(state: string | null) {
		return state ? t({
			GdmLogin: "connect.consoleStateLogin",
			DesktopReady: "connect.consoleStateDesktop",
			SessionLocked: "connect.consoleStateLocked",
			SessionStarting: "connect.consoleStateStarting",
			SessionSwitching: "connect.consoleStateSwitching",
			Booting: "connect.consoleStateBooting",
			Offline: "connect.consoleStateOffline",
		}[state] || "connect.consoleStateBooting") : "";
	}

	async function captureLanConsole(device: SavedLanDevice, inputs: Parameters<typeof lanConsoleCapture>[3] = []) {
		error.value = null;
		lanCaptureId.value = device.id;
		try {
			const capture = await lanConsoleCapture(device.address, device.port, device.public_key, inputs);
			lanConsoleTarget.value = device;
			lanConsoleImage.value = capture.image;
			lanConsoleState.value = capture.consoleState;
		} catch (e) { fail(e); } finally { lanCaptureId.value = null; }
	}

	// Prefer the exact advertised host key on the local network. Its boot-time
	// broker selects the normal desktop stream once GNOME is ready, otherwise
	// the LightDM physical-console backend. If it is not currently discoverable,
	// try the same public device identity through the authenticated relay.
	// Route selection never changes the identity pinned for LAN or relay.
	async function smartConnect(device: SavedLanDevice) {
		error.value = null;
		lanCaptureId.value = device.id;
		try {
			scanning.value = true;
			const discovered = await discoverLan();
			lanFound.value = discovered;
			const local = discovered.find((candidate) =>
				candidate.physical_console
				&& candidate.public_key === device.public_key,
			);
			if (local) {
				await lanConsoleAutoConnect({
					...device,
					address: local.address,
					port: local.port,
				});
				return;
			}

			if (activeRelay() && store.connected && store.loggedIn) {
				const relayDevices = await listDevices();
				const relayDevice = relayDevices.find((candidate) =>
					candidate.public_key === device.public_key,
				);
				if (relayDevice) {
					await lanConsoleAutoConnect(device, relayDevice.id);
					return;
				}
			}

			throw new Error(t("connect.errNoRoute"));
		} catch (e) {
			fail(e);
		} finally {
			scanning.value = false;
			lanCaptureId.value = null;
		}
	}

	function lanConsoleKey(event: KeyboardEvent, down: boolean) {
		// Only a physical browser code is forwarded. No character/password is read or logged.
		if (lanConsoleTarget.value) {void captureLanConsole(lanConsoleTarget.value, [{
			type: "key",
			code: event.code,
			down,
		}]);}
	}

	function lanConsoleClick(event: MouseEvent) {
		const target = event.currentTarget as HTMLElement;
		const rect = target.getBoundingClientRect();
		if (!rect.width || !rect.height || !lanConsoleTarget.value) {return;}
		const x = Math.max(0, Math.min(10000, Math.round((event.clientX - rect.left) / rect.width * 10000)));
		const y = Math.max(0, Math.min(10000, Math.round((event.clientY - rect.top) / rect.height * 10000)));
		void captureLanConsole(lanConsoleTarget.value, [
			{
				type: "pointer_move",
				x,
				y,
			},
			{
				type: "pointer_button",
				button: "left",
				down: true,
			},
			{
				type: "pointer_button",
				button: "left",
				down: false,
			},
		]);
	}
</script>

<style scoped>
	.search {
		width: 220px;
		flex: 0 1 220px;
	}

	.login-field {
		flex: 1 1 200px;
	}

	.cards {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: 16px;
	}

	.card {
		display: flex;
		flex-direction: column;
		border-radius: 14px;
		background: rgb(var(--v-theme-surface));
		border: 1px solid var(--rl-border);
		overflow: hidden;
	}

	.thumb {
		position: relative;
		height: 130px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #0B0D11;
		border-bottom: 1px solid var(--rl-border);
		color: var(--rl-soft);
	}

	.thumb .dim {
		opacity: 0.4;
	}

	.pill {
		position: absolute;
		top: 10px;
		left: 10px;
		height: 24px;
		padding: 0 9px;
		border-radius: 12px;
		background: rgba(15, 17, 21, 0.85);
		font-size: 12px;
	}

	.card-body {
		flex-grow: 1;
		padding: 14px 16px 16px;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.card-actions {
		margin-top: auto;
	}

	.card-title {
		margin: 0;
		font-size: 16px;
		font-weight: 600;
	}

	.tag {
		display: inline-flex;
		align-items: center;
		height: 24px;
		padding: 0 9px;
		border-radius: 6px;
		font-size: 12px;
		font-weight: 500;
		background: var(--rl-raised);
		color: var(--rl-soft);
		white-space: nowrap;
	}

	.tag.lan {
		background: rgba(142, 166, 255, 0.14);
		color: #B4C4FF;
	}

	.tag.relay {
		background: rgba(242, 176, 76, 0.14);
		color: #F2C27A;
	}

	.tag.ok {
		background: rgba(63, 208, 138, 0.14);
		color: #CFF5E2;
	}

	.console-screen {
		cursor: crosshair;
		border-radius: 8px;
	}
</style>
