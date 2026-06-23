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
	</VContainer>
</template>

<script setup lang="ts">
	import {
		onMounted, ref,
	} from "vue";
	import { invoke } from "@tauri-apps/api/core";
	import {
		disable, enable, isEnabled,
	} from "@tauri-apps/plugin-autostart";
	import { useI18n } from "vue-i18n";
	import {
		isClient, isHost, loadPublicKey, store, type TrustedKey,
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
	});

	function onLocaleChange(code: string) {
		setLocale(code);
	}
</script>
