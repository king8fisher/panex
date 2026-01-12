import { defineConfig } from 'tsup';

export default defineConfig([
  {
    entry: { index: 'src/index.ts' },
    format: ['esm'],
    dts: true,
    clean: true,
    sourcemap: true,
    target: 'node18',
    external: ['node-pty'],
  },
  {
    entry: { cli: 'src/cli.ts' },
    format: ['esm'],
    clean: false,
    sourcemap: true,
    target: 'node18',
    external: ['node-pty'],
    banner: {
      js: '#!/usr/bin/env node',
    },
  },
]);
