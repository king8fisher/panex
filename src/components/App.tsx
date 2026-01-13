import { useState, useEffect, useRef, useCallback } from 'react';
import { Box, useApp, useInput, useStdin, useStdout } from 'ink';
import type { PanexConfig } from '../types';
import { useProcessManager } from '../hooks/useProcessManager';
import { useFocusMode } from '../hooks/useFocusMode';
import { useMouseWheel } from '../hooks/useMouseWheel';
import { ProcessList, ProcessListRef, PROCESS_LIST_WIDTH } from './ProcessList';
import { OutputPanel, OutputPanelRef } from './OutputPanel';
import { StatusBar } from './StatusBar';
import { HelpPopup } from './HelpPopup';

interface AppProps {
  config: PanexConfig;
}

export function App({ config }: AppProps) {
  const { exit } = useApp();
  const { stdout } = useStdout();
  const { setRawMode } = useStdin();
  const [selected, setSelected] = useState(0);
  const [showHelp, setShowHelp] = useState(false);
  const { focusMode, enterFocus, exitFocus } = useFocusMode();

  // Refs to control scrolling
  const outputRef = useRef<OutputPanelRef>(null);
  const processListRef = useRef<ProcessListRef>(null);

  // Track auto-scroll state per process
  const [autoScroll, setAutoScroll] = useState<Record<string, boolean>>({});

  // Track scroll position per process (use ref to avoid closure issues)
  const scrollPositionsRef = useRef<Record<string, number>>({});
  const pendingRestoreRef = useRef<string | null>(null);

  const {
    names,
    getOutput,
    getStatus,
    restart,
    restartAll,
    kill,
    killAll,
    write,
    resize,
  } = useProcessManager(config);

  // Check if Shift-Tab is disabled for a process
  const isShiftTabDisabled = (name: string): boolean => {
    const setting = config.settings?.noShiftTab;
    if (setting === true) return true;
    if (Array.isArray(setting)) return setting.includes(name);
    return false;
  };

  // Calculate max panel height: terminal rows - status bar (1)
  const maxPanelHeight = stdout ? stdout.rows - 1 : undefined;

  // Resize on terminal resize
  useEffect(() => {
    const name = names[selected];
    if (name && stdout) {
      const cols = Math.floor(stdout.columns * 0.8) - 2;
      const rows = stdout.rows - 3;
      resize(name, cols, rows);
    }
  }, [stdout?.columns, stdout?.rows, selected, names, resize]);

  // Initialize auto-scroll for new processes
  useEffect(() => {
    setAutoScroll(prev => {
      const next = { ...prev };
      let changed = false;
      for (const name of names) {
        if (next[name] === undefined) {
          next[name] = true;
          changed = true;
        }
      }
      return changed ? next : prev;
    });
  }, [names]);

  const selectedName = names[selected] ?? '';
  const output = selectedName ? getOutput(selectedName) : '';
  const currentAutoScroll = selectedName ? (autoScroll[selectedName] ?? true) : true;

  // Helper to save current scroll position before switching
  const saveCurrentScrollPosition = useCallback(() => {
    if (selectedName && outputRef.current) {
      scrollPositionsRef.current[selectedName] = outputRef.current.getScrollOffset();
    }
  }, [selectedName]);

  // Custom setSelected that saves scroll position first
  const handleSetSelected = useCallback((newSelected: number | ((s: number) => number)) => {
    saveCurrentScrollPosition();
    setSelected(prev => {
      const next = typeof newSelected === 'function' ? newSelected(prev) : newSelected;
      if (next !== prev) {
        // Mark that we need to restore scroll for the new process
        pendingRestoreRef.current = names[next] ?? null;
      }
      return next;
    });
  }, [saveCurrentScrollPosition, names]);

  // Restore scroll position after render when process changes
  useEffect(() => {
    const restoreName = pendingRestoreRef.current;
    if (restoreName && restoreName === selectedName) {
      pendingRestoreRef.current = null;
      const savedOffset = scrollPositionsRef.current[restoreName];
      // Only restore if we have a saved position AND auto-scroll is disabled
      if (savedOffset !== undefined && savedOffset > 0 && !autoScroll[restoreName]) {
        // Use setImmediate to ensure render is complete
        setImmediate(() => {
          if (outputRef.current) {
            // Only restore if content actually needs scrolling
            const contentHeight = outputRef.current.getContentHeight();
            const viewportHeight = outputRef.current.getViewportHeight();
            if (contentHeight > viewportHeight) {
              const currentOffset = outputRef.current.getScrollOffset();
              if (currentOffset !== savedOffset) {
                outputRef.current.scrollBy(savedOffset - currentOffset);
              }
            }
          }
        });
      }
    }
  }, [selectedName, autoScroll]);

  // Handle auto-scroll state changes from OutputPanel
  const handleAutoScrollChange = useCallback((enabled: boolean) => {
    if (selectedName) {
      setAutoScroll(prev => ({ ...prev, [selectedName]: enabled }));
    }
  }, [selectedName]);

  // Handle mouse wheel events
  const handleWheel = useCallback((event: { type: 'wheel-up' | 'wheel-down'; x: number; y: number; }) => {
    const delta = event.type === 'wheel-up' ? -3 : 3;

    if (outputRef.current) {
      outputRef.current.scrollBy(delta);
      // Disable auto-scroll when user scrolls up
      if (event.type === 'wheel-up' && selectedName) {
        setAutoScroll(prev => ({ ...prev, [selectedName]: false }));
      }
    }
  }, [selectedName]);

  // Handle mouse click events
  const handleClick = useCallback((event: { type: 'click'; x: number; y: number; }) => {
    if (event.x <= PROCESS_LIST_WIDTH) {
      // Click on process list - exit focus mode, select process if clicked
      if (focusMode) {
        exitFocus();
      }
      const clickedIndex = event.y - 2; // Adjust for border
      if (clickedIndex >= 0 && clickedIndex < names.length) {
        handleSetSelected(clickedIndex);
      }
    } else {
      // Click on output panel - enter focus mode
      if (!focusMode) {
        enterFocus();
      }
    }
  }, [names.length, focusMode, enterFocus, exitFocus, handleSetSelected]);

  // Enable mouse tracking
  useMouseWheel({
    enabled: !showHelp, // Disable when help is shown
    onWheel: handleWheel,
    onClick: handleClick,
  });

  useInput((input, key) => {
    // Handle help popup
    if (showHelp) {
      setShowHelp(false);
      return;
    }

    // Quit (Ctrl+C always works, 'q' only in normal mode)
    if (key.ctrl && input === 'c') {
      killAll();
      // Restore terminal: disable raw mode, move cursor to bottom, clear below, disable mouse, show cursor
      setRawMode(false);
      const rows = stdout?.rows ?? 999;
      stdout?.write(`\x1b[${rows};1H\x1b[J\x1b[?1000l\x1b[?1006l\x1b[?25h\x1b[0m\n`);
      exit();
      process.exit(0);
    }

    // Focus mode input handling (before normal mode keys like 'q')
    if (focusMode) {
      const name = names[selected];
      if (!name) return;

      // Exit focus
      if (key.escape) {
        exitFocus();
        return;
      }

      // Shift-Tab exit (unless disabled)
      if (key.shift && key.tab && !isShiftTabDisabled(name)) {
        exitFocus();
        return;
      }

      // Forward Enter
      if (key.return) {
        write(name, '\r');
        return;
      }

      // Forward arrow keys
      if (key.upArrow) {
        write(name, '\x1b[A');
        return;
      }
      if (key.downArrow) {
        write(name, '\x1b[B');
        return;
      }
      if (key.leftArrow) {
        write(name, '\x1b[D');
        return;
      }
      if (key.rightArrow) {
        write(name, '\x1b[C');
        return;
      }

      // Forward regular input (filter out mouse escape sequences)
      if (input && !key.ctrl && !key.meta) {
        // Remove SGR mouse sequences like \x1b[<64;45;5M or [<0;12;7M
        const filtered = input.replace(/\x1b?\[<\d+;\d+;\d+[Mm]/g, '');
        if (filtered) {
          write(name, filtered);
        }
      }
      return;
    }

    // Normal mode

    // Quit with 'q' (only in normal mode)
    if (input === 'q') {
      killAll();
      setRawMode(false);
      const rows = stdout?.rows ?? 999;
      stdout?.write(`\x1b[${rows};1H\x1b[J\x1b[?1000l\x1b[?1006l\x1b[?25h\x1b[0m\n`);
      exit();
      process.exit(0);
    }

    // Help
    if (input === '?') {
      setShowHelp(true);
      return;
    }

    // Navigation
    if (key.upArrow || input === 'k') {
      handleSetSelected(s => Math.max(s - 1, 0));
      return;
    }
    if (key.downArrow || input === 'j') {
      handleSetSelected(s => Math.min(s + 1, names.length - 1));
      return;
    }

    // Enter focus mode
    if (key.return || key.tab) {
      enterFocus();
      return;
    }

    // Process control
    if (input === 'r') {
      const name = names[selected];
      if (name) restart(name);
      return;
    }
    if (input === 'A') {
      restartAll();
      return;
    }
    if (input === 'x') {
      const name = names[selected];
      if (name) kill(name);
      return;
    }

    // Output scrolling
    if (input === 'g') {
      outputRef.current?.scrollToTop();
      if (selectedName) {
        setAutoScroll(prev => ({ ...prev, [selectedName]: false }));
      }
      return;
    }
    if (input === 'G') {
      outputRef.current?.scrollToBottom();
      if (selectedName) {
        setAutoScroll(prev => ({ ...prev, [selectedName]: true }));
      }
      return;
    }
    if (key.pageUp) {
      const pageSize = outputRef.current?.getViewportHeight() ?? 10;
      outputRef.current?.scrollBy(-pageSize);
      if (selectedName) {
        setAutoScroll(prev => ({ ...prev, [selectedName]: false }));
      }
      return;
    }
    if (key.pageDown) {
      const pageSize = outputRef.current?.getViewportHeight() ?? 10;
      outputRef.current?.scrollBy(pageSize);
      return;
    }
  });

  const showShiftTabHint = selectedName ? !isShiftTabDisabled(selectedName) : true;

  return (
    <Box flexDirection="column" height={maxPanelHeight}>
      <Box flexDirection="row" flexGrow={1} height={maxPanelHeight ? maxPanelHeight - 1 : undefined}>
        <ProcessList
          ref={processListRef}
          names={names}
          selected={selected}
          getStatus={getStatus}
          active={!focusMode}
          height={maxPanelHeight ? maxPanelHeight - 1 : undefined}
        />
        <OutputPanel
          ref={outputRef}
          name={selectedName}
          output={output}
          active={focusMode}
          height={maxPanelHeight ? maxPanelHeight - 1 : undefined}
          autoScroll={currentAutoScroll}
          onAutoScrollChange={handleAutoScrollChange}
        />
      </Box>
      <StatusBar
        focusMode={focusMode}
        processName={selectedName}
        showShiftTabHint={showShiftTabHint}
      />
      <HelpPopup visible={showHelp} />
    </Box>
  );
}
