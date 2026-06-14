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
    <v-container style="max-width: 880px">
        <div class="d-flex align-center mb-4">
            <h2 class="text-h6">
                {{ t("relays.title") }}
            </h2>
            <v-spacer />
            <v-btn
                color="primary"
                variant="flat"
                prepend-icon="mdi-plus"
                @click="showAdd = !showAdd"
            >
                {{ t("relays.add") }}
            </v-btn>
        </div>

        <v-expand-transition>
            <v-card v-if="showAdd" variant="tonal" class="mb-4">
                <v-card-text>
                    <v-text-field
                        v-model="name"
                        :label="t('relays.name')"
                        prepend-inner-icon="mdi-tag-outline"
                        density="comfortable"
                    />
                    <v-text-field
                        v-model="http"
                        :label="t('relays.serverLabel')"
                        :placeholder="t('relays.serverPlaceholder')"
                        :hint="t('relays.serverHint')"
                        persistent-hint
                        prepend-inner-icon="mdi-web"
                        density="comfortable"
                    />
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn variant="text" @click="showAdd = false">
                        {{ t("common.cancel") }}
                    </v-btn>
                    <v-btn
                        color="primary"
                        variant="flat"
                        :disabled="!valid"
                        :loading="busy"
                        @click="add"
                    >
                        {{ t("common.save") }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-expand-transition>

        <v-alert
            v-if="store.settings.relays.length === 0"
            type="info"
            variant="tonal"
        >
            {{ t("relays.empty") }}
        </v-alert>

        <v-card v-else variant="tonal">
            <v-list density="comfortable">
                <v-list-item
                    v-for="r in store.settings.relays"
                    :key="r.id"
                    :active="store.settings.active_relay_id === r.id"
                >
                    <template #prepend>
                        <v-icon icon="mdi-server-network" />
                    </template>
                    <v-list-item-title>{{ r.name }}</v-list-item-title>
                    <v-list-item-subtitle>{{ r.http_url }}</v-list-item-subtitle>
                    <template #append>
                        <v-chip
                            v-if="store.settings.active_relay_id === r.id"
                            color="primary"
                            size="small"
                            variant="flat"
                            class="mr-2"
                        >
                            {{ t("relays.active") }}
                        </v-chip>
                        <v-btn
                            v-else
                            size="small"
                            variant="text"
                            @click="makeActive(r.id)"
                        >
                            {{ t("relays.activate") }}
                        </v-btn>
                        <v-btn
                            size="small"
                            variant="text"
                            icon="mdi-delete-outline"
                            color="error"
                            @click="remove(r.id)"
                        />
                    </template>
                </v-list-item>
            </v-list>
        </v-card>

        <v-snackbar
            :model-value="!!error"
            color="error"
            timeout="6000"
            @update:model-value="error = null"
        >
            {{ error }}
        </v-snackbar>
    </v-container>
</template>
