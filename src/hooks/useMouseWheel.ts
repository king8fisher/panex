import { useEffect, useCallback } from 'react';
import { useStdin, useStdout } from 'ink';

interface MouseWheelEvent {
  type: 'wheel-up' | 'wheel-down';
  x: number;
  y: number;
}

interface UseMouseWheelOptions {
  enabled?: boolean;
  onWheel?: (event: MouseWheelEvent) => void;
}

/**
 * Hook to enable mouse wheel tracking in the terminal.
 *
 * Uses ANSI escape sequences for SGR extended mouse mode:
 * - \x1b[?1000h - Enable mouse button tracking
 * - \x1b[?1006h - Enable SGR extended mouse mode
 *
 * Mouse wheel events (SGR mode):
 * - Scroll up: \x1b[<64;X;YM (button 64 = wheel up)
 * - Scroll down: \x1b[<65;X;YM (button 65 = wheel down)
 */
export function useMouseWheel({ enabled = true, onWheel }: UseMouseWheelOptions = {}) {
  const { stdin, setRawMode } = useStdin();
  const { stdout } = useStdout();

  const handleData = useCallback((data: Buffer) => {
    const str = data.toString();

    // Parse SGR mouse events: \x1b[<button;x;yM or \x1b[<button;x;ym
    // Button 64 = wheel up, Button 65 = wheel down
    const sgrRegex = /\x1b\[<(\d+);(\d+);(\d+)([Mm])/g;
    let match;

    while ((match = sgrRegex.exec(str)) !== null) {
      const button = parseInt(match[1] ?? '0', 10);
      const x = parseInt(match[2] ?? '0', 10);
      const y = parseInt(match[3] ?? '0', 10);
      // M = press, m = release (we only care about press for wheel)
      const isPress = match[4] === 'M';

      if (isPress) {
        if (button === 64) {
          onWheel?.({ type: 'wheel-up', x, y });
        } else if (button === 65) {
          onWheel?.({ type: 'wheel-down', x, y });
        }
      }
    }
  }, [onWheel]);

  useEffect(() => {
    if (!enabled || !stdin || !stdout) return;

    // Enable mouse tracking
    // 1000h: X11 mouse button tracking
    // 1006h: SGR extended mouse mode (for proper coordinates)
    stdout.write('\x1b[?1000h\x1b[?1006h');

    // Ensure raw mode is enabled
    setRawMode?.(true);

    // Listen for mouse events
    stdin.on('data', handleData);

    return () => {
      stdin.off('data', handleData);
      // Disable mouse tracking on cleanup
      stdout.write('\x1b[?1000l\x1b[?1006l');
    };
  }, [enabled, stdin, stdout, setRawMode, handleData]);
}
