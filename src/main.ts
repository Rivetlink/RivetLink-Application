import { createApp } from "vue";
import App from "./App.vue";
import { router } from "./router";
import { i18n } from "./i18n";

// Vuetify
import "vuetify/styles";
import { createVuetify } from "vuetify";
import * as components from "vuetify/components";
import * as directives from "vuetify/directives";
import "@mdi/font/css/materialdesignicons.css";

const vuetify = createVuetify({
	components,
	directives,
	theme: { defaultTheme: "dark" },
	icons: { defaultSet: "mdi" },
});

// Suppress the default WebKit right-click menu (Back / Forward / Stop / Reload /
// Inspect) app-wide — it doesn't belong in a desktop app. Reload lives in the
// native View menu (Cmd/Ctrl+R) instead. Applies to every window (each loads
// this entry). The viewer's canvas separately consumes right-click for control.
window.addEventListener("contextmenu", (e) => {
	e.preventDefault();
});

createApp(App).use(vuetify).use(router).use(i18n).mount("#app");
