import { EventEmitter } from 'events';
import type { ProcessConfig } from './types';

interface PtyHandle {
  write(data: string): void;
  resize(cols: number, rows: number): void;
  kill(): void;
}

export interface ManagedProcess {
  name: string;
  config: ProcessConfig;
  pty: PtyHandle | null;
  status: 'running' | 'stopped' | 'error';
  output: string[];
  exitCode: number | null;
}

export class ProcessManager extends EventEmitter {
  private processes: Map<string, ManagedProcess> = new Map();
  private maxOutputLines = 10000;

  constructor(private procs: Record<string, ProcessConfig>) {
    super();
  }

  async startAll(): Promise<void> {
    for (const [name, config] of Object.entries(this.procs)) {
      await this.start(name, config);
    }
  }

  async start(name: string, config: ProcessConfig): Promise<void> {
    const existing = this.processes.get(name);
    if (existing?.pty) {
      existing.pty.kill();
    }

    const shell = process.platform === 'win32' ? 'powershell.exe' : 'bash';
    const args = config.shell
      ? ['-c', config.shell]
      : config.cmd
        ? ['-c', config.cmd.join(' ')]
        : [];

    const cwd = config.cwd ?? process.cwd();
    const env = { ...process.env, ...config.env };

    const managed: ManagedProcess = {
      name,
      config,
      pty: null,
      status: 'running',
      output: [],
      exitCode: null,
    };

    this.processes.set(name, managed);

    try {
      const proc = Bun.spawn([shell, ...args], {
        cwd,
        env: env as Record<string, string>,
        terminal: {
          cols: 120,
          rows: 30,
          data: (_terminal: unknown, data: Uint8Array) => {
            const str = new TextDecoder().decode(data);
            managed.output.push(str);
            if (managed.output.length > this.maxOutputLines) {
              managed.output = managed.output.slice(-this.maxOutputLines);
            }
            this.emit('output', name, str);
          },
        },
      });

      managed.pty = {
        write: (data: string) => proc.terminal?.write(data),
        resize: (cols: number, rows: number) => proc.terminal?.resize(cols, rows),
        kill: () => proc.kill(),
      };

      // Handle exit
      proc.exited.then((exitCode) => {
        managed.status = exitCode === 0 ? 'stopped' : 'error';
        managed.exitCode = exitCode;
        managed.pty = null;
        this.emit('exit', name, exitCode);

        if (managed.config.autoRestart && exitCode !== 0) {
          setTimeout(() => this.start(name, managed.config), 1000);
        }
      });

      this.emit('started', name);
    } catch (error) {
      managed.status = 'error';
      managed.output = [`Error starting process: ${error}`];
      managed.exitCode = -1;
      this.emit('error', name, error);
    }
  }

  restart(name: string): void {
    const proc = this.processes.get(name);
    if (proc) {
      if (proc.pty) {
        proc.pty.kill();
      }
      proc.output = [];
      this.start(name, proc.config);
    }
  }

  restartAll(): void {
    for (const name of this.processes.keys()) {
      this.restart(name);
    }
  }

  kill(name: string): void {
    const proc = this.processes.get(name);
    if (proc?.pty) {
      proc.pty.kill();
    }
  }

  killAll(): void {
    for (const proc of this.processes.values()) {
      if (proc.pty) {
        proc.pty.kill();
      }
    }
  }

  write(name: string, data: string): void {
    const proc = this.processes.get(name);
    if (proc?.pty) {
      proc.pty.write(data);
    }
  }

  resize(name: string, cols: number, rows: number): void {
    const proc = this.processes.get(name);
    if (proc?.pty) {
      proc.pty.resize(cols, rows);
    }
  }

  getProcess(name: string): ManagedProcess | undefined {
    return this.processes.get(name);
  }

  getProcesses(): ManagedProcess[] {
    return Array.from(this.processes.values());
  }

  getNames(): string[] {
    return Array.from(this.processes.keys());
  }

  getOutput(name: string): string {
    return this.processes.get(name)?.output.join('') ?? '';
  }
}
