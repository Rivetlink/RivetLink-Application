<template>
	<div class="rl-page">
		<header>
			<h1 class="rl-h1">
				{{ t("device.receiveHelpTitle") }}
			</h1>
			<p class="rl-lead">
				{{ t("device.receiveHelpIntro") }}
			</p>
		</header>

		<div class="layout">
			<div class="main">
				<section v-if="store.hosting" class="panel" :class="{ live: store.hostPeer }">
					<span class="rl-status" :class="store.hostPeer ? 'ok' : 'info'">
						{{ t(statusKey) }}
					</span>

					<!-- Waiting: the code and how to use it -->
					<template v-if="!store.hostPeer">
						<div>
							<h2 class="rl-section-title">
								{{ t("device.code") }}
							</h2>
							<div class="d-flex flex-wrap align-center ga-4">
								<span class="code-display">{{ pinDisplay }}</span>
								<VBtn
									variant="outlined"
									:prepend-icon="copied === 'pin' ? 'mdi-check' : 'mdi-content-copy'"
									@click="copyCode"
								>
									{{ copied === "pin" ? t("device.copied") : t("device.copy") }}
								</VBtn>
							</div>
						</div>
						<ol class="steps">
							<li>
								<span class="step-no">1</span>
								<span>{{ t("device.step1") }} <strong>{{ store.settings.device_name || t("app.unnamedDevice") }}</strong></span>
							</li>
							<li>
								<span class="step-no">2</span>
								<span>{{ t("device.step2") }}</span>
							</li>
							<li>
								<span class="step-no">3</span>
								<span>{{ t("device.step3") }}</span>
							</li>
						</ol>
					</template>

					<!-- A helper is connected -->
					<template v-else>
						<div class="d-flex flex-wrap align-center ga-4">
							<div class="avatar">
								{{ initials }}
							</div>
							<div class="flex-grow-1">
								<div class="peer-name">
									{{ store.hostPeer }}
								</div>
								<div class="rl-row-sub">
									{{ t("device.connectedHint", { name: store.hostPeer }) }}
								</div>
								<div v-if="store.hostClientTrusted" class="rl-row-sub d-flex align-center ga-1 mt-1">
									<VIcon icon="mdi-shield-check-outline" size="small" color="success" />
									{{ t("device.helperRemembered") }}
								</div>
							</div>
							<VBtn
								color="error"
								variant="tonal"
								:loading="disconnecting"
								@click="onDisconnect"
							>
								{{ t("device.disconnect") }}
							</VBtn>
						</div>
						<!-- Trust-on-connect: offer to remember an as-yet-untrusted
						     device that proved its identity, so it can reconnect
						     without the code. -->
						<div v-if="store.hostClientKey && !store.hostClientTrusted" class="remember">
							<div class="flex-grow-1">
								<div class="rl-row-title">
									{{ t("device.rememberTitle", { name: store.hostPeer }) }}
								</div>
								<div class="rl-row-sub">
									{{ t("device.rememberHint") }}
									<span class="rl-mono">{{ fingerprint }}</span>
								</div>
							</div>
							<VBtn
								variant="outlined"
								prepend-icon="mdi-check"
								:loading="remembering"
								@click="onRemember"
							>
								{{ t("device.remember") }}
							</VBtn>
						</div>
					</template>
				</section>

				<section v-else class="panel">
					<VAlert type="warning" variant="tonal">
						{{ t("device.unavailable") }}
					</VAlert>
					<div>
						<VBtn
							color="primary"
							prepend-icon="mdi-refresh"
							:loading="busy"
							@click="onStart"
						>
							{{ t("device.retry") }}
						</VBtn>
					</div>
				</section>
			</div>

			<div class="side">
				<section class="rl-group">
					<div class="rl-row">
						<div class="rl-row-text">
							<span class="rl-row-title">{{ t("access.title") }}</span>
						</div>
						<RouterLink to="/settings?s=security">
							{{ t("device.manage") }}
						</RouterLink>
					</div>
					<div v-if="store.settings.trusted_keys.length === 0" class="rl-row rl-row-sub">
						{{ t("access.empty") }}
					</div>
					<div v-for="k in store.settings.trusted_keys" :key="k.id" class="rl-row">
						<VIcon icon="mdi-shield-check-outline" color="success" size="18" />
						<span class="rl-row-text">{{ k.name || "—" }}</span>
						<span class="rl-mono rl-row-sub">{{ shortKey(k.public_key) }}</span>
					</div>
				</section>

				<section class="rl-group">
					<div class="rl-row">
						<div class="rl-row-text">
							<span class="rl-row-title">{{ t("device.identityTitle") }}</span>
							<span class="rl-row-sub">{{ t("device.identitySubtitle") }}</span>
							<span class="rl-mono key-text">{{ store.publicKey || "—" }}</span>
						</div>
						<VBtn
							variant="text"
							:icon="copied === 'key' ? 'mdi-check' : 'mdi-content-copy'"
							:aria-label="t('settings.copyKey')"
							:disabled="!store.publicKey"
							@click="copyKey"
						/>
					</div>
				</section>
			</div>
		</div>
	</div>
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
		hostDisconnect, loadPublicKey, refreshHostState, startHost, store, trustClient,
	} from "../store";

	type ClientIdentity = {
		key: string | null;
		name: string | null;
		trusted: boolean;
	};

	const { t } = useI18n();
	const copied = ref<"pin" | "key" | null>(null);
	const busy = ref(false);
	const disconnecting = ref(false);
	const remembering = ref(false);

	// Short, human-comparable fingerprint of the connected client's key.
	const fingerprint = computed(() => (store.hostClientKey ?? "").slice(0, 12));

	const unlistenConnected = ref<UnlistenFn | null>(null);
	const unlistenDisconnected = ref<UnlistenFn | null>(null);
	const unlistenStopped = ref<UnlistenFn | null>(null);
	const unlistenIdentity = ref<UnlistenFn | null>(null);

	const statusKey = computed(() => {
		if (!store.hosting) {
			return "device.statusIdle";
		}
		return store.hostPeer ? "device.statusConnected" : "device.statusWaiting";
	});

	// "482913" reads easier as "482 913".
	const pinDisplay = computed(() => store.hostPin.replace(/(\d{3})(?=\d)/g, "$1 "));
	const initials = computed(() => (store.hostPeer ?? "?")
		.split(/\s+/)
		.map((w) => w[0])
		.join("")
		.slice(0, 2)
		.toUpperCase());

	function shortKey(key: string): string {
		const k = key.replace(/^ed25519:/, "");
		return k.length > 10 ? `${k.slice(0, 4)}…${k.slice(-4)}` : k;
	}

	onMounted(async () => {
		if (!store.publicKey) {
			await loadPublicKey();
		}
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
		unlistenConnected.value = await listen<string>("host://connected", (e) => {
			store.hostPeer = e.payload;
		});
		unlistenDisconnected.value = await listen("host://disconnected", () => {
			store.hostPeer = null;
			store.hostClientKey = null;
			store.hostClientTrusted = false;
		});
		// The connected client's verified identity (for the "remember" prompt).
		unlistenIdentity.value = await listen<ClientIdentity>("host://client-identity", (e) => {
			store.hostClientKey = e.payload.key;
			store.hostClientTrusted = e.payload.trusted;
		});
		unlistenStopped.value = await listen("host://stopped", () => {
			store.hosting = false;
			store.hostPin = "";
			store.hostPeer = null;
		});
	});

	onUnmounted(() => {
		unlistenConnected.value?.();
		unlistenDisconnected.value?.();
		unlistenStopped.value?.();
		unlistenIdentity.value?.();
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
		await copyText(store.hostPin, "pin");
	}

	async function copyKey() {
		await copyText(store.publicKey, "key");
	}

	async function copyText(value: string, what: "pin" | "key") {
		try {
			await navigator.clipboard.writeText(value);
			copied.value = what;
			setTimeout(() => (copied.value = null), 1500);
		} catch {
			// Clipboard may be unavailable; fields stay selectable as a fallback.
		}
	}
</script>

<style scoped>
	.layout {
		display: flex;
		flex-wrap: wrap;
		gap: 20px;
		align-items: flex-start;
	}

	.main {
		flex: 3 1 420px;
		min-width: 0;
	}

	.side {
		flex: 2 1 300px;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.panel {
		padding: 28px;
		border-radius: 16px;
		background: rgb(var(--v-theme-surface));
		border: 1px solid var(--rl-border);
		display: flex;
		flex-direction: column;
		gap: 22px;
	}

	.panel.live {
		border-color: rgba(63, 208, 138, 0.35);
	}

	.panel .rl-status {
		font-size: 14px;
		font-weight: 600;
	}

	.code-display {
		font-family: var(--rl-mono);
		font-size: 52px;
		font-weight: 700;
		letter-spacing: 0.08em;
		line-height: 1;
	}

	.steps {
		margin: 0;
		padding: 0;
		list-style: none;
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
		gap: 12px;
	}

	.steps li {
		padding: 14px;
		font-size: 14px;
		border-radius: 10px;
		background: var(--rl-sidebar);
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.step-no {
		font-family: var(--rl-mono);
		font-size: 12px;
		color: var(--rl-accent);
	}

	.avatar {
		width: 52px;
		height: 52px;
		border-radius: 12px;
		background: var(--rl-raised);
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
		font-size: 18px;
	}

	.peer-name {
		font-size: 18px;
		font-weight: 600;
	}

	.remember {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 12px;
		padding: 14px 16px;
		border-radius: 10px;
		background: var(--rl-sidebar);
	}

	.key-text {
		color: var(--rl-soft);
		overflow-wrap: anywhere;
	}
</style>
