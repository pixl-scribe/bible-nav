import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import prettierRecommended from 'eslint-plugin-prettier/recommended';

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
  }
);
