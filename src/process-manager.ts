import * as pty from 'node-pty';
import { EventEmitter } from 'events';
import type { ProcessConfig } from './types';

export interface ManagedProcess {
  name: string;
  config: ProcessConfig;
  pty: pty.IPty | null;
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

    try {
      const ptyProcess = pty.spawn(shell, args, {
        name: 'xterm-256color',
        cols: 120,
        rows: 30,
        cwd,
        env: env as Record<string, string>,
      });

      const managed: ManagedProcess = {
        name,
        config,
        pty: ptyProcess,
        status: 'running',
        output: [],
        exitCode: null,
      };

      this.processes.set(name, managed);

      ptyProcess.onData((data) => {
        managed.output.push(data);
        // Trim output if too long
        if (managed.output.length > this.maxOutputLines) {
          managed.output = managed.output.slice(-this.maxOutputLines);
        }
        this.emit('output', name, data);
      });

      ptyProcess.onExit(({ exitCode }) => {
        managed.status = exitCode === 0 ? 'stopped' : 'error';
        managed.exitCode = exitCode;
        managed.pty = null;
        this.emit('exit', name, exitCode);

        // Auto-restart if configured
        if (config.autoRestart && exitCode !== 0) {
          setTimeout(() => {
            this.start(name, config);
          }, 1000);
        }
      });

      this.emit('started', name);
    } catch (error) {
      const managed: ManagedProcess = {
        name,
        config,
        pty: null,
        status: 'error',
        output: [`Error starting process: ${error}`],
        exitCode: -1,
      };
      this.processes.set(name, managed);
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
