# Panex Internals

## Terminal Emulation

Panex implements a terminal emulator to properly display output from child processes. This document covers key implementation details.

### Architecture

```
┌──────────────┐     ┌─────────────┐     ┌────────────────┐
│ Child Process│ ──► │     PTY     │ ──► │ TerminalBuffer │
│  (glow, etc) │ ◄── │             │ ◄── │                │
└──────────────┘     └─────────────┘     └────────────────┘
                          ▲                      │
                          │                      ▼
                     responses            ┌───────────────┐
                                          │   Renderer    │
                                          └───────────────┘
```

### VTE Parser Persistence

The VTE (Virtual Terminal Emulator) parser must be persisted across `write()` calls. ANSI escape sequences can span multiple read chunks:

```rust
// WRONG: Parser state lost between writes
pub fn write(&mut self, data: &[u8]) {
    let mut parser = vte::Parser::new();  // New parser each time!
    for byte in data {
        parser.advance(self, *byte);
    }
}

// CORRECT: Parser persisted in struct
pub struct TerminalBuffer {
    state: TerminalState,
    parser: vte::Parser,  // Persisted
}

pub fn write(&mut self, data: &[u8]) {
    for byte in data {
        self.parser.advance(&mut self.state, *byte);
    }
}
```

### Terminal Query/Response Protocol

Modern terminal applications query terminal capabilities on startup. Without responses, apps timeout (typically 5-8 seconds).

#### Device Attributes (DA)

```
Query:    \x1b[c  or  \x1b[0c
Response: \x1b[?1;2c  (VT100 with Advanced Video Option)
```

Apps like `glow`, `bat`, and other Charmbracelet tools use this to detect terminal capabilities.

#### Cursor Position Report (CPR)

```
Query:    \x1b[6n
Response: \x1b[{row};{col}R  (1-indexed)
```

Used by apps that need to know cursor position for layout calculations.

#### Device Status Report (DSR)

```
Query:    \x1b[5n
Response: \x1b[0n  ("OK")
```

#### Implementation

Since `TerminalState` (which implements `vte::Perform`) doesn't have direct PTY access, we use a response queue:

1. `csi_dispatch` queues responses in `pending_responses: Vec<Vec<u8>>`
2. After `write()`, caller drains the queue via `take_pending_responses()`
3. `ProcessManager` writes responses back to PTY

```rust
// In TerminalState::csi_dispatch
'c' => {
    // Device Attributes - respond as VT100 with AVO
    self.pending_responses.push(b"\x1b[?1;2c".to_vec());
}

// In ProcessManager::handle_output
let responses = process.buffer.take_pending_responses();
if let Some(ref pty) = process.pty {
    for response in responses {
        let _ = pty.write(&response);
    }
}
```

### Supported Escape Sequences

#### Cursor Movement
| Sequence        | Name | Action                  |
| --------------- | ---- | ----------------------- |
| `\x1b[{n}A`     | CUU  | Cursor up n rows        |
| `\x1b[{n}B`     | CUD  | Cursor down n rows      |
| `\x1b[{n}C`     | CUF  | Cursor forward n cols   |
| `\x1b[{n}D`     | CUB  | Cursor back n cols      |
| `\x1b[{n}E`     | CNL  | Cursor to next line     |
| `\x1b[{n}F`     | CPL  | Cursor to previous line |
| `\x1b[{n}G`     | CHA  | Cursor to column n      |
| `\x1b[{r};{c}H` | CUP  | Cursor to row r, col c  |

#### Display Control
| Sequence  | Name | Action                   |
| --------- | ---- | ------------------------ |
| `\x1b[J`  | ED   | Erase from cursor to end |
| `\x1b[2J` | ED   | Erase entire display     |
| `\x1b[K`  | EL   | Erase from cursor to EOL |
| `\x1b[2K` | EL   | Erase entire line        |

#### Character Control
| Byte   | Name | Action                  |
| ------ | ---- | ----------------------- |
| `0x08` | BS   | Backspace               |
| `0x09` | HT   | Tab                     |
| `0x0A` | LF   | Line feed (newline)     |
| `0x0D` | CR   | Carriage return (col 0) |

#### SGR (Select Graphic Rendition)
Full support for:
- Modifiers: bold, dim, italic, underline, blink, reverse, hidden, strikethrough
- Standard colors (30-37, 40-47)
- Bright colors (90-97, 100-107)
- 256-color mode (`\x1b[38;5;{n}m`)
- 24-bit RGB (`\x1b[38;2;{r};{g};{b}m`)

### Scrollback Buffer

- Max lines: 10,000 (`MAX_SCROLLBACK`)
- Storage: `VecDeque<Line>` for efficient front/back operations
- Auto-scroll follows cursor position, not buffer end

## UI Layout & PTY Sizing

### Screen Layout

```
┌────────────────────┬─┬──────────────────────────────────────┐
│                    │ │                                      │
│   Process List     │D│          Output Panel                │
│   (20 cols)        │E│    (width - 21 cols)                 │
│                    │L│                                      │
│                    │ │                                      │
├────────────────────┴─┴──────────────────────────────────────┤
│                      Status Bar (1 row)                     │
└─────────────────────────────────────────────────────────────┘
```

- Process list: fixed 20 columns
- Delimiter: 1 column (empty space)
- Output panel: remaining width (`total_width - 21`)
- Status bar: 1 row at bottom

### PTY Size Calculation

The PTY must be told the exact dimensions of the output panel, not the full terminal size. TUI apps (lazygit, btm, gitui) query terminal size via SIGWINCH and draw accordingly.

```rust
// Initial size
let output_cols = terminal_width - 21;  // process list + delimiter
let output_rows = terminal_height - 1;   // status bar

// On resize event
pm.resize(cols - 21, rows - 1);
```

**Common bug**: Passing full terminal dimensions causes TUI apps to draw content that gets clipped or wrapped incorrectly.

### Focus Indication

Panel focus indicated via selected process item highlighting:
- **Normal mode** (process list focused): Blue background on selected item
- **Focus mode** (output panel focused): Dark gray background on selected item

No borders on panels - saves space and reduces visual clutter.

## Auto-Scroll Behavior

### The Problem with TUI Apps

Full-screen TUI apps (lazygit, gitui, btm) exhibited a "jumping" behavior where the display would shift one row down, then back up on each redraw. The top row would disappear momentarily.

**Root cause**: Our terminal emulator wraps immediately when cursor reaches the last column, setting `cursor_row = visible`. Real terminals use "pending wrap" state where cursor stays at the last column until the next character.

```
Real terminal:          Our implementation:
─────────────────       ─────────────────
Print at (22, 58)       Print at (22, 58)
cursor = (22, 58)       cursor = (23, 0)  ← wrapped immediately
pending_wrap = true
```

### The Auto-Scroll Logic

Original (buggy) logic:
```rust
if cursor_row >= visible {
    scroll_offset = cursor_row - visible + 1;
}
```

When `cursor_row == visible` (e.g., 23 == 23):
- Scroll offset becomes 1
- Row 0 is hidden
- On next redraw, app moves cursor back, offset resets to 0
- **Result**: Flicker/jumping

Fixed logic:
```rust
if cursor_row > visible {  // Changed from >=
    scroll_offset = cursor_row - (visible - 1);
}
```

When `cursor_row == visible`:
- Condition is false, no scroll
- Row 0 stays visible
- **Result**: Stable display for TUI apps

### Why This Works

For TUI apps:
- They draw within a fixed viewport (rows 0 to visible-1)
- Cursor at row `visible` is transient (wrap state, no actual content)
- Not scrolling keeps their intended viewport intact

For scrollback (cat, long output):
- Real content on row `visible+1` triggers scroll
- Still follows cursor when there's actual content below viewport

### Pin Feature

Users can disable auto-scroll ("pin") by:
- Scrolling up manually
- Pressing `g` to go to top

Pin indicator (📌) appears in process list when pinned. Re-enables when:
- User scrolls to bottom
- Presses `G` to go to bottom

## Mouse Handling

Click detection for process selection:
```rust
if event.column < 20 {  // Within process list
    let index = event.row as usize;  // Direct mapping, no border offset
    if index < process_count {
        selected_index = index;
    }
}
```

Scroll wheel works on output panel regardless of click position.
