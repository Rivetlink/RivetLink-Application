<template>
	<VDialog v-model="open" max-width="520">
		<VCard>
			<VCardTitle>
				{{ target ? t("access.removeTitle") : t("access.addTitle") }}
			</VCardTitle>
			<VCardText>
				<template v-if="target">
					<p class="mb-4">
						{{ t("access.removeConfirm", { name: target.name || target.public_key }) }}
					</p>
				</template>
				<template v-else>
					<VTextField
						v-model="name"
						:label="t('access.nameLabel')"
						prepend-inner-icon="mdi-tag"
						density="comfortable"
						autofocus
					/>
					<VTextarea
						v-model="publicKey"
						:label="t('access.keyLabel')"
						prepend-inner-icon="mdi-key"
						density="comfortable"
						rows="2"
						auto-grow
						:hint="t('access.keyHint')"
						persistent-hint
						class="mb-2"
					/>
				</template>
				<VTextField
					v-model="osPassword"
					:label="t('access.osPasswordLabel')"
					type="password"
					prepend-inner-icon="mdi-shield-account"
					density="comfortable"
					:hint="t('access.osPasswordHint')"
					persistent-hint
					@keyup.enter="submit"
				/>
				<VAlert
					v-if="error"
					type="error"
					variant="tonal"
					density="compact"
					class="mt-3"
				>
					{{ error }}
				</VAlert>
			</VCardText>
			<VCardActions>
				<VSpacer />
				<VBtn variant="text" :disabled="busy" @click="open = false">
					{{ t("common.cancel") }}
				</VBtn>
				<VBtn
					:color="target ? 'error' : 'primary'"
					variant="flat"
					:loading="busy"
					@click="submit"
				>
					{{ target ? t("access.removeBtn") : t("access.add") }}
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
		addTrustedKey, removeTrustedKey, type TrustedKey,
	} from "../store";

	const open = defineModel<boolean>({ required: true });
	const props = defineProps<{ target: TrustedKey | null }>();

	const { t } = useI18n();
	const name = ref("");
	const publicKey = ref("");
	const osPassword = ref("");
	const error = ref("");
	const busy = ref(false);

	// Reset fields whenever the dialog opens.
	watch(open, (visible) => {
		if (visible) {
			name.value = "";
			publicKey.value = "";
			osPassword.value = "";
			error.value = "";
			busy.value = false;
		}
	});

	function message(code: string): string {
		const known = ["wrong-os-password", "duplicate-key", "empty-key"];
		const key = known.includes(code) ? code : "generic";
		return t(`access.err.${key}`);
	}

	async function submit() {
		error.value = "";
		busy.value = true;
		try {
			if (props.target) {
				await removeTrustedKey(props.target.id, osPassword.value);
			} else {
				await addTrustedKey(name.value, publicKey.value, osPassword.value);
			}
			open.value = false;
		} catch (e) {
			error.value = message(typeof e === "string" ? e : String(e));
		} finally {
			busy.value = false;
		}
	}
</script>
