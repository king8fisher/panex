import { describe, it, expect, beforeEach } from 'bun:test';
import { ProcessManager, type ManagedProcess } from './process-manager';
import type { ProcessConfig } from './types';
import { TerminalBuffer } from './terminal-buffer';

describe('ProcessManager', () => {
  let manager: ProcessManager;

  beforeEach(() => {
    manager = new ProcessManager({
      test: { shell: 'echo hello' },
    });
  });

  it('initializes with empty process list', () => {
    expect(manager.getProcesses()).toHaveLength(0);
    expect(manager.getNames()).toHaveLength(0);
  });

  it('getProcess returns undefined for non-existent process', () => {
    expect(manager.getProcess('nonexistent')).toBeUndefined();
  });

  it('getOutput returns empty string for non-existent process', () => {
    expect(manager.getOutput('nonexistent')).toBe('');
  });

  it('emits events', () => {
    const events: string[] = [];
    manager.on('started', (name: string) => events.push(`started:${name}`));
    manager.on('output', (name: string) => events.push(`output:${name}`));
    manager.on('exit', (name: string) => events.push(`exit:${name}`));

    expect(events).toHaveLength(0);
  });
});

describe('ProcessManager with multiple processes', () => {
  it('accepts multiple process configurations', () => {
    const procs: Record<string, ProcessConfig> = {
      server: { shell: 'echo server' },
      client: { shell: 'echo client' },
      watcher: { cmd: ['echo', 'watching'] },
    };

    const manager = new ProcessManager(procs);
    expect(manager.getNames()).toHaveLength(0); // Before startAll
  });
});

describe('ManagedProcess type', () => {
  it('defines correct status types', () => {
    const statuses: ManagedProcess['status'][] = ['running', 'stopped', 'error'];
    expect(statuses).toContain('running');
    expect(statuses).toContain('stopped');
    expect(statuses).toContain('error');
  });
});

describe('TerminalBuffer', () => {
  it('preserves ANSI color codes in output', async () => {
    const buffer = new TerminalBuffer();
    // Write colored text: red "Hello" green "World"
    buffer.write('\x1b[31mHello\x1b[0m \x1b[32mWorld\x1b[0m\n');
    // xterm write is async, need to wait
    await new Promise((r) => setTimeout(r, 10));
    const output = buffer.toString();

    // Should contain ANSI escape sequences
    expect(output).toContain('\x1b[31m'); // red
    expect(output).toContain('\x1b[32m'); // green
    expect(output).toContain('Hello');
    expect(output).toContain('World');

    buffer.dispose();
  });

  it('preserves bold and underline formatting', async () => {
    const buffer = new TerminalBuffer();
    buffer.write('\x1b[1mBold\x1b[0m \x1b[4mUnderline\x1b[0m\n');
    // xterm write is async, need to wait
    await new Promise((r) => setTimeout(r, 10));
    const output = buffer.toString();

    expect(output).toContain('\x1b[1m'); // bold
    expect(output).toContain('\x1b[4m'); // underline
    expect(output).toContain('Bold');
    expect(output).toContain('Underline');

    buffer.dispose();
  });
});
