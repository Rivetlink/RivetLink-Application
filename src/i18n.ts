// Internationalisation. English is the default; the chosen language is
// remembered in localStorage. Add a locale by dropping a JSON file in
// ./locales and registering it in `messages` + `SUPPORTED`.

import { createI18n } from "vue-i18n";
import en from "./locales/en_en.json";
import nl from "./locales/nl_nl.json";

export const SUPPORTED = [
    {
        code: "en",
        label: "English",
    },
    {
        code: "nl",
        label: "Nederlands",
    },
];

const saved = localStorage.getItem("locale");
const initial = saved && SUPPORTED.some((l) => l.code === saved) ? saved : "en";

export const i18n = createI18n({
    legacy: false,
    locale: initial,
    fallbackLocale: "en",
    globalInjection: true,
    messages: {
        en,
        nl,
    },
});

type Locale = typeof i18n.global.locale.value;

export function setLocale(code: string): void {
    i18n.global.locale.value = code as Locale;
    localStorage.setItem("locale", code);
}
