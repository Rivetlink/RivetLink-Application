// App routes. Hash history because the webview loads from a custom protocol —
// there's no dev server doing path-based routing. `titleKey` is an i18n key
// resolved in the shell.

import {
	createRouter, createWebHashHistory,
} from "vue-router";

import Connect from "./views/Connect.vue";
import ThisDevice from "./views/ThisDevice.vue";
import Relays from "./views/Relays.vue";
import Resources from "./views/Resources.vue";
import Settings from "./views/Settings.vue";
import Viewer from "./views/Viewer.vue";
import OverlayBorder from "./views/OverlayBorder.vue";
import OverlayPanel from "./views/OverlayPanel.vue";

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
			path: "/resources",
			component: Resources,
			meta: {
				titleKey: "nav.resources",
				icon: "mdi-chart-areaspline",
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
		{
			// Standalone live-stream window — no nav shell (meta.bare).
			path: "/viewer",
			component: Viewer,
			meta: {
				bare: true,
			},
		},
		{
			// Host "being viewed" overlay: a click-through red border covering the
			// screen. `meta.overlay` renders it outside the Vuetify shell so the
			// window stays transparent.
			path: "/overlay-border",
			component: OverlayBorder,
			meta: {
				overlay: true,
			},
		},
		{
			// Host overlay badge (bottom-right): collapsible, shows who's viewing.
			path: "/overlay-panel",
			component: OverlayPanel,
			meta: {
				overlay: true,
			},
		},
	],
});
