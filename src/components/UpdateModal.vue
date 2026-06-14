<template>
	<VDialog v-model="updateState.dialog" max-width="420">
		<VCard>
			<VCardTitle>{{ t("updates.title") }}</VCardTitle>
			<VCardText>
				<div class="d-flex align-center mb-2">
					<VIcon icon="mdi-shield-lock-outline" color="primary" class="mr-2" />
					<span>RivetLink {{ updateState.current }}</span>
				</div>

				<div v-if="updateState.checking" class="d-flex align-center">
					<VProgressCircular indeterminate size="18" class="mr-2" />
					{{ t("updates.checking") }}
				</div>
				<VAlert
					v-else-if="updateState.status === UpdateStatus.Available"
					type="info"
					variant="tonal"
					density="compact"
				>
					{{ t("updates.available", { version: updateState.latest }) }}
				</VAlert>
				<VAlert
					v-else-if="updateState.status === UpdateStatus.UpToDate"
					type="success"
					variant="tonal"
					density="compact"
				>
					{{ t("updates.upToDate") }}
				</VAlert>
				<VAlert
					v-else-if="updateState.status === UpdateStatus.Error"
					type="warning"
					variant="tonal"
					density="compact"
				>
					{{ t("updates.error") }}
				</VAlert>
			</VCardText>
			<VCardActions>
				<VBtn
					v-if="updateState.status === UpdateStatus.Available"
					color="primary"
					variant="text"
					:loading="updateState.installing"
					:prepend-icon="updateState.canAutoInstall ? 'mdi-download' : 'mdi-open-in-new'"
					@click="installUpdate"
				>
					{{ updateState.canAutoInstall ? t("updates.install") : t("updates.download") }}
				</VBtn>
				<VSpacer />
				<VBtn variant="text" @click="updateState.dialog = false">
					{{ t("common.close") }}
				</VBtn>
			</VCardActions>
		</VCard>
	</VDialog>
</template>

<script setup lang="ts">
	import { useI18n } from "vue-i18n";
	import {
		installUpdate, updateState, UpdateStatus,
	} from "../updates";

	const { t } = useI18n();
</script>
