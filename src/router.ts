// App routes. Hash history because the webview loads from a custom protocol —
// there's no dev server doing path-based routing. `titleKey` is an i18n key
// resolved in the shell.

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
				titleKey: "nav.connect",
				icon: "mdi-monitor-share",
				role: "client",
			},
		},
		{
			path: "/device",
			component: ThisDevice,
			meta: {
				titleKey: "nav.device",
				icon: "mdi-laptop",
				role: "host",
			},
		},
		{
			path: "/relays",
			component: Relays,
			meta: {
				titleKey: "nav.relays",
				icon: "mdi-server-network",
			},
		},
		{
			path: "/settings",
			component: Settings,
			meta: {
				titleKey: "nav.settings",
				icon: "mdi-cog",
			},
		},
	],
});
