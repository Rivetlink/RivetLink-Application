<script setup lang="ts">
import {
	computed, ref,
} from "vue";
import { useI18n } from "vue-i18n";
import {
	addRelay, removeRelay, setActiveRelay, store,
} from "../store";

const { t } = useI18n();

const showAdd = ref(false);
const name = ref("");
const http = ref("");
const busy = ref(false);
const error = ref<string | null>(null);

const valid = computed(
	() => name.value.trim().length > 0 && http.value.trim().startsWith("http"),
);

async function add() {
	error.value = null;
	busy.value = true;
	try {
		await addRelay(name.value, http.value);
		showAdd.value = false;
		name.value = "";
		http.value = "";
	} catch (e) {
		error.value = typeof e === "string" ? e : String(e);
	} finally {
		busy.value = false;
	}
}

async function makeActive(id: string) {
	error.value = null;
	try {
		await setActiveRelay(id);
	} catch (e) {
		error.value = typeof e === "string" ? e : String(e);
	}
}

async function remove(id: string) {
	error.value = null;
	try {
		await removeRelay(id);
	} catch (e) {
		error.value = typeof e === "string" ? e : String(e);
	}
}
</script>

<template>
	<VContainer style="max-width: 880px">
		<div class="d-flex align-center mb-4">
			<h2 class="text-h6">
				{{ t("relays.title") }}
			</h2>
			<VSpacer />
			<VBtn
				color="primary"
				variant="flat"
				prepend-icon="mdi-plus"
				@click="showAdd = !showAdd"
			>
				{{ t("relays.add") }}
			</VBtn>
		</div>

		<VExpandTransition>
			<VCard v-if="showAdd" variant="tonal" class="mb-4">
				<VCardText>
					<VTextField
						v-model="name"
						:label="t('relays.name')"
						prepend-inner-icon="mdi-tag-outline"
						density="comfortable"
					/>
					<VTextField
						v-model="http"
						:label="t('relays.serverLabel')"
						:placeholder="t('relays.serverPlaceholder')"
						:hint="t('relays.serverHint')"
						persistent-hint
						prepend-inner-icon="mdi-web"
						density="comfortable"
					/>
				</VCardText>
				<VCardActions>
					<VSpacer />
					<VBtn variant="text" @click="showAdd = false">
						{{ t("common.cancel") }}
					</VBtn>
					<VBtn
						color="primary"
						variant="flat"
						:disabled="!valid"
						:loading="busy"
						@click="add"
					>
						{{ t("common.save") }}
					</VBtn>
				</VCardActions>
			</VCard>
		</VExpandTransition>

		<VAlert
			v-if="store.settings.relays.length === 0"
			type="info"
			variant="tonal"
		>
			{{ t("relays.empty") }}
		</VAlert>

		<VCard v-else variant="tonal">
			<VList density="comfortable">
				<VListItem
					v-for="r in store.settings.relays"
					:key="r.id"
					:active="store.settings.active_relay_id === r.id"
				>
					<template #prepend>
						<VIcon icon="mdi-server-network" />
					</template>
					<VListItemTitle>{{ r.name }}</VListItemTitle>
					<VListItemSubtitle>{{ r.http_url }}</VListItemSubtitle>
					<template #append>
						<VChip
							v-if="store.settings.active_relay_id === r.id"
							color="primary"
							size="small"
							variant="flat"
							class="mr-2"
						>
							{{ t("relays.active") }}
						</VChip>
						<VBtn
							v-else
							size="small"
							variant="text"
							@click="makeActive(r.id)"
						>
							{{ t("relays.activate") }}
						</VBtn>
						<VBtn
							size="small"
							variant="text"
							icon="mdi-delete-outline"
							color="error"
							@click="remove(r.id)"
						/>
					</template>
				</VListItem>
			</VList>
		</VCard>

		<VSnackbar
			:model-value="!!error"
			color="error"
			timeout="6000"
			@update:model-value="error = null"
		>
			{{ error }}
		</VSnackbar>
	</VContainer>
</template>
