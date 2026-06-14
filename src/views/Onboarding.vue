<template>
	<VContainer class="fill-height" style="max-width: 720px">
		<VCard class="w-100" variant="flat" color="transparent">
			<div class="text-center mb-6">
				<VIcon icon="mdi-shield-lock-outline" color="primary" size="48" />
				<h1 class="text-h4 mt-2">
					{{ t("onboarding.welcomeTitle") }}
				</h1>
				<p class="text-medium-emphasis">
					{{ t("onboarding.welcomeSubtitle") }}
				</p>
			</div>

			<VWindow v-model="step">
				<!-- Step 1: role -->
				<VWindowItem :value="1">
					<VCard variant="tonal">
						<VCardTitle>{{ t("onboarding.roleQuestion") }}</VCardTitle>
						<VCardSubtitle>{{ t("onboarding.roleHint") }}</VCardSubtitle>
						<VCardText>
							<VListItem
								class="rounded mb-2 border"
								:active="roleHost"
								@click="roleHost = !roleHost"
							>
								<template #prepend>
									<VCheckboxBtn :model-value="roleHost" />
								</template>
								<VListItemTitle>{{ t("onboarding.hostTitle") }}</VListItemTitle>
								<VListItemSubtitle class="text-wrap">
									{{ t("onboarding.hostSubtitle") }}
								</VListItemSubtitle>
							</VListItem>

							<VListItem
								class="rounded border"
								:active="roleClient"
								@click="roleClient = !roleClient"
							>
								<template #prepend>
									<VCheckboxBtn :model-value="roleClient" />
								</template>
								<VListItemTitle>{{ t("onboarding.clientTitle") }}</VListItemTitle>
								<VListItemSubtitle class="text-wrap">
									{{ t("onboarding.clientSubtitle") }}
								</VListItemSubtitle>
							</VListItem>

							<p class="text-caption text-medium-emphasis mt-3">
								{{ t("onboarding.changeLater") }}
							</p>
						</VCardText>
						<VCardActions>
							<VSpacer />
							<VBtn
								color="primary"
								variant="flat"
								:disabled="!rolesValid"
								@click="step = 2"
							>
								{{ t("common.next") }}
							</VBtn>
						</VCardActions>
					</VCard>
				</VWindowItem>

				<!-- Step 2: server -->
				<VWindowItem :value="2">
					<VCard variant="tonal">
						<VCardTitle>{{ t("onboarding.serverTitle") }}</VCardTitle>
						<VCardSubtitle class="text-wrap">
							{{ t("onboarding.serverSubtitle") }}
						</VCardSubtitle>
						<VCardText>
							<VTextField
								v-model="relayName"
								:label="t('onboarding.nameLabel')"
								:hint="t('onboarding.nameHint')"
								persistent-hint
								prepend-inner-icon="mdi-tag-outline"
								density="comfortable"
								class="mb-2"
							/>
							<VTextField
								v-model="relayUrl"
								:label="t('onboarding.serverLabel')"
								:placeholder="t('onboarding.serverPlaceholder')"
								:hint="t('onboarding.serverHint')"
								persistent-hint
								prepend-inner-icon="mdi-web"
								density="comfortable"
							/>
						</VCardText>
						<VCardActions>
							<VBtn variant="text" @click="step = 1">
								{{ t("common.back") }}
							</VBtn>
							<VSpacer />
							<VBtn
								color="primary"
								variant="flat"
								:disabled="!relayOk"
								@click="step = 3"
							>
								{{ relayUrl.trim() ? t("common.next") : t("common.skip") }}
							</VBtn>
						</VCardActions>
					</VCard>
				</VWindowItem>

				<!-- Step 3: device name -->
				<VWindowItem :value="3">
					<VCard variant="tonal">
						<VCardTitle>{{ t("onboarding.deviceTitle") }}</VCardTitle>
						<VCardSubtitle>{{ t("onboarding.deviceSubtitle") }}</VCardSubtitle>
						<VCardText>
							<VTextField
								v-model="deviceName"
								:label="t('onboarding.deviceLabel')"
								:placeholder="t('onboarding.devicePlaceholder')"
								prepend-inner-icon="mdi-laptop"
								density="comfortable"
								@keyup.enter="finish"
							/>
							<VAlert
								v-if="error"
								type="error"
								variant="tonal"
								density="compact"
							>
								{{ error }}
							</VAlert>
						</VCardText>
						<VCardActions>
							<VBtn variant="text" @click="step = 2">
								{{ t("common.back") }}
							</VBtn>
							<VSpacer />
							<VBtn
								color="primary"
								variant="flat"
								:loading="busy"
								@click="finish"
							>
								{{ t("onboarding.finish") }}
							</VBtn>
						</VCardActions>
					</VCard>
				</VWindowItem>
			</VWindow>
		</VCard>
	</VContainer>
</template>

<script setup lang="ts">
	import {
		computed, ref,
	} from "vue";
	import { useI18n } from "vue-i18n";
	import {
		addRelay, completeSetup,
	} from "../store";

	const emit = defineEmits<{ done: [] }>();
	const { t } = useI18n();

	const step = ref(1);

	// Step 1 — roles
	const roleHost = ref(true);
	const roleClient = ref(true);
	const rolesValid = computed(() => roleHost.value || roleClient.value);

	// Step 2 — server (optional; only the HTTP address, the app derives the rest)
	const relayName = ref(t("onboarding.defaultServerName"));
	const relayUrl = ref("");
	// Empty is allowed (skip); if filled it must look like an http(s) URL.
	const relayOk = computed(() => {
		const url = relayUrl.value.trim();
		return url === "" || url.startsWith("http");
	});

	// Step 3 — device name
	const deviceName = ref("");

	const error = ref<string | null>(null);
	const busy = ref(false);

	async function finish() {
		error.value = null;
		busy.value = true;
		try {
			// The server is optional — only save one if an address was entered.
			if (relayUrl.value.trim()) {
				await addRelay(relayName.value, relayUrl.value);
			}
			const roles: string[] = [];
			if (roleHost.value) roles.push("host");
			if (roleClient.value) roles.push("client");
			await completeSetup(deviceName.value.trim() || t("onboarding.defaultDeviceName"), roles);
			emit("done");
		} catch (e) {
			error.value = typeof e === "string" ? e : String(e);
			step.value = 2; // most failures are the server address
		} finally {
			busy.value = false;
		}
	}
</script>
