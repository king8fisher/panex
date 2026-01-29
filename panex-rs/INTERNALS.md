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
