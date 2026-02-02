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

#### Window Size Query (XTWINOPS)

```
Query:    \x1b[18t
Response: \x1b[8;{rows};{cols}t
```

TUI apps like `lazygit`, `gitui`, and others query terminal size via this escape sequence. Without a response, they fall back to 80x24 defaults. This is separate from SIGWINCH which handles resize events - this query is for initial size detection.

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

### Resize Debouncing

Terminal resize events flood in during window dragging. Without debouncing, each event triggers PTY resize which causes TUI apps to redraw. For CPU-intensive processes this can cause lag.

Solution: Store pending resize dimensions, only apply after 50ms of no new resize events:

```rust
let mut pending_resize: Option<(u16, u16)> = None;
let mut resize_deadline: Option<Instant> = None;
const RESIZE_DEBOUNCE: Duration = Duration::from_millis(50);

// On size change detection
if last_size != Some(current_size) {
    pending_resize = Some(current_size);
    resize_deadline = Some(Instant::now() + RESIZE_DEBOUNCE);
    last_size = Some(current_size);
}

// Apply after deadline passes
if let (Some((cols, rows)), Some(deadline)) = (pending_resize, resize_deadline) {
    if Instant::now() >= deadline {
        pm.resize(cols.saturating_sub(21), rows.saturating_sub(1));
        pending_resize = None;
        resize_deadline = None;
    }
}
```

Each new resize event resets the deadline, so resize only applies when dragging stops.

### Focus Indication

Panel focus indicated via selected process item highlighting:
- **Browse mode** (process list focused): Blue background on selected item
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
    scroll_offset = cursor_row - visible;  // Show content, not empty cursor line
}
```

Two fixes:
1. `>` instead of `>=`: When cursor is exactly at `visible`, don't scroll (transient wrap state)
2. `cursor_row - visible` instead of `cursor_row - visible + 1`: Don't show the empty line where cursor sits after newline

**Result**: Stable display for TUI apps + no wasted viewport line on empty cursor row

### Why This Works

For TUI apps:
- They draw within a fixed viewport (rows 0 to visible-1)
- Cursor at row `visible` is transient (wrap state, no actual content)
- Not scrolling keeps their intended viewport intact

For scrollback (cat, date loop, long output):
- After outputting "line\n", cursor is on empty row N+1
- Old formula showed row N+1 (empty), wasting one line of viewport
- New formula: `scroll_offset = cursor_row - visible` keeps cursor just below viewport
- Shows rows [scroll_offset, scroll_offset + visible - 1], all with actual content

### Content Line Count

The buffer tracks all lines including empty ones created by cursor movement. For scroll calculations, we use `content_line_count()` which excludes trailing empty lines:

```rust
pub fn content_line_count(&self) -> usize {
    let mut count = self.lines.len();
    while count > 0 && self.lines[count - 1].cells.is_empty() {
        count -= 1;
    }
    count.max(1)
}
```

Used in:
- `scroll_down()`: max_scroll calculation
- `OutputPanel::render()`: total_lines for display bounds

This ensures manual scrolling and rendering never show trailing empty lines.

### Wrap Mode Auto-Scroll

When line wrapping is enabled, auto-scroll must use display line count (not buffer row count) to position correctly.

**The Problem**

Without proper calculation, auto-scroll uses buffer row as scroll offset:
```rust
// WRONG for wrap mode
process.scroll_offset = cursor_row;  // Buffer row 5
// But display might have 15 wrapped lines - misses last ~10 lines
```

**The Fix**

Calculate total display lines (excluding trailing empty) and scroll to show bottom:
```rust
let content_count = /* exclude trailing empty buffer lines */;
let total_display_lines = if process.wrap_enabled && cols > 0 {
    lines.iter().take(content_count).map(|line| {
        if line.cells.is_empty() { 1 }
        else { (line.cells.len() + cols - 1) / cols }
    }).sum::<usize>().max(1)
} else {
    content_count
};
if total_display_lines > visible {
    process.scroll_offset = total_display_lines - visible;
}
```

This matches the logic in `display_line_count()` used by manual scroll functions.

### Trailing Empty Line Exclusion

Both wrap and no-wrap modes must exclude trailing empty buffer lines for consistent behavior:

**No-wrap mode**: Uses `content_line_count()` which already excludes trailing empty lines.

**Wrap mode**: Must also exclude trailing empty lines via `content_buffer_line_count()`:
```rust
fn content_buffer_line_count(buffer: &VecDeque<Line>) -> usize {
    let mut count = buffer.len();
    while count > 0 && buffer[count - 1].cells.is_empty() {
        count -= 1;
    }
    count.max(1)
}
```

Used in:
- `display_line_count()` for wrap mode scroll calculations
- `OutputPanel::render()` when building wrapped display lines

Without this, wrap mode would show an extra empty line at bottom that no-wrap mode doesn't show.

### Auto-Scroll vs Manual Scroll Consistency

**The Bug**

Programs that output content ending with `\n` (like `glow`) would show an empty line at bottom during auto-scroll, but pressing `g` (which calls `scroll_to_bottom`) would make it disappear.

**Root Cause**

Auto-scroll originally used **cursor position** to calculate scroll offset:
```rust
// WRONG: cursor-based counting
let scroll_pos = cursor_row;  // Cursor on empty line after "content\n"
process.scroll_offset = scroll_pos - visible;
```

But render and manual scroll used **content-based counting** (excluding trailing empty lines):
```rust
// Render uses content_line_count (excludes trailing empty)
let total_lines = content_line_count();
```

When cursor sits on trailing empty line (row N after N-1 content rows):
- Auto-scroll: `scroll_offset = N - visible` (includes empty line)
- Render: only shows N-1 content lines
- Result: scroll_offset is 1 too high, showing empty line at bottom

**The Fix**

Auto-scroll now uses content-based counting, matching render and manual scroll:
```rust
let content_count = /* exclude trailing empty lines */;
let total_display_lines = /* count display lines for content only */;
process.scroll_offset = total_display_lines.saturating_sub(visible);
```

All three paths (auto-scroll, manual scroll, render) now use identical line counting logic.

### Pin Feature

Users can disable auto-scroll ("pin") by:
- Scrolling up manually
- Pressing `g` to toggle pin state
- Pressing `t` to jump to top (auto-pins)

Pin indicator (⇅) appears in process list (white on red) when pinned. Re-enables when:
- Pressing `g` again to toggle back
- Pressing `b` to jump to bottom
- User scrolls to bottom manually

Keybindings:
- `g` - Toggle pin (if following → pin; if pinned → unpin and follow)
- `t` - Jump to top (pins)
- `b` - Jump to bottom (unpins)

## Mouse Handling

### Click Zones

| Zone                    | Action                                  |
| ----------------------- | --------------------------------------- |
| Left panel (col < 20)   | Exit focus, select process if valid row |
| Right panel (col ≥ 21)  | Enter focus mode                        |
| Status bar (bottom row) | Exit focus mode                         |

```rust
if is_status_bar {
    app.exit_focus();
} else if event.column < 20 {
    // Select process if valid row
    if index < pm.process_count() {
        app.selected_index = index;
    }
    app.exit_focus();
} else if event.column >= 21 {
    app.enter_focus();
}
```

### Scroll Wheel

Works on output panel regardless of click position or mode.

## Render-Time Width Truncation

### The Problem

When the terminal is resized narrower after a process has output content, the stored lines may be wider than the current display. Programs like `fastfetch` use cursor positioning to place content at specific columns. If lines auto-wrapped during storage, content would be corrupted (text from different columns interleaved).

### Solution

Lines are stored at their full width (up to `MAX_LINE_WIDTH = 2000`), and truncation happens only at render time:

```rust
// In output_panel.rs
let spans: Vec<Span> = line
    .cells
    .iter()
    .take(inner_width)  // Truncate at render time
    .map(|cell| Span::styled(cell.c.to_string(), cell.style))
    .collect();
```

Key changes in `buffer.rs`:
1. Removed auto-wrap in `put_char()` - lines can grow as long as needed
2. Cursor positioning clamps to `MAX_LINE_WIDTH` instead of `self.cols`

### Benefits

- **Resize wider**: Hidden content reappears (it was always stored, just not displayed)
- **Resize narrower**: Content clips on right edge but isn't corrupted
- **No interleaving**: Cursor-positioned content stays on correct lines

### Memory Protection

`MAX_LINE_WIDTH = 2000` prevents runaway memory allocation from malicious or buggy escape sequences while allowing normal terminal widths.

## Per-Process Key Passthrough

### Use Case

Many TUI apps (vim, helix) use Esc and Shift-Tab for their own keybindings. The `!` suffix on process names enables full key passthrough - both Esc and Shift-Tab are forwarded to the process instead of exiting focus mode.

### Syntax

Append `!` to the process name:

```bash
panex "helix" "npm run dev" -n "helix!,server"
```

Here, "helix" receives all keys including Esc and Shift-Tab. Exit focus by clicking the left panel.

### Key Forwarding

When `!` is set:
- `Esc` → forwarded as `\x1b`
- `Shift-Tab` → forwarded as `\x1b[Z` (CSI Z)
- **Mouse click on left panel** → only way to exit focus mode

### Implementation

In `config.rs`:
```rust
let (name, proc_no_shift_tab) = if raw_name.ends_with('!') {
    (raw_name.trim_end_matches('!').to_string(), true)
} else {
    (raw_name, false)
};
```

In `input/handler.rs`:
```rust
KeyCode::Esc if !no_shift_tab => {
    app.exit_focus();
    return;
}
KeyCode::BackTab if !no_shift_tab => {
    app.exit_focus();
    return;
}
```

In `input/mouse.rs`, clicking left panel always exits focus:
```rust
if event.column < 20 {
    app.selected_index = index;
    app.exit_focus();
}
```

The status bar shows "Click LPanel:exit" when key passthrough is enabled.

## Line Wrapping

### Default Behavior

By default, lines longer than the viewport width are truncated at render time. This preserves content integrity for cursor-positioned output (like `fastfetch`) while allowing resize-wider to reveal hidden content.

### Wrap Mode

Users can enable line wrapping per process via:
- **`w` key** in browse mode - toggles wrap on/off for selected process
- **`:w` suffix** on process name - enables wrap at startup

```bash
# Enable wrapping at startup
panex "npm run build" -n "build:w"

# Combine with key passthrough (either order)
panex "helix" -n "helix!:w"
panex "helix" -n "helix:w!"
```

### Implementation

When wrap is enabled, `OutputPanel::render()` splits long lines into multiple display lines:

```rust
if process.wrap_enabled && inner_width > 0 {
    for line in buffer.iter() {
        for chunk in line.cells.chunks(inner_width) {
            wrapped_lines.push(Line::from(chunk_to_spans(chunk)));
        }
    }
}
```

Scroll functions use `display_line_count()` to account for wrapped lines:

```rust
fn display_line_count(process: &ManagedProcess, viewport_width: usize) -> usize {
    if process.wrap_enabled && viewport_width > 0 {
        buffer.iter().map(|line| {
            (line.cells.len() + viewport_width - 1) / viewport_width
        }).sum()
    } else {
        process.buffer.content_line_count()
    }
}
```

### Visual Indicator

Wrap state shown in process list as `w` (black on white) to the left of the pin indicator (⇅).

### Suffix Parsing

Suffixes set flags but the display name preserves the original form. This allows two processes with the same base name but different suffixes (e.g., `fastfetch` and `fastfetch:w`).

```rust
let name = raw_name.clone();  // Preserve original
let mut temp = raw_name;

loop {
    if temp.ends_with('!') {
        temp = temp.trim_end_matches('!').to_string();
        proc_no_shift_tab = true;
    } else if temp.ends_with(":w") {
        temp = temp.trim_end_matches(":w").to_string();
        wrap_enabled = true;
    } else {
        break;
    }
}
```

## Process Group Termination

### The Problem

When killing a process via `child.kill()`, only the immediate shell process (`/bin/sh -c command`) is terminated. Child processes spawned by that shell continue running as orphans.

Example: `panex "npm run dev"` spawns:
```
sh -c "npm run dev"    ← killed
  └── node server.js   ← continues running!
```

### Solution

Kill the entire process group using negative PID:

```rust
pub fn kill(&self) -> Result<()> {
    #[cfg(unix)]
    if let Some(pid) = child.process_id() {
        // Kill entire process group with SIGTERM
        unsafe { libc::kill(-(pid as i32), libc::SIGTERM); }

        // Give processes time to terminate gracefully
        std::thread::sleep(Duration::from_millis(50));

        // Force kill if still running
        unsafe { libc::kill(-(pid as i32), libc::SIGKILL); }
    }

    let _ = child.kill();  // Fallback
    let _ = child.wait();  // Reap zombie
    Ok(())
}
```

### Why This Works

Processes spawned in a PTY get a new session via `setsid()`. The shell becomes session leader and process group leader, so `pid == pgid`. Killing with `-pid` sends the signal to all processes in that group.

### Caveats

- Requires `libc` crate (Unix only)
- Children that create their own process groups (via `setpgid`) won't be killed
- The 50ms grace period balances responsiveness vs graceful shutdown

## Clean Shutdown

### The Problem

On quit, escape sequences can leak to the user's shell prompt. Two sources:

1. **PTY output** - Child processes may still have buffered output when killed
2. **Mouse events** - SGR mouse sequences (`\x1b[<64;51;16M`) buffered in terminal input

Example leak after scrolling in glow: `64;51;16M64;51;16M64;51;16M...`

### Solution

Shutdown sequence in `run()`:

```rust
// 1. Kill processes and wait for reader threads
pm.shutdown();
tokio::time::sleep(Duration::from_millis(50)).await;

// 2. Disable mouse capture first (stops new events)
execute!(terminal.backend_mut(), DisableMouseCapture)?;

// 3. Drain pending input events
while crossterm::event::poll(Duration::from_millis(10))? {
    let _ = crossterm::event::read();
}

// 4. Restore terminal state
disable_raw_mode()?;
execute!(
    terminal.backend_mut(),
    LeaveAlternateScreen,
    ResetColor,
    SetAttribute(Attribute::Reset),
)?;
terminal.show_cursor()?;
io::stdout().flush()?;
```

Key insight: `DisableMouseCapture` must happen before draining events, otherwise terminal keeps sending mouse sequences while we try to drain them.

## Process Restart Race Condition

### The Problem

When restarting a process, it would sometimes appear "paused" and require a second restart. Root cause: race condition between old and new reader threads.

Sequence:
1. `restart_process` kills old process, starts new one
2. Old reader thread (still running) eventually gets EOF
3. Old thread sends `ProcessExited(name, ...)`
4. `handle_exit` receives event, sets `pty = None` on the NEW process
5. New process appears dead/paused

The issue: events are keyed by process name, but after restart the same name refers to a different process instance.

### Solution: Generation Counter

Each process has a `generation: u64` that increments on every start. Events include the generation they originated from:

```rust
pub enum AppEvent {
    ProcessOutput(String, Generation, Vec<u8>),
    ProcessExited(String, Generation, Option<i32>),
    ProcessError(String, Generation, String),
    // ...
}
```

Handlers check generation before acting:

```rust
pub fn handle_exit(&mut self, name: &str, gen: Generation, code: Option<i32>) {
    if let Some(process) = self.processes.get_mut(name) {
        // Ignore events from old process instances
        if process.generation != gen {
            return;
        }
        // ... handle exit
    }
}
```

This eliminates the race entirely - old events are silently dropped regardless of timing.

### Benefits

- Restart delay reduced from 100ms to 50ms (no longer timing-dependent)
- `restart_all` can kill all processes first, then start all (faster)
- No more "paused" processes after restart
