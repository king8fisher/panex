import blessed from 'blessed';
import type { PanexConfig } from './types';
import { ProcessManager } from './process-manager';

export async function createTUI(config: PanexConfig): Promise<void> {
  const processManager = new ProcessManager(config.procs);

  // Create screen
  const screen = blessed.screen({
    smartCSR: true,
    title: 'panex',
    fullUnicode: true,
  });

  // Process list (left panel)
  const processList = blessed.list({
    parent: screen,
    label: ' PROCESSES ',
    top: 0,
    left: 0,
    width: '20%',
    height: '100%-1',
    border: { type: 'line' },
    style: {
      border: { fg: 'blue' },
      selected: { bg: 'blue', fg: 'white' },
      item: { fg: 'white' },
    },
    keys: true,
    vi: true,
    mouse: config.settings?.mouse ?? true,
    scrollbar: {
      ch: '│',
      style: { bg: 'blue' },
    },
  });

  // Output panel (right panel)
  const outputBox = blessed.box({
    parent: screen,
    label: ' OUTPUT ',
    top: 0,
    left: '20%',
    width: '80%',
    height: '100%-1',
    border: { type: 'line' },
    style: {
      border: { fg: 'green' },
    },
    scrollable: true,
    alwaysScroll: true,
    scrollbar: {
      ch: '│',
      style: { bg: 'green' },
    },
    mouse: config.settings?.mouse ?? true,
    keys: true,
    vi: true,
  });

  // Status bar
  const statusBar = blessed.box({
    parent: screen,
    bottom: 0,
    left: 0,
    width: '100%',
    height: 1,
    style: {
      bg: 'blue',
      fg: 'white',
    },
    content: ' [↑↓/jk] select  [Enter] focus  [r] restart  [a] restart all  [x] kill  [q] quit  [?] help ',
  });

  // Help popup
  const helpBox = blessed.box({
    parent: screen,
    top: 'center',
    left: 'center',
    width: '60%',
    height: '60%',
    label: ' Help ',
    border: { type: 'line' },
    style: {
      border: { fg: 'yellow' },
      bg: 'black',
    },
    hidden: true,
    content: `
  Keyboard Shortcuts
  ──────────────────

  Navigation
    ↑/↓ or j/k    Navigate process list
    g/G           Scroll to top/bottom of output
    PgUp/PgDn     Scroll output

  Process Control
    Enter         Focus process (interactive mode)
    Esc           Exit focus mode
    r             Restart selected process
    a             Restart all processes
    x             Kill selected process

  General
    ?             Toggle this help
    q             Quit panex

  Press any key to close this help...
    `,
  });

  // State
  let selectedIndex = 0;
  let focusMode = false;
  const processNames = Object.keys(config.procs);

  // Update process list UI
  function updateProcessList() {
    const items = processNames.map((name, i) => {
      const proc = processManager.getProcess(name);
      const status = proc?.status === 'running' ? '●' : proc?.status === 'error' ? '✗' : '○';
      const prefix = i === selectedIndex ? '▶' : ' ';
      return `${prefix} ${name} ${status}`;
    });
    processList.setItems(items);
    processList.select(selectedIndex);
    screen.render();
  }

  // Update output panel
  function updateOutput() {
    const name = processNames[selectedIndex];
    if (name) {
      outputBox.setLabel(` OUTPUT: ${name} `);
      const output = processManager.getOutput(name);
      outputBox.setContent(output);
      outputBox.setScrollPerc(100); // Scroll to bottom
    }
    screen.render();
  }

  // Event handlers
  processManager.on('output', (name: string) => {
    if (name === processNames[selectedIndex]) {
      updateOutput();
    }
  });

  processManager.on('started', () => {
    updateProcessList();
  });

  processManager.on('exit', () => {
    updateProcessList();
  });

  // Keyboard handling
  screen.key(['q', 'C-c'], () => {
    processManager.killAll();
    process.exit(0);
  });

  screen.key(['?'], () => {
    helpBox.toggle();
    screen.render();
  });

  screen.key(['escape'], () => {
    if (!helpBox.hidden) {
      helpBox.hide();
      screen.render();
      return;
    }
    if (focusMode) {
      focusMode = false;
      statusBar.setContent(' [↑↓/jk] select  [Enter] focus  [r] restart  [a] restart all  [x] kill  [q] quit  [?] help ');
      screen.render();
    }
  });

  helpBox.key(['escape', 'q', '?', 'enter', 'space'], () => {
    helpBox.hide();
    screen.render();
  });

  screen.key(['up', 'k'], () => {
    if (focusMode || !helpBox.hidden) return;
    selectedIndex = Math.max(0, selectedIndex - 1);
    updateProcessList();
    updateOutput();
  });

  screen.key(['down', 'j'], () => {
    if (focusMode || !helpBox.hidden) return;
    selectedIndex = Math.min(processNames.length - 1, selectedIndex + 1);
    updateProcessList();
    updateOutput();
  });

  screen.key(['enter'], () => {
    if (!helpBox.hidden) {
      helpBox.hide();
      screen.render();
      return;
    }
    focusMode = !focusMode;
    const name = processNames[selectedIndex];
    if (focusMode && name) {
      statusBar.setContent(` FOCUS: ${name} - Type to interact, [Esc] to exit focus mode `);
    } else {
      statusBar.setContent(' [↑↓/jk] select  [Enter] focus  [r] restart  [a] restart all  [x] kill  [q] quit  [?] help ');
    }
    screen.render();
  });

  screen.key(['r'], () => {
    if (focusMode || !helpBox.hidden) return;
    const name = processNames[selectedIndex];
    if (name) {
      processManager.restart(name);
    }
  });

  screen.key(['a'], () => {
    if (focusMode || !helpBox.hidden) return;
    processManager.restartAll();
  });

  screen.key(['x'], () => {
    if (focusMode || !helpBox.hidden) return;
    const name = processNames[selectedIndex];
    if (name) {
      processManager.kill(name);
    }
  });

  screen.key(['g'], () => {
    if (focusMode || !helpBox.hidden) return;
    outputBox.setScrollPerc(0);
    screen.render();
  });

  screen.key(['S-g'], () => {
    if (focusMode || !helpBox.hidden) return;
    outputBox.setScrollPerc(100);
    screen.render();
  });

  // Forward input in focus mode
  screen.on('keypress', (ch: string, key: { full: string }) => {
    if (focusMode && ch) {
      const name = processNames[selectedIndex];
      if (name) {
        processManager.write(name, ch);
      }
    }
  });

  // Handle resize
  screen.on('resize', () => {
    const name = processNames[selectedIndex];
    if (name) {
      const cols = Math.floor((screen.width as number) * 0.8) - 2;
      const rows = (screen.height as number) - 3;
      processManager.resize(name, cols, rows);
    }
  });

  // Initial render
  updateProcessList();
  updateOutput();
  processList.focus();

  // Start all processes
  await processManager.startAll();
  updateProcessList();
  updateOutput();

  screen.render();
}