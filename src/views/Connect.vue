<script setup lang="ts">
import {
    onMounted, ref,
} from "vue";
import {
    activeRelay,
    captureScreenshot,
    connect,
    listDevices,
    login,
    store,
    type Device,
} from "../store";

const tab = ref("devices");

const email = ref("");
const password = ref("");

const devices = ref<Device[]>([]);
const selected = ref<string | null>(null);
const screenshot = ref<string | null>(null);

const busy = ref(false);
const busyMsg = ref<string | null>(null);
const error = ref<string | null>(null);

const sessionCode = ref("");

function fail(e: unknown) {
    error.value = typeof e === "string" ? e : String(e);
}

onMounted(() => {
    // Best-effort: open the relay connection up front.
    if (activeRelay() && !store.connected) {
        doConnect();
    }
});

async function doConnect() {
    error.value = null;
    busy.value = true;
    try {
        await connect();
    } catch (e) {
        fail(e);
    } finally {
        busy.value = false;
    }
}

async function doLogin() {
    error.value = null;
    busy.value = true;
    try {
        await login(email.value, password.value);
        await refresh();
    } catch (e) {
        fail(e);
    } finally {
        busy.value = false;
    }
}

async function refresh() {
    error.value = null;
    busy.value = true;
    try {
        devices.value = await listDevices();
    } catch (e) {
        fail(e);
    } finally {
        busy.value = false;
    }
}

async function capture() {
    if (!selected.value) return;
    error.value = null;
    screenshot.value = null;
    busyMsg.value = "Sessie aanvragen — de host kan om goedkeuring vragen…";
    try {
        screenshot.value = await captureScreenshot(selected.value);
    } catch (e) {
        fail(e);
    } finally {
        busyMsg.value = null;
    }
}
</script>

<template>
    <v-container style="max-width: 880px">
        <!-- No relay yet -->
        <v-alert
            v-if="!activeRelay()"
            type="info"
            variant="tonal"
            class="mb-4"
        >
            Nog geen actieve relay. Voeg er een toe onder
            <router-link to="/relays">
                Relays
            </router-link>.
        </v-alert>

        <template v-else>
            <v-card variant="tonal" class="mb-4">
                <v-card-text class="d-flex align-center">
                    <v-icon icon="mdi-server-network" class="mr-2" />
                    <div>
                        <div class="text-body-1">
                            {{ activeRelay()?.name }}
                        </div>
                        <div class="text-caption text-medium-emphasis">
                            {{ activeRelay()?.http_url }}
                        </div>
                    </div>
                    <v-spacer />
                    <v-chip
                        :color="store.connected ? 'green' : 'grey'"
                        size="small"
                        variant="flat"
                    >
                        {{ store.connected ? "verbonden" : "niet verbonden" }}
                    </v-chip>
                </v-card-text>
            </v-card>

            <v-tabs v-model="tab" class="mb-4">
                <v-tab value="devices" prepend-icon="mdi-monitor">
                    Apparaten
                </v-tab>
                <v-tab value="code" prepend-icon="mdi-numeric">
                    Sessie-code
                </v-tab>
            </v-tabs>

            <v-window v-model="tab">
                <!-- Managed devices -->
                <v-window-item value="devices">
                    <!-- Sign in -->
                    <v-card v-if="!store.loggedIn" variant="tonal">
                        <v-card-title>Inloggen</v-card-title>
                        <v-card-subtitle>
                            Meld je aan om de apparaten in je organisatie te zien.
                        </v-card-subtitle>
                        <v-card-text>
                            <v-text-field
                                v-model="email"
                                label="E-mail"
                                prepend-inner-icon="mdi-email-outline"
                                density="comfortable"
                            />
                            <v-text-field
                                v-model="password"
                                label="Wachtwoord"
                                type="password"
                                prepend-inner-icon="mdi-lock-outline"
                                density="comfortable"
                                hide-details
                                @keyup.enter="doLogin"
                            />
                        </v-card-text>
                        <v-card-actions>
                            <v-spacer />
                            <v-btn
                                color="primary"
                                variant="flat"
                                :loading="busy"
                                :disabled="!store.connected"
                                @click="doLogin"
                            >
                                Inloggen
                            </v-btn>
                        </v-card-actions>
                    </v-card>

                    <!-- Device list -->
                    <template v-else>
                        <v-card variant="tonal">
                            <v-card-title class="d-flex align-center">
                                Apparaten
                                <v-spacer />
                                <v-btn
                                    size="small"
                                    variant="text"
                                    icon="mdi-refresh"
                                    :loading="busy"
                                    @click="refresh"
                                />
                            </v-card-title>
                            <v-card-text>
                                <v-alert
                                    v-if="!busy && devices.length === 0"
                                    type="warning"
                                    variant="tonal"
                                    density="compact"
                                >
                                    Nog geen apparaten in je organisatie.
                                </v-alert>
                                <v-list v-else lines="two" density="comfortable">
                                    <v-list-item
                                        v-for="d in devices"
                                        :key="d.id"
                                        :active="selected === d.id"
                                        @click="selected = d.id"
                                    >
                                        <template #prepend>
                                            <v-icon icon="mdi-monitor" />
                                        </template>
                                        <v-list-item-title>{{ d.hostname || d.id }}</v-list-item-title>
                                        <v-list-item-subtitle>
                                            {{ d.platform || "onbekend" }} · laatst gezien
                                            {{ d.last_seen || "nooit" }}
                                        </v-list-item-subtitle>
                                        <template #append>
                                            <v-icon
                                                v-if="selected === d.id"
                                                icon="mdi-check-circle"
                                                color="primary"
                                            />
                                        </template>
                                    </v-list-item>
                                </v-list>
                            </v-card-text>
                            <v-card-actions>
                                <v-spacer />
                                <v-btn
                                    color="primary"
                                    variant="flat"
                                    prepend-icon="mdi-camera"
                                    :disabled="!selected"
                                    :loading="!!busyMsg"
                                    @click="capture"
                                >
                                    Screenshot maken
                                </v-btn>
                            </v-card-actions>
                        </v-card>

                        <v-card v-if="screenshot" class="mt-4">
                            <v-card-title>Screenshot</v-card-title>
                            <v-img :src="screenshot" />
                        </v-card>
                    </template>
                </v-window-item>

                <!-- Session code (planned) -->
                <v-window-item value="code">
                    <v-card variant="tonal">
                        <v-card-title class="d-flex align-center">
                            Verbinden met een sessie-code
                            <v-chip class="ml-2" size="x-small" color="amber">
                                binnenkort
                            </v-chip>
                        </v-card-title>
                        <v-card-subtitle>
                            Ad-hoc support zonder account — de host deelt een code.
                        </v-card-subtitle>
                        <v-card-text>
                            <v-otp-input
                                v-model="sessionCode"
                                length="9"
                                disabled
                            />
                            <p class="text-caption text-medium-emphasis">
                                Dit verbindingsmodel wordt nog gebouwd (relay + protocol).
                            </p>
                        </v-card-text>
                    </v-card>
                </v-window-item>
            </v-window>
        </template>

        <!-- Feedback -->
        <v-snackbar :model-value="!!busyMsg" color="grey-darken-3" timeout="-1">
            <v-progress-circular indeterminate size="18" class="mr-2" /> {{ busyMsg }}
        </v-snackbar>
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
