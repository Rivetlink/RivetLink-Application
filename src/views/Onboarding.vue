<script setup lang="ts">
import {
    computed, ref,
} from "vue";
import { useI18n } from "vue-i18n";
import {
    addRelay, completeSetup,
} from "../store";

const emit = defineEmits<{ done: [] }>();
const { t } = useI18n();

const step = ref(1);

// Step 1 — roles
const roleHost = ref(true);
const roleClient = ref(true);
const rolesValid = computed(() => roleHost.value || roleClient.value);

// Step 2 — server (optional; only the HTTP address, the app derives the rest)
const relayName = ref(t("onboarding.defaultServerName"));
const relayUrl = ref("");
// Empty is allowed (skip); if filled it must look like an http(s) URL.
const relayOk = computed(() => {
    const url = relayUrl.value.trim();
    return url === "" || url.startsWith("http");
});

// Step 3 — device name
const deviceName = ref("");

const error = ref<string | null>(null);
const busy = ref(false);

async function finish() {
    error.value = null;
    busy.value = true;
    try {
        // The server is optional — only save one if an address was entered.
        if (relayUrl.value.trim()) {
            await addRelay(relayName.value, relayUrl.value);
        }
        const roles: string[] = [];
        if (roleHost.value) roles.push("host");
        if (roleClient.value) roles.push("client");
        await completeSetup(deviceName.value.trim() || t("onboarding.defaultDeviceName"), roles);
        emit("done");
    } catch (e) {
        error.value = typeof e === "string" ? e : String(e);
        step.value = 2; // most failures are the server address
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
                    {{ t("onboarding.welcomeTitle") }}
                </h1>
                <p class="text-medium-emphasis">
                    {{ t("onboarding.welcomeSubtitle") }}
                </p>
            </div>

            <v-window v-model="step">
                <!-- Step 1: role -->
                <v-window-item :value="1">
                    <v-card variant="tonal">
                        <v-card-title>{{ t("onboarding.roleQuestion") }}</v-card-title>
                        <v-card-subtitle>{{ t("onboarding.roleHint") }}</v-card-subtitle>
                        <v-card-text>
                            <v-list-item
                                class="rounded mb-2 border"
                                :active="roleHost"
                                @click="roleHost = !roleHost"
                            >
                                <template #prepend>
                                    <v-checkbox-btn :model-value="roleHost" />
                                </template>
                                <v-list-item-title>{{ t("onboarding.hostTitle") }}</v-list-item-title>
                                <v-list-item-subtitle class="text-wrap">
                                    {{ t("onboarding.hostSubtitle") }}
                                </v-list-item-subtitle>
                            </v-list-item>

                            <v-list-item
                                class="rounded border"
                                :active="roleClient"
                                @click="roleClient = !roleClient"
                            >
                                <template #prepend>
                                    <v-checkbox-btn :model-value="roleClient" />
                                </template>
                                <v-list-item-title>{{ t("onboarding.clientTitle") }}</v-list-item-title>
                                <v-list-item-subtitle class="text-wrap">
                                    {{ t("onboarding.clientSubtitle") }}
                                </v-list-item-subtitle>
                            </v-list-item>

                            <p class="text-caption text-medium-emphasis mt-3">
                                {{ t("onboarding.changeLater") }}
                            </p>
                        </v-card-text>
                        <v-card-actions>
                            <v-spacer />
                            <v-btn
                                color="primary"
                                variant="flat"
                                :disabled="!rolesValid"
                                @click="step = 2"
                            >
                                {{ t("common.next") }}
                            </v-btn>
                        </v-card-actions>
                    </v-card>
                </v-window-item>

                <!-- Step 2: server -->
                <v-window-item :value="2">
                    <v-card variant="tonal">
                        <v-card-title>{{ t("onboarding.serverTitle") }}</v-card-title>
                        <v-card-subtitle class="text-wrap">
                            {{ t("onboarding.serverSubtitle") }}
                        </v-card-subtitle>
                        <v-card-text>
                            <v-text-field
                                v-model="relayName"
                                :label="t('onboarding.nameLabel')"
                                :hint="t('onboarding.nameHint')"
                                persistent-hint
                                prepend-inner-icon="mdi-tag-outline"
                                density="comfortable"
                                class="mb-2"
                            />
                            <v-text-field
                                v-model="relayUrl"
                                :label="t('onboarding.serverLabel')"
                                :placeholder="t('onboarding.serverPlaceholder')"
                                :hint="t('onboarding.serverHint')"
                                persistent-hint
                                prepend-inner-icon="mdi-web"
                                density="comfortable"
                            />
                        </v-card-text>
                        <v-card-actions>
                            <v-btn variant="text" @click="step = 1">
                                {{ t("common.back") }}
                            </v-btn>
                            <v-spacer />
                            <v-btn
                                color="primary"
                                variant="flat"
                                :disabled="!relayOk"
                                @click="step = 3"
                            >
                                {{ relayUrl.trim() ? t("common.next") : t("common.skip") }}
                            </v-btn>
                        </v-card-actions>
                    </v-card>
                </v-window-item>

                <!-- Step 3: device name -->
                <v-window-item :value="3">
                    <v-card variant="tonal">
                        <v-card-title>{{ t("onboarding.deviceTitle") }}</v-card-title>
                        <v-card-subtitle>{{ t("onboarding.deviceSubtitle") }}</v-card-subtitle>
                        <v-card-text>
                            <v-text-field
                                v-model="deviceName"
                                :label="t('onboarding.deviceLabel')"
                                :placeholder="t('onboarding.devicePlaceholder')"
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
                                {{ t("common.back") }}
                            </v-btn>
                            <v-spacer />
                            <v-btn
                                color="primary"
                                variant="flat"
                                :loading="busy"
                                @click="finish"
                            >
                                {{ t("onboarding.finish") }}
                            </v-btn>
                        </v-card-actions>
                    </v-card>
                </v-window-item>
            </v-window>
        </v-card>
    </v-container>
</template>
