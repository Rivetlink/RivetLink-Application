<script setup lang="ts">
import {
    computed, onMounted, onUnmounted, ref,
} from "vue";
import { useRoute } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { router } from "./router";
import {
    loadSettings, store, isHost, isClient,
} from "./store";
import Onboarding from "./views/Onboarding.vue";

const route = useRoute();
const drawer = ref(true);

// Developer console is off by default; toggle it with Ctrl/Cmd+Shift+I or F12.
function onKeydown(e: KeyboardEvent) {
    const toggleCombo = (e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === "i";
    if (toggleCombo || e.key === "F12") {
        e.preventDefault();
        invoke("toggle_devtools").catch(() => { /* devtools unavailable */ });
    }
}

onMounted(async () => {
    window.addEventListener("keydown", onKeydown);
    await loadSettings();
});

onUnmounted(() => {
    window.removeEventListener("keydown", onKeydown);
});

// Nav items, filtered by the roles the user enabled during onboarding.
const navItems = computed(() => {
    return router.options.routes
        .filter((r) => r.meta && r.meta.title)
        .filter((r) => {
            const role = r.meta?.role;
            if (role === "client") return isClient();
            if (role === "host") return isHost();
            return true;
        })
        .map((r) => ({
            path: r.path,
            title: r.meta?.title as string,
            icon: r.meta?.icon as string,
        }));
});

const currentTitle = computed(() => (route.meta?.title as string) ?? "RivetLink");

function onSetupDone() {
    // Settings are refreshed inside completeSetup; just leave the gate.
    router.replace("/");
}
</script>

<template>
    <v-app>
        <!-- Loading -->
        <v-main v-if="!store.loaded">
            <v-container class="fill-height d-flex align-center justify-center">
                <v-progress-circular indeterminate color="primary" size="40" />
            </v-container>
        </v-main>

        <!-- First-run onboarding -->
        <v-main v-else-if="!store.settings.setup_complete">
            <Onboarding @done="onSetupDone" />
        </v-main>

        <!-- Main shell -->
        <template v-else>
            <v-navigation-drawer v-model="drawer" color="grey-darken-4">
                <div class="pa-4 d-flex align-center">
                    <v-icon icon="mdi-shield-lock-outline" color="primary" class="mr-2" />
                    <span class="text-h6">RivetLink</span>
                </div>
                <v-divider />
                <v-list nav density="comfortable">
                    <v-list-item
                        v-for="item in navItems"
                        :key="item.path"
                        :to="item.path"
                        :prepend-icon="item.icon"
                        :title="item.title"
                    />
                </v-list>
                <template #append>
                    <div class="pa-3 text-caption text-medium-emphasis">
                        {{ store.settings.device_name || "Naamloos apparaat" }}
                    </div>
                </template>
            </v-navigation-drawer>

            <v-app-bar flat color="grey-darken-4" density="comfortable">
                <v-app-bar-nav-icon @click="drawer = !drawer" />
                <v-app-bar-title>{{ currentTitle }}</v-app-bar-title>
            </v-app-bar>

            <v-main>
                <router-view />
            </v-main>
        </template>
    </v-app>
</template>
