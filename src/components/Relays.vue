<template>
	<div class="d-flex flex-column ga-6">
		<div class="d-flex flex-wrap align-end justify-space-between ga-3">
			<div>
				<h2 class="rl-h2">
					{{ t("settings.tabs.servers") }}
				</h2>
				<p class="rl-lead">
					{{ t("relays.subtitle") }}
				</p>
			</div>
			<VBtn
				color="primary"
				prepend-icon="mdi-plus"
				@click="showAdd = !showAdd"
			>
				{{ t("relays.add") }}
			</VBtn>
		</div>

		<VExpandTransition>
			<div v-if="showAdd" class="rl-group pa-4">
				<VTextField
					v-model="name"
					:label="t('relays.name')"
					density="comfortable"
				/>
				<VTextField
					v-model="http"
					:label="t('relays.serverLabel')"
					:placeholder="t('relays.serverPlaceholder')"
					:hint="t('relays.serverHint')"
					persistent-hint
					density="comfortable"
				/>
				<div class="d-flex justify-end ga-2 mt-4">
					<VBtn variant="text" @click="showAdd = false">
						{{ t("common.cancel") }}
					</VBtn>
					<VBtn
						color="primary"
						:disabled="!valid"
						:loading="busy"
						@click="add"
					>
						{{ t("common.save") }}
					</VBtn>
				</div>
			</div>
		</VExpandTransition>

		<div class="rl-group">
			<div v-if="store.settings.relays.length === 0" class="rl-row rl-row-sub">
				{{ t("relays.empty") }}
			</div>
			<div v-for="r in store.settings.relays" :key="r.id" class="rl-row">
				<div class="rl-row-text">
					<span class="rl-row-title">{{ r.name }}</span>
					<span class="rl-row-sub rl-mono">{{ r.http_url }}</span>
				</div>
				<span
					v-if="store.settings.active_relay_id === r.id"
					class="rl-status"
					:class="store.connected ? 'ok' : ''"
				>{{ t("relays.active") }}</span>
				<VBtn
					v-else
					variant="outlined"
					@click="makeActive(r.id)"
				>
					{{ t("relays.activate") }}
				</VBtn>
				<VBtn
					variant="text"
					icon="mdi-delete-outline"
					:aria-label="t('relays.remove')"
					@click="remove(r.id)"
				/>
			</div>
		</div>

		<VSnackbar
			:model-value="!!error"
			color="error"
			timeout="6000"
			@update:model-value="error = null"
		>
			{{ error }}
		</VSnackbar>
	</div>
</template>

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
