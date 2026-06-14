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

createApp(App).use(vuetify).use(router).use(i18n).mount("#app");
