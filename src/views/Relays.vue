<script setup lang="ts">
import {
    computed, ref,
} from "vue";
import {
    addRelay, removeRelay, setActiveRelay, store,
} from "../store";

const showAdd = ref(false);
const name = ref("");
const http = ref("http://127.0.0.1:8080");
const ws = ref("ws://127.0.0.1:8080/ws");
const busy = ref(false);
const error = ref<string | null>(null);

const valid = computed(
    () => name.value.trim().length > 0
        && http.value.startsWith("http")
        && ws.value.startsWith("ws"),
);

async function add() {
    error.value = null;
    busy.value = true;
    try {
        await addRelay(name.value, http.value, ws.value);
        showAdd.value = false;
        name.value = "";
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
                Opgeslagen relays
            </h2>
            <v-spacer />
            <v-btn
                color="primary"
                variant="flat"
                prepend-icon="mdi-plus"
                @click="showAdd = !showAdd"
            >
                Relay toevoegen
            </v-btn>
        </div>

        <v-expand-transition>
            <v-card v-if="showAdd" variant="tonal" class="mb-4">
                <v-card-text>
                    <v-text-field
                        v-model="name"
                        label="Naam"
                        prepend-inner-icon="mdi-tag-outline"
                        density="comfortable"
                    />
                    <v-text-field
                        v-model="http"
                        label="HTTP URL"
                        prepend-inner-icon="mdi-web"
                        density="comfortable"
                    />
                    <v-text-field
                        v-model="ws"
                        label="WebSocket URL"
                        prepend-inner-icon="mdi-transit-connection-variant"
                        density="comfortable"
                        hide-details
                    />
                </v-card-text>
                <v-card-actions>
                    <v-spacer />
                    <v-btn variant="text" @click="showAdd = false">
                        Annuleren
                    </v-btn>
                    <v-btn
                        color="primary"
                        variant="flat"
                        :disabled="!valid"
                        :loading="busy"
                        @click="add"
                    >
                        Opslaan
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-expand-transition>

        <v-alert
            v-if="store.settings.relays.length === 0"
            type="info"
            variant="tonal"
        >
            Nog geen relays opgeslagen.
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
                    <v-list-item-subtitle>{{ r.http_url }} · {{ r.ws_url }}</v-list-item-subtitle>
                    <template #append>
                        <v-chip
                            v-if="store.settings.active_relay_id === r.id"
                            color="primary"
                            size="small"
                            variant="flat"
                            class="mr-2"
                        >
                            actief
                        </v-chip>
                        <v-btn
                            v-else
                            size="small"
                            variant="text"
                            @click="makeActive(r.id)"
                        >
                            Activeren
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
