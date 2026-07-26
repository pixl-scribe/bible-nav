import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import prettierRecommended from 'eslint-plugin-prettier/recommended';
import svelte from 'eslint-plugin-svelte';
import svelteParser from 'svelte-eslint-parser';

export default tseslint.config(
  js.configs.recommended,
  ...tseslint.configs.recommended,
  prettierRecommended, // Must be last to override other configs
  {
    ignores: [
      'src-tauri/',
      'node_modules/',
      'dist/',
      'static/',
      'build/',
      '.idea/',
      '.svelte-kit',
      '.vscode/',
    ],
  },
  ...svelte.configs['flat/recommended'],
  {
    files: ['**/*.svelte', '**/*.svelte.ts'],
    languageOptions: {
      parser: svelteParser,
      // If using TypeScript inside Svelte
      parserOptions: {
        parser: '@typescript-eslint/parser',
      },
    },
    rules: {
      'no-undef': 'off',
    },
  }
);
