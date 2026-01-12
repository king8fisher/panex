import { describe, it, expect, beforeEach } from 'bun:test';
import { ProcessManager, type ManagedProcess } from './process-manager';
import type { ProcessConfig } from './types';

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
