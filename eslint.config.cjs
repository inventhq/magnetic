/* Flat config for ESLint v9 */
const tsParser = require("@typescript-eslint/parser");
const tsPlugin = require("@typescript-eslint/eslint-plugin");
const importPlugin = require("eslint-plugin-import");

/** @type {import('eslint').Linter.FlatConfig[]} */
module.exports = [
  {
    ignores: [
      "**/node_modules/**",
      "**/dist/**",
      "**/*.d.ts"
    ],
  },
  {
    files: ["**/*.ts", "**/*.tsx"],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        tsconfigRootDir: __dirname,
        project: ["./tsconfig.base.json"]
      }
    },
    plugins: {
      "@typescript-eslint": tsPlugin,
      import: importPlugin
    },
    settings: {
      "import/resolver": {
        // Use the dedicated TS resolver
        typescript: {
          alwaysTryTypes: true,
          project: ["./tsconfig.base.json"]
        }
      }
    },
    rules: {
      // Discipline rails
      "max-lines": ["error", { max: 400, skipBlankLines: true, skipComments: true }],

      // Dependency guardrails
      "import/no-extraneous-dependencies": ["error", { devDependencies: true }]
    }
  },
  // UI firewall for sdk-web (no framework deps in P1 kernel/impl)
  {
    files: ["packages/sdk-web/**/*.{ts,tsx}"],
    rules: {
      "no-restricted-imports": ["error", {
        "paths": ["react", "react-dom", "vue", "solid-js", "svelte"]
      }]
    }
  },
  // Tests: relax dep checks so vitest globals/imports don't trigger errors
  {
    files: ["**/*.spec.ts", "**/*.spec.tsx", "**/__tests__/**/*.{ts,tsx}"],
    rules: {
      "import/no-extraneous-dependencies": ["off"]
    }
  }
];
