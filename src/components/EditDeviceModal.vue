<template>
	<VDialog v-model="open" max-width="480">
		<VCard>
			<VCardTitle>{{ t("settings.editDevice") }}</VCardTitle>
			<VCardText>
				<VTextField
					v-model="name"
					:label="t('onboarding.deviceLabel')"
					:placeholder="t('onboarding.devicePlaceholder')"
					persistent-placeholder
					density="comfortable"
					autofocus
				/>
				<div class="text-subtitle-2 mt-2 mb-1">
					{{ t("settings.roles") }}
				</div>
				<VCheckbox
					v-model="host"
					:label="t('onboarding.hostTitle')"
					hide-details
					density="comfortable"
				/>
				<VCheckbox
					v-model="client"
					:label="t('onboarding.clientTitle')"
					hide-details
					density="comfortable"
				/>
				<p v-if="!host && !client" class="text-caption text-error mt-2">
					{{ t("settings.rolesRequired") }}
				</p>
			</VCardText>
			<VCardActions>
				<VSpacer />
				<VBtn variant="text" @click="open = false">
					{{ t("common.cancel") }}
				</VBtn>
				<VBtn
					color="primary"
					variant="flat"
					:loading="saving"
					:disabled="!host && !client"
					@click="save"
				>
					{{ t("common.save") }}
				</VBtn>
			</VCardActions>
		</VCard>
	</VDialog>
</template>

<script setup lang="ts">
	import {
		ref, watch,
	} from "vue";
	import { useI18n } from "vue-i18n";
	import {
		isClient, isHost, store, updateDevice,
	} from "../store";

	const open = defineModel<boolean>({ required: true });

	const { t } = useI18n();
	const name = ref("");
	const host = ref(false);
	const client = ref(false);
	const saving = ref(false);

	// Load the current values whenever the dialog opens.
	watch(open, (visible) => {
		if (visible) {
			name.value = store.settings.device_name;
			host.value = isHost();
			client.value = isClient();
		}
	});

	async function save() {
		if (!host.value && !client.value) {return;}
		saving.value = true;
		try {
			const roles: string[] = [];
			if (host.value) {roles.push("host");}
			if (client.value) {roles.push("client");}
			await updateDevice(name.value.trim() || t("onboarding.defaultDeviceName"), roles);
			open.value = false;
		} finally {
			saving.value = false;
		}
	}
</script>
