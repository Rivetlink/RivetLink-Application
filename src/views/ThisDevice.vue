<script setup lang="ts">
import {
    onMounted, ref,
} from "vue";
import {
    loadPublicKey, store,
} from "../store";

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
                        {{ store.settings.device_name || "Naamloos apparaat" }}
                    </div>
                    <div class="text-caption text-medium-emphasis">
                        Dit apparaat
                    </div>
                </div>
                <v-spacer />
                <v-chip color="grey" size="small" variant="flat">
                    <v-icon start icon="mdi-circle" size="x-small" /> offline
                </v-chip>
            </v-card-text>
        </v-card>

        <v-card variant="tonal" class="mb-4">
            <v-card-title>Identiteit</v-card-title>
            <v-card-subtitle>
                Geef deze sleutel aan een support-client om die vooraf te vertrouwen (TOFU).
            </v-card-subtitle>
            <v-card-text>
                <v-text-field
                    :model-value="store.publicKey"
                    label="Publieke sleutel"
                    readonly
                    density="comfortable"
                    append-inner-icon="mdi-content-copy"
                    @click:append-inner="copyKey"
                />
                <v-fade-transition>
                    <span v-if="copied" class="text-caption text-success">Gekopieerd!</span>
                </v-fade-transition>
            </v-card-text>
        </v-card>

        <v-card variant="tonal">
            <v-card-title class="d-flex align-center">
                Host-agent
                <v-chip class="ml-2" size="x-small" color="amber">
                    binnenkort
                </v-chip>
            </v-card-title>
            <v-card-text>
                <p class="text-body-2 text-medium-emphasis mb-2">
                    Om deze pc echt bedienbaar te maken draait er straks een host-agent die:
                </p>
                <v-list density="compact" class="bg-transparent">
                    <v-list-item prepend-icon="mdi-shield-check" title="Toestemming vraagt bij elke verbinding (consent-dialog)" />
                    <v-list-item prepend-icon="mdi-account-key" title="Vertrouwde clients beheert (toevoegen / intrekken)" />
                    <v-list-item prepend-icon="mdi-monitor-screenshot" title="Scherm deelt en input ontvangt" />
                </v-list>
                <p class="text-caption text-medium-emphasis mt-2">
                    Deze integratie (rivetlink-agent) is de volgende milestone.
                </p>
            </v-card-text>
        </v-card>
    </v-container>
</template>
