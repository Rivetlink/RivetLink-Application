<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

type Device = {
    id: string;
    hostname: string | null;
    platform: string | null;
    last_seen: string | null;
};

// Connection
const relayHttpUrl = ref("http://127.0.0.1:8080");
const relayWsUrl = ref("ws://127.0.0.1:8080/ws");
const publicKey = ref<string | null>(null);
const connecting = ref(false);

// Auth
const email = ref("");
const password = ref("");
const loggedIn = ref(false);
const loggingIn = ref(false);

// Devices / capture
const devices = ref<Device[]>([]);
const loadingDevices = ref(false);
const selected = ref<string | null>(null);
const capturing = ref(false);
const screenshot = ref<string | null>(null);

// Feedback
const error = ref<string | null>(null);
const busyMsg = ref<string | null>(null);

function fail(e: unknown) {
    error.value = typeof e === "string" ? e : String(e);
}

async function connect() {
    error.value = null;
    connecting.value = true;
    try {
        publicKey.value = await invoke<string>("init_client", {
            relayWsUrl: relayWsUrl.value,
            relayHttpUrl: relayHttpUrl.value,
        });
    } catch (e) {
        fail(e);
    } finally {
        connecting.value = false;
    }
}

async function doLogin() {
    error.value = null;
    loggingIn.value = true;
    try {
        await invoke("login", {
            email: email.value,
            password: password.value,
        });
        loggedIn.value = true;
        await refreshDevices();
    } catch (e) {
        fail(e);
    } finally {
        loggingIn.value = false;
    }
}

async function refreshDevices() {
    error.value = null;
    loadingDevices.value = true;
    try {
        devices.value = await invoke<Device[]>("list_devices");
    } catch (e) {
        fail(e);
    } finally {
        loadingDevices.value = false;
    }
}

async function capture() {
    if (!selected.value) return;
    error.value = null;
    screenshot.value = null;
    capturing.value = true;
    busyMsg.value = "Requesting session — the host may prompt its operator to approve…";
    try {
        screenshot.value = await invoke<string>("capture_screenshot", {
            deviceId: selected.value,
        });
    } catch (e) {
        fail(e);
    } finally {
        capturing.value = false;
        busyMsg.value = null;
    }
}
</script>

<template>
    <v-app>
        <v-app-bar color="primary" density="comfortable" flat>
            <v-app-bar-title>
                <v-icon icon="mdi-shield-lock-outline" class="mr-2" />
                RivetLink
            </v-app-bar-title>
            <template #append>
                <v-chip
                    v-if="loggedIn"
                    color="green"
                    size="small"
                    variant="flat"
                >
                    <v-icon start icon="mdi-check-circle" /> signed in
                </v-chip>
            </template>
        </v-app-bar>

        <v-main>
            <v-container style="max-width: 880px">
                <!-- 1. Connection -->
                <v-card class="mb-4" variant="tonal">
                    <v-card-title>1 · Relay</v-card-title>
                    <v-card-text>
                        <v-text-field
                            v-model="relayHttpUrl"
                            label="Relay HTTP URL"
                            prepend-inner-icon="mdi-web"
                            density="comfortable"
                            :disabled="!!publicKey"
                        />
                        <v-text-field
                            v-model="relayWsUrl"
                            label="Relay WebSocket URL"
                            prepend-inner-icon="mdi-transit-connection-variant"
                            density="comfortable"
                            :disabled="!!publicKey"
                            hide-details
                        />
                        <v-alert
                            v-if="publicKey"
                            class="mt-3"
                            type="info"
                            variant="tonal"
                            density="compact"
                        >
                            Your client key (give it to a host to pre-trust):
                            <code class="d-block mt-1 text-caption">{{ publicKey }}</code>
                        </v-alert>
                    </v-card-text>
                    <v-card-actions>
                        <v-spacer />
                        <v-btn
                            :loading="connecting"
                            :disabled="!!publicKey"
                            color="primary"
                            variant="flat"
                            @click="connect"
                        >
                            {{ publicKey ? "Connected" : "Connect" }}
                        </v-btn>
                    </v-card-actions>
                </v-card>

                <!-- 2. Login -->
                <v-card class="mb-4" variant="tonal" :disabled="!publicKey">
                    <v-card-title>2 · Sign in</v-card-title>
                    <v-card-text>
                        <v-text-field
                            v-model="email"
                            label="Email"
                            prepend-inner-icon="mdi-email-outline"
                            density="comfortable"
                            :disabled="loggedIn"
                        />
                        <v-text-field
                            v-model="password"
                            label="Password"
                            type="password"
                            prepend-inner-icon="mdi-lock-outline"
                            density="comfortable"
                            hide-details
                            :disabled="loggedIn"
                            @keyup.enter="doLogin"
                        />
                    </v-card-text>
                    <v-card-actions>
                        <v-spacer />
                        <v-btn
                            :loading="loggingIn"
                            :disabled="loggedIn"
                            color="primary"
                            variant="flat"
                            @click="doLogin"
                        >
                            {{ loggedIn ? "Signed in" : "Sign in" }}
                        </v-btn>
                    </v-card-actions>
                </v-card>

                <!-- 3. Devices + capture -->
                <v-card variant="tonal" :disabled="!loggedIn">
                    <v-card-title class="d-flex align-center">
                        3 · Devices
                        <v-spacer />
                        <v-btn
                            size="small"
                            variant="text"
                            :loading="loadingDevices"
                            icon="mdi-refresh"
                            @click="refreshDevices"
                        />
                    </v-card-title>
                    <v-card-text>
                        <v-alert
                            v-if="loggedIn && !loadingDevices && devices.length === 0"
                            type="warning"
                            variant="tonal"
                            density="compact"
                        >
                            No devices registered in your organization yet.
                        </v-alert>

                        <v-list
                            v-else
                            lines="two"
                            density="comfortable"
                        >
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
                                    {{ d.platform || "unknown" }} · last seen {{ d.last_seen || "never" }}
                                </v-list-item-subtitle>
                                <template #append>
                                    <v-icon v-if="selected === d.id" icon="mdi-check-circle" color="primary" />
                                </template>
                            </v-list-item>
                        </v-list>
                    </v-card-text>
                    <v-card-actions>
                        <v-spacer />
                        <v-btn
                            :loading="capturing"
                            :disabled="!selected"
                            color="primary"
                            variant="flat"
                            prepend-icon="mdi-camera"
                            @click="capture"
                        >
                            Capture screenshot
                        </v-btn>
                    </v-card-actions>
                </v-card>

                <!-- Result -->
                <v-card v-if="screenshot" class="mt-4">
                    <v-card-title>Screenshot</v-card-title>
                    <v-img :src="screenshot" />
                </v-card>
            </v-container>
        </v-main>

        <!-- Busy / error feedback -->
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
    </v-app>
</template>
