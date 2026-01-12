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
    const rawNames = options.names?.split(',') ?? commands.map((_, i) => `proc${i + 1}`);

    // Ensure unique names by adding suffix for duplicates
    const usedNames = new Map<string, number>();
    const names = rawNames.map((name, i) => {
      const baseName = name || `proc${i + 1}`;
      const count = usedNames.get(baseName) ?? 0;
      usedNames.set(baseName, count + 1);
      return count === 0 ? baseName : `${baseName}-${count + 1}`;
    });

    const config: PanexConfig = {
      procs: Object.fromEntries(
        commands.map((cmd, i) => [names[i], { shell: cmd }])
      ),
    };

    await createTUI(config);
  });

program.parse();