<script setup lang="ts">
import {
    computed, onMounted, onUnmounted, ref,
} from "vue";
import { useRoute } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import {
    listen, type UnlistenFn,
} from "@tauri-apps/api/event";
import { router } from "./router";
import {
    loadSettings, store, isHost, isClient,
} from "./store";
import {
    updateState, checkForUpdates, openReleasesPage,
} from "./updates";
import Onboarding from "./views/Onboarding.vue";

const route = useRoute();
const { t } = useI18n();
const drawer = ref(true);
let unlistenMenu: UnlistenFn | null = null;

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
    // The native "RivetLink -> Check for Updates" menu item fires this event.
    unlistenMenu = await listen("menu://check-updates", () => checkForUpdates());
    await loadSettings();
});

onUnmounted(() => {
    window.removeEventListener("keydown", onKeydown);
    unlistenMenu?.();
});

// Nav items, filtered by the roles the user enabled during onboarding.
const navItems = computed(() => {
    return router.options.routes
        .filter((r) => r.meta && r.meta.titleKey)
        .filter((r) => {
            const role = r.meta?.role;
            if (role === "client") return isClient();
            if (role === "host") return isHost();
            return true;
        })
        .map((r) => ({
            path: r.path,
            titleKey: r.meta?.titleKey as string,
            icon: r.meta?.icon as string,
        }));
});

const currentTitle = computed(() => {
    const key = route.meta?.titleKey as string | undefined;
    return key ? t(key) : "RivetLink";
});
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
            <Onboarding @done="router.replace('/')" />
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
                        :title="t(item.titleKey)"
                    />
                </v-list>
                <template #append>
                    <div class="pa-3 text-caption text-medium-emphasis">
                        {{ store.settings.device_name || t("app.unnamedDevice") }}
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

        <!-- Check for updates (from the RivetLink native menu) -->
        <v-dialog v-model="updateState.dialog" max-width="420">
            <v-card>
                <v-card-title>{{ t("updates.title") }}</v-card-title>
                <v-card-text>
                    <div class="d-flex align-center mb-2">
                        <v-icon icon="mdi-shield-lock-outline" color="primary" class="mr-2" />
                        <span>RivetLink {{ updateState.current }}</span>
                    </div>

                    <div v-if="updateState.checking" class="d-flex align-center">
                        <v-progress-circular indeterminate size="18" class="mr-2" />
                        {{ t("updates.checking") }}
                    </div>
                    <v-alert
                        v-else-if="updateState.status === 'available'"
                        type="info"
                        variant="tonal"
                        density="compact"
                    >
                        {{ t("updates.available", { version: updateState.latest }) }}
                    </v-alert>
                    <v-alert
                        v-else-if="updateState.status === 'uptodate'"
                        type="success"
                        variant="tonal"
                        density="compact"
                    >
                        {{ t("updates.upToDate") }}
                    </v-alert>
                    <v-alert
                        v-else-if="updateState.status === 'error'"
                        type="warning"
                        variant="tonal"
                        density="compact"
                    >
                        {{ t("updates.error") }}
                    </v-alert>
                </v-card-text>
                <v-card-actions>
                    <v-btn
                        v-if="updateState.status === 'available'"
                        color="primary"
                        variant="text"
                        prepend-icon="mdi-open-in-new"
                        @click="openReleasesPage"
                    >
                        {{ t("updates.download") }}
                    </v-btn>
                    <v-spacer />
                    <v-btn variant="text" @click="updateState.dialog = false">
                        {{ t("common.close") }}
                    </v-btn>
                </v-card-actions>
            </v-card>
        </v-dialog>
    </v-app>
</template>
