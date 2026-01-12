import { describe, it, expect, beforeAll, afterAll } from 'bun:test';
import { $ } from 'bun';
import { readFile, mkdtemp, rm } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';

describe('CLI Integration', () => {
  beforeAll(async () => {
    await $`bun run build`;
  });

  it('dist/cli.js has single shebang on line 1', async () => {
    const content = await readFile('dist/cli.js', 'utf-8');
    const lines = content.split('\n');
    expect(lines[0]).toBe('#!/usr/bin/env bun');
    expect(lines[1]).not.toBe('#!/usr/bin/env bun');
  });

  it('dist/index.js has no shebang', async () => {
    const content = await readFile('dist/index.js', 'utf-8');
    expect(content.startsWith('#!/')).toBe(false);
  });

  it('CLI --help works', async () => {
    const result = await $`bun dist/cli.js --help`.text();
    expect(result).toContain('panex');
    expect(result).toContain('Commands to run in parallel');
  });

  it('CLI --version works', async () => {
    const result = await $`bun dist/cli.js --version`.text();
    expect(result.trim()).toMatch(/^\d+\.\d+\.\d+$/);
  });
});

describe('CLI Package Integration (simulates bunx)', () => {
  let tempDir: string;
  let tarball: string;

  beforeAll(async () => {
    // Pack the package (npm pack outputs the tarball filename)
    const packOutput = await $`npm pack --silent`.text();
    tarball = packOutput.trim();

    // Create temp directory and install the tarball
    tempDir = await mkdtemp(join(tmpdir(), 'panex-test-'));
    await $`cd ${tempDir} && bun init -y`.quiet();
    await $`cd ${tempDir} && bun add ${join(process.cwd(), tarball)}`.quiet();
  });

  afterAll(async () => {
    if (tempDir) {
      await rm(tempDir, { recursive: true, force: true });
    }
    if (tarball) {
      await rm(tarball, { force: true });
    }
  });

  it('installed CLI has correct shebang', async () => {
    const cliPath = join(tempDir, 'node_modules', 'panex', 'dist', 'cli.js');
    const content = await readFile(cliPath, 'utf-8');
    const lines = content.split('\n');
    expect(lines[0]).toBe('#!/usr/bin/env bun');
    expect(lines[1]).not.toBe('#!/usr/bin/env bun');
  });

  it('bunx panex --help works from installed package', async () => {
    const result = await $`cd ${tempDir} && bunx panex --help`.text();
    expect(result).toContain('panex');
    expect(result).toContain('Commands to run in parallel');
  });

  it('bunx panex --version works from installed package', async () => {
    const result = await $`cd ${tempDir} && bunx panex --version`.text();
    expect(result.trim()).toMatch(/^\d+\.\d+\.\d+$/);
  });
});
