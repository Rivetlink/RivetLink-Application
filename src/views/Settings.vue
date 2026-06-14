<script setup lang="ts">
import { onMounted } from "vue";
import { useI18n } from "vue-i18n";
import {
    isClient, isHost, loadPublicKey, store,
} from "../store";
import {
    SUPPORTED, setLocale,
} from "../i18n";

const {
    t, locale,
} = useI18n();

onMounted(async () => {
    if (!store.publicKey) {
        await loadPublicKey();
    }
});

function onLocaleChange(code: string) {
    setLocale(code);
}
</script>

<template>
    <v-container style="max-width: 880px">
        <v-card variant="tonal" class="mb-4">
            <v-card-title>{{ t("settings.thisDevice") }}</v-card-title>
            <v-list class="bg-transparent">
                <v-list-item :title="t('settings.name')" :subtitle="store.settings.device_name || '—'" />
                <v-list-item :title="t('settings.roles')">
                    <template #subtitle>
                        <v-chip v-if="isHost()" size="x-small" class="mr-1">
                            {{ t("common.host") }}
                        </v-chip>
                        <v-chip v-if="isClient()" size="x-small">
                            {{ t("common.client") }}
                        </v-chip>
                        <span v-if="!isHost() && !isClient()">—</span>
                    </template>
                </v-list-item>
            </v-list>
        </v-card>

        <v-card variant="tonal" class="mb-4">
            <v-card-title>{{ t("settings.language") }}</v-card-title>
            <v-card-text>
                <v-select
                    :model-value="locale"
                    :items="SUPPORTED"
                    item-title="label"
                    item-value="code"
                    density="comfortable"
                    hide-details
                    prepend-inner-icon="mdi-translate"
                    @update:model-value="onLocaleChange"
                />
            </v-card-text>
        </v-card>

        <v-card variant="tonal" class="mb-4">
            <v-card-title>{{ t("settings.identityTitle") }}</v-card-title>
            <v-card-text>
                <v-text-field
                    :model-value="store.publicKey"
                    :label="t('settings.publicKey')"
                    readonly
                    density="comfortable"
                    hide-details
                />
                <p class="text-caption text-medium-emphasis mt-2">
                    {{ t("settings.privateNote") }}
                </p>
            </v-card-text>
        </v-card>

        <v-card variant="tonal">
            <v-card-title>{{ t("settings.about") }}</v-card-title>
            <v-list class="bg-transparent">
                <v-list-item title="RivetLink" :subtitle="t('settings.tagline')" />
                <v-list-item :title="t('settings.version')" subtitle="0.1.3" />
            </v-list>
        </v-card>
    </v-container>
</template>
