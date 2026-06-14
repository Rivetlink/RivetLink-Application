<template>
	<VDialog v-model="open" max-width="460">
		<VCard>
			<VCardTitle>{{ t("connect.lanConnectTitle", { name: target?.name }) }}</VCardTitle>
			<VCardText>
				<VTextField
					v-model="pin"
					:label="t('connect.lanPinLabel')"
					prepend-inner-icon="mdi-dialpad"
					density="comfortable"
					autofocus
					@keyup.enter="submit"
				/>
				<p class="text-caption text-medium-emphasis">
					{{ t("connect.lanPinHint") }}
				</p>
			</VCardText>
			<VCardActions>
				<VSpacer />
				<VBtn variant="text" @click="open = false">
					{{ t("common.cancel") }}
				</VBtn>
				<VBtn color="primary" variant="flat" @click="submit">
					{{ t("connect.lanConnect") }}
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
	import type { SavedLanDevice } from "../store";

	const open = defineModel<boolean>({ required: true });
	defineProps<{ target: SavedLanDevice | null }>();
	const emit = defineEmits<{ connect: [pin: string] }>();

	const { t } = useI18n();
	const pin = ref("");

	// Reset the field each time the dialog opens.
	watch(open, (visible) => {
		if (visible) {pin.value = "";}
	});

	function submit() {
		open.value = false;
		emit("connect", pin.value.trim());
	}
</script>
