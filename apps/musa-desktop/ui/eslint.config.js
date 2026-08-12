// @ts-check
import eslint from "@eslint/js";
import globals from "globals";
import svelte from "eslint-plugin-svelte";
import ts from "typescript-eslint";

export default ts.config(
  {
    ignores: ["dist/", "node_modules/", "test-results/", "playwright-report/"],
  },
  eslint.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs["flat/recommended"],
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
  },
  {
    files: ["**/*.svelte", "**/*.svelte.ts", "**/*.svelte.js"],
    languageOptions: {
      parserOptions: {
        parser: ts.parser,
      },
    },
  },
  {
    rules: {
      // `_` is the project's throwaway binding (e.g. `{#each … as _, i}`).
      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
          caughtErrorsIgnorePattern: "^_",
        },
      ],
    },
  },
);
