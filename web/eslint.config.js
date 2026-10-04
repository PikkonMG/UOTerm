import { defineConfig, globalIgnores } from 'eslint/config';
import tseslint from 'typescript-eslint';

export default defineConfig(
  // Build output and the files the build makes are not ours to lint.
  globalIgnores(['dist', 'src/wasm', 'src/fonts']),
  tseslint.configs.recommended,
);
