import js from "@eslint/js";
import globals from "globals";
import tseslint from "typescript-eslint";
import {defineConfig} from "eslint/config";

import react from "eslint-plugin-react";
import reactHooks from "eslint-plugin-react-hooks";

import type {ConfigObject} from "@eslint/core";

const config: ConfigObject[] = defineConfig(
    {
        ignores: ["dist"],
    },
    js.configs.recommended,
    ...tseslint.configs.recommended,
    react.configs.flat.recommended,
    reactHooks.configs.flat.recommended,
    {
        languageOptions: {
            ...react.configs.flat.recommended.languageOptions,
            globals: globals.browser,
        },
        rules: {
            "react/react-in-jsx-scope": "off",
            "react/jsx-key": ["error", {checkFragmentShorthand: true}],

            "no-duplicate-imports": "error",
            "no-else-return": "warn",
            "no-empty": ["warn", {allowEmptyCatch: true}],
            "no-iterator": "error",
            "no-lonely-if": "error",
            "no-unneeded-ternary": "error",
            "no-useless-concat": "warn",
            "no-var": "warn",
            "prefer-template": "warn",
        },
        settings: {
            react: {
                version: "detect",
            },
        },
    },
);

export default config;
