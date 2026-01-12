#!/usr/bin/env node

import { Command } from 'commander';
import type { PanexConfig } from './types';
import { createTUI } from './tui';

const program = new Command();

program
  .name('panex')
  .description('Terminal UI for running multiple processes in parallel')
  .version('0.1.0')
  .argument('<commands...>', 'Commands to run in parallel')
  .option('-n, --names <names>', 'Comma-separated names for each process')
  .action(async (commands: string[], options: { names?: string }) => {
    const names = options.names?.split(',') ?? commands.map((_, i) => `proc${i + 1}`);
    const config: PanexConfig = {
      procs: Object.fromEntries(
        commands.map((cmd, i) => [
          names[i] ?? `proc${i + 1}`,
          { shell: cmd },
        ])
      ),
    };

    await createTUI(config);
  });

program.parse();