import { useState, useEffect, useCallback, useRef } from 'react';
import { ProcessManager, type ManagedProcess } from '../process-manager';
import type { PanexConfig, ProcessStatus } from '../types';

export interface UseProcessManagerResult {
  processManager: ProcessManager;
  processes: Map<string, ManagedProcess>;
  names: string[];
  getOutput: (name: string) => string;
  getStatus: (name: string) => ProcessStatus;
  restart: (name: string) => void;
  restartAll: () => void;
  kill: (name: string) => void;
  killAll: () => void;
  write: (name: string, data: string) => void;
  resize: (name: string, cols: number, rows: number) => void;
}

export function useProcessManager(config: PanexConfig): UseProcessManagerResult {
  const [, forceUpdate] = useState({});
  const processManagerRef = useRef<ProcessManager | null>(null);

  if (!processManagerRef.current) {
    processManagerRef.current = new ProcessManager(config.procs);
  }

  const pm = processManagerRef.current;

  useEffect(() => {
    const update = () => forceUpdate({});
    pm.on('output', update);
    pm.on('started', update);
    pm.on('exit', update);
    pm.on('error', update);

    pm.startAll();

    return () => {
      pm.removeAllListeners();
      pm.killAll();
    };
  }, [pm]);

  const getOutput = useCallback((name: string) => pm.getOutput(name), [pm]);

  const getStatus = useCallback((name: string): ProcessStatus => {
    const proc = pm.getProcess(name);
    return proc?.status ?? 'stopped';
  }, [pm]);

  const restart = useCallback((name: string) => pm.restart(name), [pm]);
  const restartAll = useCallback(() => pm.restartAll(), [pm]);
  const kill = useCallback((name: string) => pm.kill(name), [pm]);
  const killAll = useCallback(() => pm.killAll(), [pm]);
  const write = useCallback((name: string, data: string) => pm.write(name, data), [pm]);
  const resize = useCallback((name: string, cols: number, rows: number) => pm.resize(name, cols, rows), [pm]);

  return {
    processManager: pm,
    processes: new Map(pm.getNames().map(n => [n, pm.getProcess(n)!])),
    names: pm.getNames(),
    getOutput,
    getStatus,
    restart,
    restartAll,
    kill,
    killAll,
    write,
    resize,
  };
}
