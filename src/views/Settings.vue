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
				<VListItem :title="t('settings.version')" :subtitle="version || '—'" />
			</VList>
		</VCard>

		<EditDeviceModal v-model="editOpen" />
	</VContainer>
</template>

<script setup lang="ts">
	import {
		onMounted, ref,
	} from "vue";
	import { invoke } from "@tauri-apps/api/core";
	import { useI18n } from "vue-i18n";
	import {
		isClient, isHost, loadPublicKey, store,
	} from "../store";
	import {
		SUPPORTED, setLocale,
	} from "../i18n";
	import EditDeviceModal from "../components/EditDeviceModal.vue";

	const {
		t, locale,
	} = useI18n();

	const editOpen = ref(false);
	const version = ref("");

	onMounted(async () => {
		version.value = await invoke<string>("app_version");
		if (!store.publicKey) {
			await loadPublicKey();
		}
	});

	function onLocaleChange(code: string) {
		setLocale(code);
	}
</script>
