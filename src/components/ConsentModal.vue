<template>
	<VDialog
		v-model="open"
		max-width="460"
		persistent
	>
		<VCard>
			<VCardTitle class="d-flex align-center ga-2">
				<VIcon icon="mdi-account-question-outline" color="primary" />
				{{ t("consent.title") }}
			</VCardTitle>
			<VCardText>
				<p class="mb-3">
					{{ t("consent.body", { name: req?.name || t("consent.someone") }) }}
				</p>
				<VAlert
					type="warning"
					variant="tonal"
					density="compact"
					class="mb-3"
					:text="t('consent.controlNote')"
				/>
				<div v-if="req?.key" class="text-caption text-medium-emphasis mb-3">
					{{ t("consent.key") }} <code>{{ fingerprint }}</code>
				</div>
				<VCheckbox
					v-model="remember"
					:disabled="!req?.key"
					:label="t('consent.remember')"
					density="compact"
					hide-details
				/>
				<div class="text-caption text-medium-emphasis mt-1 ml-1">
					{{ t("consent.rememberHint") }}
				</div>
			</VCardText>
			<VCardActions>
				<VSpacer />
				<VBtn variant="text" @click="decide(false)">
					{{ t("consent.reject") }}
				</VBtn>
				<VBtn color="primary" variant="flat" @click="decide(true)">
					{{ t("consent.accept") }}
				</VBtn>
			</VCardActions>
		</VCard>
	</VDialog>
</template>

<script setup lang="ts">
	import {
		computed, onMounted, onUnmounted, ref,
	} from "vue";
	import { useI18n } from "vue-i18n";
	import {
		listen, type UnlistenFn,
	} from "@tauri-apps/api/event";
	import { respondConsent } from "../store";

	type ConsentRequest = {
		id: number;
		key: string | null;
		name: string;
	};

	const { t } = useI18n();
	const open = ref(false);
	const remember = ref(false);
	const req = ref<ConsentRequest | null>(null);
	const fingerprint = computed(() => (req.value?.key ?? "").slice(0, 12));

	let unlisten: UnlistenFn | null = null;

	async function decide(accept: boolean): Promise<void> {
		const current = req.value;
		open.value = false;
		if (current) {
			// "Remember" only makes sense on accept, and only with a key.
			const rememberIt = accept && remember.value && current.key !== null;
			await respondConsent(current.id, accept, rememberIt);
		}
	}

	onMounted(async () => {
		unlisten = await listen<ConsentRequest>("host://consent-request", (e) => {
			req.value = e.payload;
			remember.value = false; // default off, fresh each time
			open.value = true;
		});
	});

	onUnmounted(() => {
		unlisten?.();
	});
</script>
