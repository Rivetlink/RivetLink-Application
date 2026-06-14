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
				</div>
				<VSpacer />
				<VChip color="grey" size="small" variant="flat">
					<VIcon start icon="mdi-circle" size="x-small" /> {{ t("device.offline") }}
				</VChip>
			</VCardText>
		</VCard>

		<VCard variant="tonal" class="mb-4">
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
				<VFadeTransition>
					<span v-if="copied" class="text-caption text-success">{{ t("device.copied") }}</span>
				</VFadeTransition>
			</VCardText>
		</VCard>

		<VCard variant="tonal">
			<VCardTitle class="d-flex align-center">
				{{ t("device.hostAgentTitle") }}
				<VChip class="ml-2" size="x-small" color="amber">
					{{ t("common.soon") }}
				</VChip>
			</VCardTitle>
			<VCardText>
				<p class="text-body-2 text-medium-emphasis mb-2">
					{{ t("device.hostAgentIntro") }}
				</p>
				<VList density="compact" class="bg-transparent">
					<VListItem prepend-icon="mdi-shield-check" :title="t('device.hostConsent')" />
					<VListItem prepend-icon="mdi-account-key" :title="t('device.hostTrusted')" />
					<VListItem prepend-icon="mdi-monitor-screenshot" :title="t('device.hostScreen')" />
				</VList>
				<p class="text-caption text-medium-emphasis mt-2">
					{{ t("device.hostAgentNote") }}
				</p>
			</VCardText>
		</VCard>
	</VContainer>
</template>

<script setup lang="ts">
	import {
		onMounted, ref,
	} from "vue";
	import { useI18n } from "vue-i18n";
	import {
		loadPublicKey, store,
	} from "../store";

	const { t } = useI18n();
	const copied = ref(false);

	onMounted(async () => {
		if (!store.publicKey) {
			await loadPublicKey();
		}
	});

	async function copyKey() {
		try {
			await navigator.clipboard.writeText(store.publicKey);
			copied.value = true;
			setTimeout(() => (copied.value = false), 1500);
		} catch {
		// Clipboard may be unavailable; the field is selectable as a fallback.
		}
	}
</script>
