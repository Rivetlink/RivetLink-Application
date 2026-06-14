<script setup lang="ts">
import {
    onMounted, ref,
} from "vue";
import { useI18n } from "vue-i18n";
import {
    loadPublicKey, store,
} from "../store";

const { t } = useI18n();
const copied = ref(false);

onMounted(async () => {
    if (!store.publicKey) {
        await loadPublicKey();
    }
});

async function copyKey() {
    try {
        await navigator.clipboard.writeText(store.publicKey);
        copied.value = true;
        setTimeout(() => (copied.value = false), 1500);
    } catch {
        // Clipboard may be unavailable; the field is selectable as a fallback.
    }
}
</script>

<template>
    <v-container style="max-width: 880px">
        <v-card variant="tonal" class="mb-4">
            <v-card-text class="d-flex align-center">
                <v-icon icon="mdi-laptop" size="32" class="mr-3" />
                <div>
                    <div class="text-h6">
                        {{ store.settings.device_name || t("app.unnamedDevice") }}
                    </div>
                    <div class="text-caption text-medium-emphasis">
                        {{ t("device.thisDevice") }}
                    </div>
                </div>
                <v-spacer />
                <v-chip color="grey" size="small" variant="flat">
                    <v-icon start icon="mdi-circle" size="x-small" /> {{ t("device.offline") }}
                </v-chip>
            </v-card-text>
        </v-card>

        <v-card variant="tonal" class="mb-4">
            <v-card-title>{{ t("device.identityTitle") }}</v-card-title>
            <v-card-subtitle class="text-wrap">
                {{ t("device.identitySubtitle") }}
            </v-card-subtitle>
            <v-card-text>
                <v-text-field
                    :model-value="store.publicKey"
                    :label="t('device.publicKey')"
                    readonly
                    density="comfortable"
                    append-inner-icon="mdi-content-copy"
                    @click:append-inner="copyKey"
                />
                <v-fade-transition>
                    <span v-if="copied" class="text-caption text-success">{{ t("device.copied") }}</span>
                </v-fade-transition>
            </v-card-text>
        </v-card>

        <v-card variant="tonal">
            <v-card-title class="d-flex align-center">
                {{ t("device.hostAgentTitle") }}
                <v-chip class="ml-2" size="x-small" color="amber">
                    {{ t("common.soon") }}
                </v-chip>
            </v-card-title>
            <v-card-text>
                <p class="text-body-2 text-medium-emphasis mb-2">
                    {{ t("device.hostAgentIntro") }}
                </p>
                <v-list density="compact" class="bg-transparent">
                    <v-list-item prepend-icon="mdi-shield-check" :title="t('device.hostConsent')" />
                    <v-list-item prepend-icon="mdi-account-key" :title="t('device.hostTrusted')" />
                    <v-list-item prepend-icon="mdi-monitor-screenshot" :title="t('device.hostScreen')" />
                </v-list>
                <p class="text-caption text-medium-emphasis mt-2">
                    {{ t("device.hostAgentNote") }}
                </p>
            </v-card-text>
        </v-card>
    </v-container>
</template>
