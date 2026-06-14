<script setup lang="ts">
import {
    computed, ref,
} from "vue";
import {
    addRelay, completeSetup,
} from "../store";

const emit = defineEmits<{ done: [] }>();

const step = ref(1);

// Step 1 — roles
const roleHost = ref(true);
const roleClient = ref(true);
const rolesValid = computed(() => roleHost.value || roleClient.value);

// Step 2 — first relay
const relayName = ref("Mijn relay");
const relayHttp = ref("http://127.0.0.1:8080");
const relayWs = ref("ws://127.0.0.1:8080/ws");
const relayValid = computed(
    () => relayName.value.trim().length > 0
        && relayHttp.value.startsWith("http")
        && relayWs.value.startsWith("ws"),
);

// Step 3 — device name
const deviceName = ref("");

const error = ref<string | null>(null);
const busy = ref(false);

async function finish() {
    error.value = null;
    busy.value = true;
    try {
        await addRelay(relayName.value, relayHttp.value, relayWs.value);
        const roles: string[] = [];
        if (roleHost.value) roles.push("host");
        if (roleClient.value) roles.push("client");
        await completeSetup(deviceName.value.trim() || "Mijn apparaat", roles);
        emit("done");
    } catch (e) {
        error.value = typeof e === "string" ? e : String(e);
        step.value = 2; // most failures are relay-URL validation
    } finally {
        busy.value = false;
    }
}
</script>

<template>
    <v-container class="fill-height" style="max-width: 720px">
        <v-card class="w-100" variant="flat" color="transparent">
            <div class="text-center mb-6">
                <v-icon icon="mdi-shield-lock-outline" color="primary" size="48" />
                <h1 class="text-h4 mt-2">
                    Welkom bij RivetLink
                </h1>
                <p class="text-medium-emphasis">
                    Eenmalige instelling van dit apparaat.
                </p>
            </div>

            <v-window v-model="step">
                <!-- Step 1: role -->
                <v-window-item :value="1">
                    <v-card variant="tonal">
                        <v-card-title>Wat wil je op deze machine doen?</v-card-title>
                        <v-card-text>
                            <v-list-item
                                class="rounded mb-2"
                                :active="roleHost"
                                @click="roleHost = !roleHost"
                            >
                                <template #prepend>
                                    <v-checkbox-btn :model-value="roleHost" />
                                </template>
                                <v-list-item-title>Deze pc deelbaar maken (Host)</v-list-item-title>
                                <v-list-item-subtitle>
                                    Iemand anders kan — met jouw toestemming — dit scherm zien.
                                </v-list-item-subtitle>
                            </v-list-item>

                            <v-list-item
                                class="rounded"
                                :active="roleClient"
                                @click="roleClient = !roleClient"
                            >
                                <template #prepend>
                                    <v-checkbox-btn :model-value="roleClient" />
                                </template>
                                <v-list-item-title>Andere pc's bedienen (Client)</v-list-item-title>
                                <v-list-item-subtitle>
                                    Verbind met hosts in je organisatie of via een sessie-code.
                                </v-list-item-subtitle>
                            </v-list-item>
                        </v-card-text>
                        <v-card-actions>
                            <v-spacer />
                            <v-btn
                                color="primary"
                                variant="flat"
                                :disabled="!rolesValid"
                                @click="step = 2"
                            >
                                Volgende
                            </v-btn>
                        </v-card-actions>
                    </v-card>
                </v-window-item>

                <!-- Step 2: relay -->
                <v-window-item :value="2">
                    <v-card variant="tonal">
                        <v-card-title>Voeg een relay toe</v-card-title>
                        <v-card-subtitle>
                            De relay regelt de verbinding. Zelf gehost of die van je organisatie.
                        </v-card-subtitle>
                        <v-card-text>
                            <v-text-field
                                v-model="relayName"
                                label="Naam"
                                prepend-inner-icon="mdi-tag-outline"
                                density="comfortable"
                            />
                            <v-text-field
                                v-model="relayHttp"
                                label="HTTP URL"
                                prepend-inner-icon="mdi-web"
                                density="comfortable"
                            />
                            <v-text-field
                                v-model="relayWs"
                                label="WebSocket URL"
                                prepend-inner-icon="mdi-transit-connection-variant"
                                density="comfortable"
                                hide-details
                            />
                        </v-card-text>
                        <v-card-actions>
                            <v-btn variant="text" @click="step = 1">
                                Terug
                            </v-btn>
                            <v-spacer />
                            <v-btn
                                color="primary"
                                variant="flat"
                                :disabled="!relayValid"
                                @click="step = 3"
                            >
                                Volgende
                            </v-btn>
                        </v-card-actions>
                    </v-card>
                </v-window-item>

                <!-- Step 3: device name -->
                <v-window-item :value="3">
                    <v-card variant="tonal">
                        <v-card-title>Naam van dit apparaat</v-card-title>
                        <v-card-subtitle>Zo herkennen anderen deze machine.</v-card-subtitle>
                        <v-card-text>
                            <v-text-field
                                v-model="deviceName"
                                label="Apparaatnaam"
                                placeholder="bijv. Kantoor-pc"
                                prepend-inner-icon="mdi-laptop"
                                density="comfortable"
                                @keyup.enter="finish"
                            />
                            <v-alert
                                v-if="error"
                                type="error"
                                variant="tonal"
                                density="compact"
                            >
                                {{ error }}
                            </v-alert>
                        </v-card-text>
                        <v-card-actions>
                            <v-btn variant="text" @click="step = 2">
                                Terug
                            </v-btn>
                            <v-spacer />
                            <v-btn
                                color="primary"
                                variant="flat"
                                :loading="busy"
                                @click="finish"
                            >
                                Aan de slag
                            </v-btn>
                        </v-card-actions>
                    </v-card>
                </v-window-item>
            </v-window>
        </v-card>
    </v-container>
</template>
