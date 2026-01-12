// Check for Bun runtime - must be at top before any Bun APIs are used
if (typeof Bun === 'undefined') {
  console.error('Error: panex requires Bun runtime.');
  console.error('Please run with bunx instead of npx:');
  console.error('\tbunx panex "cmd1" "cmd2"');
  process.exit(1);
}

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
  .option('--no-shift-tab [names]', 'Disable Shift-Tab to exit focus (all or comma-separated process names)')
  .action(async (commands: string[], options: { names?: string; shiftTab?: boolean | string }) => {
    const rawNames = options.names?.split(',') ?? commands.map((_, i) => `proc${i + 1}`);

    // Ensure unique names by adding suffix for duplicates
    const usedNames = new Map<string, number>();
    const names = rawNames.map((name, i) => {
      const baseName = name || `proc${i + 1}`;
      const count = usedNames.get(baseName) ?? 0;
      usedNames.set(baseName, count + 1);
      return count === 0 ? baseName : `${baseName}-${count + 1}`;
    });

    // Parse noShiftTab option
    let noShiftTab: boolean | string[] | undefined;
    if (options.shiftTab === false) {
      noShiftTab = true; // --no-shift-tab without args disables for all
    } else if (typeof options.shiftTab === 'string') {
      noShiftTab = options.shiftTab.split(','); // --no-shift-tab api,mobile
    }

    const config: PanexConfig = {
      procs: Object.fromEntries(
        commands.map((cmd, i) => [names[i], { shell: cmd }])
      ),
      settings: {
        noShiftTab,
      },
    };

    await createTUI(config);
  });

program.parse();