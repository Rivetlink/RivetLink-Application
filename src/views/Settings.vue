<script setup lang="ts">
import { onMounted } from "vue";
import {
    isClient, isHost, loadPublicKey, store,
} from "../store";

onMounted(async () => {
    if (!store.publicKey) {
        await loadPublicKey();
    }
});
</script>

<template>
    <v-container style="max-width: 880px">
        <v-card variant="tonal" class="mb-4">
            <v-card-title>Dit apparaat</v-card-title>
            <v-list class="bg-transparent">
                <v-list-item title="Naam" :subtitle="store.settings.device_name || '—'" />
                <v-list-item title="Rollen">
                    <template #subtitle>
                        <v-chip v-if="isHost()" size="x-small" class="mr-1">
                            Host
                        </v-chip>
                        <v-chip v-if="isClient()" size="x-small">
                            Client
                        </v-chip>
                        <span v-if="!isHost() && !isClient()">—</span>
                    </template>
                </v-list-item>
            </v-list>
        </v-card>

        <v-card variant="tonal" class="mb-4">
            <v-card-title>Identiteit</v-card-title>
            <v-card-text>
                <v-text-field
                    :model-value="store.publicKey"
                    label="Publieke sleutel"
                    readonly
                    density="comfortable"
                    hide-details
                />
                <p class="text-caption text-medium-emphasis mt-2">
                    De privésleutel verlaat dit apparaat nooit.
                </p>
            </v-card-text>
        </v-card>

        <v-card variant="tonal">
            <v-card-title>Over</v-card-title>
            <v-list class="bg-transparent">
                <v-list-item title="RivetLink" subtitle="Zero-trust remote control" />
                <v-list-item title="Versie" subtitle="0.1.0" />
            </v-list>
        </v-card>
    </v-container>
</template>
