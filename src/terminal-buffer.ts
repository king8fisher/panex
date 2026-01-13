import { Terminal } from '@xterm/headless';

/**
 * A terminal buffer that properly interprets ANSI escape sequences including:
 * - Cursor movement (\x1b[A up, \x1b[B down, \x1b[C right, \x1b[D left)
 * - Cursor positioning (\x1b[H, \x1b[row;colH)
 * - Line clearing (\x1b[K erase to end, \x1b[2K erase line)
 * - Screen clearing (\x1b[2J clear screen)
 * - Carriage return (\r) for in-place updates like progress bars
 */
export class TerminalBuffer {
  private terminal: Terminal;
  private rows: number;
  private cols: number;

  constructor(cols = 200, rows = 500) {
    this.rows = rows;
    this.cols = cols;
    this.terminal = new Terminal({
      cols,
      rows,
      scrollback: 10000,
      allowProposedApi: true,
    });
  }

  /**
   * Write data to the terminal buffer.
   * The terminal will interpret all ANSI escape sequences.
   */
  write(data: string): void {
    this.terminal.write(data);
  }

  /**
   * Get the current terminal buffer content as an array of lines.
   * Only returns lines that have content (not all 500 rows).
   */
  getLines(): string[] {
    const buffer = this.terminal.buffer.active;
    const lines: string[] = [];

    // Get actual content length (baseY is lines scrolled off + cursorY + 1)
    const contentLength = buffer.baseY + buffer.cursorY + 1;

    for (let i = 0; i < contentLength; i++) {
      const line = buffer.getLine(i);
      if (line) {
        lines.push(line.translateToString(true)); // trim trailing whitespace
      }
    }

    // Remove trailing empty lines
    while (lines.length > 0 && lines[lines.length - 1] === '') {
      lines.pop();
    }

    return lines;
  }

  /**
   * Get the terminal content as a single string with newlines.
   */
  toString(): string {
    return this.getLines().join('\n');
  }

  /**
   * Clear the terminal buffer.
   */
  clear(): void {
    this.terminal.reset();
  }

  /**
   * Resize the terminal.
   */
  resize(cols: number, rows: number): void {
    this.cols = cols;
    this.rows = rows;
    this.terminal.resize(cols, rows);
  }

  /**
   * Dispose of the terminal to free resources.
   */
  dispose(): void {
    this.terminal.dispose();
  }
}
