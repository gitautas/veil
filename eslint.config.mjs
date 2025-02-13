// @ts-check

import eslint from "@eslint/js";
import tseslint from "typescript-eslint";
import json from "@eslint/json";
import eslintConfigPrettier from "eslint-config-prettier";

export default tseslint.config(
  eslint.configs.recommended,
  tseslint.configs.recommended,
  eslintConfigPrettier,
  {
    ignores: ["**/*.js", "**/dist"],
    plugins: {
      json,
    },
    languageOptions: {
      parserOptions: {
        projectService: {
          allowDefaultProject: ["eslint.config.mjs", "tsconfig.json",],
          loadTypeScriptPlugins: true,
        },
      },
    },
  },
);
