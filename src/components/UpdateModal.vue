<template>
	<VDialog
		v-model="updateState.dialog"
		max-width="420"
		:persistent="updateState.forced"
	>
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
					:type="updateState.forced ? 'warning' : 'info'"
					variant="tonal"
					density="compact"
				>
					{{ t("updates.available", { version: updateState.latest }) }}
					<div v-if="updateState.forced" class="mt-1 font-weight-medium">
						{{ t("updates.required") }}
					</div>
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
					v-if="canInstall"
					color="primary"
					variant="text"
					:loading="updateState.installing"
					:prepend-icon="updateState.canAutoInstall ? 'mdi-download' : 'mdi-open-in-new'"
					@click="installUpdate"
				>
					{{ updateState.canAutoInstall ? t("updates.install") : t("updates.download") }}
				</VBtn>
				<VSpacer />
				<VBtn
					v-if="!updateState.forced"
					variant="text"
					@click="updateState.dialog = false"
				>
					{{ t("common.close") }}
				</VBtn>
			</VCardActions>
		</VCard>
	</VDialog>
</template>

<script setup lang="ts">
	import { computed } from "vue";
	import { useI18n } from "vue-i18n";
	import {
		installUpdate, updateState, UpdateStatus,
	} from "../updates";

	const { t } = useI18n();

	// Show the install/download action whenever an update is available — and also
	// after a *forced* install fails, so a mandatory dialog (which has no close
	// button) always keeps a retry path instead of trapping the user.
	const canInstall = computed(() =>
		updateState.status === UpdateStatus.Available
		|| (updateState.forced && updateState.status === UpdateStatus.Error));
</script>
