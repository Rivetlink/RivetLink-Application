// Flat ESLint config (ESLint 9) for the RivetLink desktop app.
//
// Stack: Vue 3 SFCs + TypeScript. Formatting is owned by @stylistic (no
// Prettier, so the two never fight). House style: tab indentation (one tab
// per level, displayed 4 wide), double quotes, semicolons, trailing commas on
// multiline, and — per project preference — object literals expanded with each
// property on its own line.

import js from "@eslint/js";
import globals from "globals";
import tseslint from "typescript-eslint";
import pluginVue from "eslint-plugin-vue";
import stylistic from "@stylistic/eslint-plugin";

export default tseslint.config(
  {
    ignores: [
      "dist",
      "node_modules",
      "src-tauri/target",
      "src-tauri/gen",
      // Generated / boilerplate ambient declarations (e.g. vite-env.d.ts).
      "**/*.d.ts",
    ],
  },

  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...pluginVue.configs["flat/recommended"],

  // Tell the Vue parser to use the TS parser for <script lang="ts">.
  {
    files: ["**/*.vue"],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
  },

  {
    files: ["**/*.{ts,vue}"],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "module",
      globals: {
        ...globals.browser,
      },
    },
    plugins: {
      "@stylistic": stylistic,
    },
    rules: {
      // ---- Object layout (the headline preference) -----------------------
      // Each property on its own line; no two properties sharing a line.
      "@stylistic/object-property-newline": [
        "error",
        {
          allowAllPropertiesOnSameLine: false,
        },
      ],
      // Require line breaks inside braces once an object spans multiple lines
      // (and keep the opening/closing braces consistent).
      "@stylistic/object-curly-newline": [
        "error",
        {
          multiline: true,
          consistent: true,
          minProperties: 2,
        },
      ],
      "@stylistic/object-curly-spacing": ["error", "always"],

      // ---- General formatting --------------------------------------------
      "@stylistic/indent": ["error", "tab"],
      "@stylistic/quotes": ["error", "double", { avoidEscape: true }],
      "@stylistic/semi": ["error", "always"],
      "@stylistic/comma-dangle": ["error", "always-multiline"],
      "@stylistic/comma-spacing": ["error", { before: false, after: true }],
      "@stylistic/arrow-parens": ["error", "always"],
      "@stylistic/eol-last": ["error", "always"],
      "@stylistic/no-multiple-empty-lines": ["error", { max: 1, maxBOF: 0, maxEOF: 0 }],
      "@stylistic/no-trailing-spaces": "error",
      "@stylistic/space-before-blocks": "error",
      "@stylistic/keyword-spacing": "error",

      // ---- Correctness / hygiene -----------------------------------------
      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
        },
      ],
      "@typescript-eslint/no-explicit-any": "warn",

      // ---- Vue ------------------------------------------------------------
      // <script setup> components don't need a multi-word name.
      "vue/multi-word-component-names": "off",
      // SFC block order: markup first, then logic, then styles.
      "vue/block-order": [
        "error",
        {
          order: ["template", "script", "style"],
        },
      ],
      // Write components PascalCase in templates: <VCard>, not <v-card>.
      // registeredComponentsOnly:false so it also covers Vuetify's globally
      // registered v-* components (and RouterView/RouterLink).
      "vue/component-name-in-template-casing": [
        "error",
        "PascalCase",
        {
          registeredComponentsOnly: false,
        },
      ],
      // Tab indentation in both template and <script> blocks.
      "vue/html-indent": ["error", "tab"],
      // baseIndent: 1 — content inside <script> starts one tab in, not at
      // column 0, so the block body sits under the opening tag.
      "vue/script-indent": [
        "error",
        "tab",
        {
          baseIndent: 1,
          switchCase: 1,
        },
      ],
      "vue/max-attributes-per-line": [
        "error",
        {
          singleline: { max: 3 },
          multiline: { max: 1 },
        },
      ],
    },
  },

  // Inside .vue files, `vue/script-indent` owns <script> indentation, so the
  // core stylistic `indent` rule must stand down to avoid a double-report.
  {
    files: ["**/*.vue"],
    rules: {
      "@stylistic/indent": "off",
    },
  },
);
