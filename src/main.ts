import { createApp } from "vue";
import App from "./App.vue";
import { router } from "./router";
import { i18n } from "./i18n";
import { reportDecoders } from "./video";

// Vuetify
import "vuetify/styles";
import { createVuetify } from "vuetify";
import * as components from "vuetify/components";
import * as directives from "vuetify/directives";
import "@mdi/font/css/materialdesignicons.css";
// Bundled, not fetched: the CSP only allows fonts from the app itself.
import "@fontsource-variable/inter";
import "./theme.css";

const vuetify = createVuetify({
	components,
	directives,
	theme: {
		defaultTheme: "rivet",
		themes: {
			rivet: {
				dark: true,
				colors: {
					background: "#0F1115",
					surface: "#171A20",
					"surface-bright": "#1F2430",
					"surface-light": "#1F2430",
					"surface-variant": "#1F2430",
					"on-surface-variant": "#E8EAED",
					"on-background": "#E8EAED",
					"on-surface": "#E8EAED",
					primary: "#3F5FE0",
					"on-primary": "#FFFFFF",
					secondary: "#8EA6FF",
					success: "#3FD08A",
					info: "#8EA6FF",
					warning: "#F2B04C",
					error: "#FF6B6B",
				},
				variables: { "medium-emphasis-opacity": 0.72 },
			},
		},
	},
	defaults: {
		VBtn: { rounded: "lg" },
		VTextField: { variant: "outlined" },
		VTextarea: { variant: "outlined" },
		VSelect: { variant: "outlined" },
		VSwitch: {
			color: "primary",
			inset: true,
		},
	},
	icons: { defaultSet: "mdi" },
});

// Suppress the default WebKit right-click menu (Back / Forward / Stop / Reload /
// Inspect) app-wide — it doesn't belong in a desktop app. Reload lives in the
// native View menu (Cmd/Ctrl+R) instead. Applies to every window (each loads
// this entry). The viewer's canvas separately consumes right-click for control.
window.addEventListener("contextmenu", (e) => {
	e.preventDefault();
});

reportDecoders();

createApp(App).use(vuetify).use(router).use(i18n).mount("#app");
