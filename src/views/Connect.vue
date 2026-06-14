<script setup lang="ts">
import {
	onMounted, ref,
} from "vue";
import { useI18n } from "vue-i18n";
import {
	activeRelay,
	captureScreenshot,
	connect,
	listDevices,
	login,
	store,
	type Device,
} from "../store";

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

function fail(e: unknown) {
	error.value = typeof e === "string" ? e : String(e);
}

function deviceMeta(d: Device): string {
	return t("connect.deviceMeta", {
		platform: d.platform || t("connect.unknown"),
		seen: d.last_seen || t("connect.never"),
	});
}

onMounted(() => {
	if (activeRelay() && !store.connected) {
		doConnect();
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
	if (!selected.value) return;
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
</script>

<template>
	<VContainer style="max-width: 880px">
		<!-- No relay yet -->
		<VAlert
			v-if="!activeRelay()"
			type="info"
			variant="tonal"
			class="mb-4"
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

			<VTabs v-model="tab" class="mb-4">
				<VTab value="devices" prepend-icon="mdi-monitor">
					{{ t("connect.tabDevices") }}
				</VTab>
				<VTab value="code" prepend-icon="mdi-numeric">
					{{ t("connect.tabCode") }}
				</VTab>
			</VTabs>

			<VWindow v-model="tab">
				<!-- Managed devices -->
				<VWindowItem value="devices">
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
		</template>

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
