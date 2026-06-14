// App routes. Hash history because the webview loads from a custom protocol —
// there's no dev server doing path-based routing.

import {
    createRouter, createWebHashHistory,
} from "vue-router";

import Connect from "./views/Connect.vue";
import ThisDevice from "./views/ThisDevice.vue";
import Relays from "./views/Relays.vue";
import Settings from "./views/Settings.vue";

export const router = createRouter({
    history: createWebHashHistory(),
    routes: [
        {
            path: "/",
            redirect: "/connect",
        },
        {
            path: "/connect",
            component: Connect,
            meta: {
                title: "Verbinden",
                icon: "mdi-monitor-share",
                role: "client",
            },
        },
        {
            path: "/device",
            component: ThisDevice,
            meta: {
                title: "Dit apparaat",
                icon: "mdi-laptop",
                role: "host",
            },
        },
        {
            path: "/relays",
            component: Relays,
            meta: {
                title: "Relays",
                icon: "mdi-server-network",
            },
        },
        {
            path: "/settings",
            component: Settings,
            meta: {
                title: "Instellingen",
                icon: "mdi-cog",
            },
        },
    ],
});
