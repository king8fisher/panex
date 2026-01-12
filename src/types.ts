export interface ProcessConfig {
  cmd?: string[];
  shell?: string;
  cwd?: string;
  env?: Record<string, string>;
  autoRestart?: boolean;
}

export interface PanexConfig {
  procs: Record<string, ProcessConfig>;
  settings?: {
    mouse?: boolean;
    /** Disable Shift-Tab for all processes (true) or specific process names (string[]) */
    noShiftTab?: boolean | string[];
  };
}
