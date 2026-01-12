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
  };
}
