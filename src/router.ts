// App routes. Hash history because the webview loads from a custom protocol —
// there's no dev server doing path-based routing. `titleKey` is an i18n key
// resolved in the shell.

import {
	createRouter, createWebHashHistory,
} from "vue-router";

import Connect from "./views/Connect.vue";
import ThisDevice from "./views/ThisDevice.vue";
import Settings from "./views/Settings.vue";
import Help from "./views/Help.vue";
import Viewer from "./views/Viewer.vue";
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
				icon: "mdi-monitor",
				role: "client",
			},
		},
		{
			path: "/device",
			component: ThisDevice,
			meta: {
				titleKey: "nav.device",
				icon: "mdi-lifebuoy",
				role: "host",
			},
		},
		{
			path: "/settings",
			component: Settings,
			meta: {
				titleKey: "nav.settings",
				icon: "mdi-cog-outline",
			},
		},
		{
			path: "/help",
			component: Help,
			meta: {
				titleKey: "nav.help",
				icon: "mdi-help-circle-outline",
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
			// Host "being viewed" badge (bottom-right): collapsible, shows who's
			// viewing. Rendered by window label in App.vue, not via this view; the
			// route exists only so the hash URL matches (no router warning).
			path: "/overlay-panel",
			component: OverlayPanel,
			meta: {
				overlay: true,
			},
		},
	],
});
