<script setup lang="ts">
import {
	onMounted, ref,
} from "vue";
import { useI18n } from "vue-i18n";
import {
	isClient, isHost, loadPublicKey, store, updateDevice,
} from "../store";
import {
	SUPPORTED, setLocale,
} from "../i18n";

const {
	t, locale,
} = useI18n();

onMounted(async () => {
	if (!store.publicKey) {
		await loadPublicKey();
	}
});

function onLocaleChange(code: string) {
	setLocale(code);
}

// --- Edit this device (name + roles) ---
const editOpen = ref(false);
const editName = ref("");
const editHost = ref(false);
const editClient = ref(false);
const saving = ref(false);

function openEdit() {
	editName.value = store.settings.device_name;
	editHost.value = isHost();
	editClient.value = isClient();
	editOpen.value = true;
}

async function saveEdit() {
	if (!editHost.value && !editClient.value) return;
	saving.value = true;
	try {
		const roles: string[] = [];
		if (editHost.value) roles.push("host");
		if (editClient.value) roles.push("client");
		await updateDevice(editName.value.trim() || t("onboarding.defaultDeviceName"), roles);
		editOpen.value = false;
	} finally {
		saving.value = false;
	}
}
</script>

<template>
	<VContainer style="max-width: 880px">
		<VCard variant="tonal" class="mb-4">
			<VCardTitle class="d-flex align-center">
				{{ t("settings.thisDevice") }}
				<VSpacer />
				<VBtn
					variant="text"
					size="small"
					prepend-icon="mdi-pencil"
					@click="openEdit"
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

		<VCard variant="tonal">
			<VCardTitle>{{ t("settings.about") }}</VCardTitle>
			<VList class="bg-transparent">
				<VListItem title="RivetLink" :subtitle="t('settings.tagline')" />
				<VListItem :title="t('settings.version')" subtitle="0.1.5" />
			</VList>
		</VCard>

		<VDialog v-model="editOpen" max-width="480">
			<VCard>
				<VCardTitle>{{ t("settings.editDevice") }}</VCardTitle>
				<VCardText>
					<VTextField
						v-model="editName"
						:label="t('onboarding.deviceLabel')"
						:placeholder="t('onboarding.devicePlaceholder')"
						persistent-placeholder
						density="comfortable"
						autofocus
						@keyup.enter="saveEdit"
					/>
					<div class="text-subtitle-2 mt-2 mb-1">
						{{ t("settings.roles") }}
					</div>
					<VCheckbox
						v-model="editHost"
						:label="t('onboarding.hostTitle')"
						hide-details
						density="comfortable"
					/>
					<VCheckbox
						v-model="editClient"
						:label="t('onboarding.clientTitle')"
						hide-details
						density="comfortable"
					/>
					<p v-if="!editHost && !editClient" class="text-caption text-error mt-2">
						{{ t("settings.rolesRequired") }}
					</p>
				</VCardText>
				<VCardActions>
					<VSpacer />
					<VBtn variant="text" @click="editOpen = false">
						{{ t("common.cancel") }}
					</VBtn>
					<VBtn
						color="primary"
						variant="flat"
						:loading="saving"
						:disabled="!editHost && !editClient"
						@click="saveEdit"
					>
						{{ t("common.save") }}
					</VBtn>
				</VCardActions>
			</VCard>
		</VDialog>
	</VContainer>
</template>
