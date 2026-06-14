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

// Step 2 — server (only the HTTP address; the app derives the rest)
const relayName = ref("Mijn server");
const relayUrl = ref("");
const relayValid = computed(
    () => relayName.value.trim().length > 0 && relayUrl.value.trim().startsWith("http"),
);

// Step 3 — device name
const deviceName = ref("");

const error = ref<string | null>(null);
const busy = ref(false);

async function finish() {
    error.value = null;
    busy.value = true;
    try {
        await addRelay(relayName.value, relayUrl.value);
        const roles: string[] = [];
        if (roleHost.value) roles.push("host");
        if (roleClient.value) roles.push("client");
        await completeSetup(deviceName.value.trim() || "Mijn computer", roles);
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
                    Welkom bij RivetLink
                </h1>
                <p class="text-medium-emphasis">
                    Hiermee kijk je veilig op afstand mee op een computer — of laat je een
                    ander meekijken op die van jou. We stellen deze computer even in.
                    Dit hoeft maar één keer.
                </p>
            </div>

            <v-window v-model="step">
                <!-- Step 1: role -->
                <v-window-item :value="1">
                    <v-card variant="tonal">
                        <v-card-title>Wat ga je met deze computer doen?</v-card-title>
                        <v-card-subtitle>Kies wat past. Je mag ook allebei kiezen.</v-card-subtitle>
                        <v-card-text>
                            <v-list-item
                                class="rounded mb-2 border"
                                :active="roleHost"
                                @click="roleHost = !roleHost"
                            >
                                <template #prepend>
                                    <v-checkbox-btn :model-value="roleHost" />
                                </template>
                                <v-list-item-title>Ik wil hulp krijgen op deze computer</v-list-item-title>
                                <v-list-item-subtitle class="text-wrap">
                                    Iemand anders mag dan op dit scherm meekijken — maar alleen
                                    als jij op dat moment zelf op "Ja" klikt. Kies dit als jíj
                                    geholpen wilt worden.
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
                                <v-list-item-title>Ik wil iemand anders helpen</v-list-item-title>
                                <v-list-item-subtitle class="text-wrap">
                                    Jij kijkt mee op de computer van een ander om te helpen.
                                    Kies dit als jíj degene bent die helpt.
                                </v-list-item-subtitle>
                            </v-list-item>

                            <p class="text-caption text-medium-emphasis mt-3">
                                Je kunt dit later altijd nog aanpassen.
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
                                Volgende
                            </v-btn>
                        </v-card-actions>
                    </v-card>
                </v-window-item>

                <!-- Step 2: server -->
                <v-window-item :value="2">
                    <v-card variant="tonal">
                        <v-card-title>Met welke server verbind je?</v-card-title>
                        <v-card-subtitle class="text-wrap">
                            RivetLink gebruikt een server als tussenpersoon om de verbinding te
                            leggen. Je hebt het adres nodig — meestal krijg je dat van je
                            beheerder, of je gebruikt je eigen server.
                        </v-card-subtitle>
                        <v-card-text>
                            <v-text-field
                                v-model="relayName"
                                label="Naam"
                                hint="Een naam die jij herkent, bijvoorbeeld 'Werk'."
                                persistent-hint
                                prepend-inner-icon="mdi-tag-outline"
                                density="comfortable"
                                class="mb-2"
                            />
                            <v-text-field
                                v-model="relayUrl"
                                label="Serveradres"
                                placeholder="https://relay.mijnbedrijf.nl"
                                hint="Het webadres van de server. De app regelt de rest zelf."
                                persistent-hint
                                prepend-inner-icon="mdi-web"
                                density="comfortable"
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
                        <v-card-title>Hoe heet deze computer?</v-card-title>
                        <v-card-subtitle>Zo ziet de ander meteen welke computer van jou is.</v-card-subtitle>
                        <v-card-text>
                            <v-text-field
                                v-model="deviceName"
                                label="Naam van deze computer"
                                placeholder="bijvoorbeeld: Laptop van Jan"
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
                                Klaar, ga verder
                            </v-btn>
                        </v-card-actions>
                    </v-card>
                </v-window-item>
            </v-window>
        </v-card>
    </v-container>
</template>
