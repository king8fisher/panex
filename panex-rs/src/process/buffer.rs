use ratatui::style::{Color, Modifier, Style};
use std::collections::VecDeque;
use vte::{Params, Perform};

const MAX_SCROLLBACK: usize = 10_000;
const MAX_LINE_WIDTH: usize = 2000; // Max column to prevent runaway memory allocation

#[derive(Debug, Clone, Default)]
pub struct Cell {
    pub c: char,
    pub style: Style,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub cells: Vec<Cell>,
}

impl Line {
    pub fn new() -> Self {
        Self { cells: Vec::new() }
    }
}

impl Default for Line {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TerminalBuffer {
    state: TerminalState,
    parser: vte::Parser,
}

struct TerminalState {
    lines: VecDeque<Line>,
    cursor_row: usize,
    cursor_col: usize,
    cols: usize,
    rows: usize,
    current_style: Style,
    saved_cursor: Option<(usize, usize)>,
    pending_responses: Vec<Vec<u8>>,
    /// Scroll region: (top, bottom) 0-indexed, inclusive. None = full screen.
    scroll_region: Option<(usize, usize)>,
    /// Whether we're in alternate screen buffer mode (CSI ?1049h).
    /// In alternate mode, buffer is fixed at `rows` lines and scrolls at bottom.
    alternate_screen: bool,
}

impl TerminalBuffer {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self {
            state: TerminalState::new(cols, rows),
            parser: vte::Parser::new(),
        }
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.state.cols = cols;
        self.state.rows = rows;
        self.state.scroll_region = None;
        // In alternate screen mode, truncate buffer to new screen size
        // and clamp cursor so no stale lines remain past the screen bottom.
        if self.state.alternate_screen && rows > 0 {
            while self.state.lines.len() > rows {
                self.state.lines.pop_back();
            }
            if self.state.cursor_row >= rows {
                self.state.cursor_row = rows - 1;
            }
        }
    }

    pub fn write(&mut self, data: &[u8]) {
        for byte in data {
            self.parser.advance(&mut self.state, *byte);
        }
    }

    /// Returns line count excluding trailing empty lines.
    /// Avoids showing empty cursor line after newline.
    pub fn content_line_count(&self) -> usize {
        let mut count = self.state.lines.len();
        while count > 0 && self.state.lines[count - 1].cells.is_empty() {
            count -= 1;
        }
        count.max(1)
    }

    pub fn get_all_lines(&self) -> &VecDeque<Line> {
        &self.state.lines
    }

    pub fn is_alternate_screen(&self) -> bool {
        self.state.alternate_screen
    }

    pub fn take_pending_responses(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.state.pending_responses)
    }
}

impl TerminalState {
    fn new(cols: usize, rows: usize) -> Self {
        let mut lines = VecDeque::with_capacity(MAX_SCROLLBACK);
        lines.push_back(Line::new());
        Self {
            lines,
            cursor_row: 0,
            cursor_col: 0,
            cols,
            rows,
            current_style: Style::default(),
            saved_cursor: None,
            pending_responses: Vec::new(),
            scroll_region: None,
            alternate_screen: false,
        }
    }

    fn ensure_row(&mut self, row: usize) {
        // In alternate screen mode, never grow buffer beyond `rows` lines
        let max = if self.alternate_screen && self.rows > 0 {
            self.rows.saturating_sub(1)
        } else {
            row
        };
        let target = row.min(max);
        while self.lines.len() <= target {
            self.lines.push_back(Line::new());
        }
        // Trim if over max scrollback (only relevant in normal mode)
        if !self.alternate_screen {
            while self.lines.len() > MAX_SCROLLBACK {
                self.lines.pop_front();
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                }
            }
        }
    }

    fn ensure_col(&mut self, col: usize) {
        self.ensure_row(self.cursor_row);
        let line = &mut self.lines[self.cursor_row];
        while line.cells.len() <= col {
            line.cells.push(Cell {
                c: ' ',
                style: Style::default(),
            });
        }
    }

    fn newline(&mut self) {
        if let Some((top, bottom)) = self.scroll_region {
            if self.cursor_row >= top && self.cursor_row <= bottom {
                if self.cursor_row == bottom {
                    self.scroll_region_up(top, bottom, 1);
                } else {
                    self.cursor_row += 1;
                    self.ensure_row(self.cursor_row);
                }
                self.cursor_col = 0;
                return;
            }
        }
        if self.alternate_screen && self.rows > 0 {
            // In alternate screen: clamp to screen bottom, never grow buffer
            let screen_bottom = self.rows - 1;
            if self.cursor_row < screen_bottom {
                self.cursor_row += 1;
            }
            // At screen bottom outside scroll region: stay put
            self.cursor_col = 0;
            return;
        }
        // Normal mode: grow buffer (preserves scrollback)
        self.cursor_row += 1;
        self.cursor_col = 0;
        self.ensure_row(self.cursor_row);
    }

    /// Scroll content up within [top, bottom] by `n` lines.
    fn scroll_region_up(&mut self, top: usize, bottom: usize, n: usize) {
        self.ensure_row(bottom);
        for _ in 0..n {
            if top <= bottom && bottom < self.lines.len() {
                self.lines.remove(top);
                self.lines.insert(bottom, Line::new());
            }
        }
    }

    /// Scroll content down within [top, bottom] by `n` lines.
    fn scroll_region_down(&mut self, top: usize, bottom: usize, n: usize) {
        self.ensure_row(bottom);
        for _ in 0..n {
            if top <= bottom && bottom < self.lines.len() {
                self.lines.remove(bottom);
                self.lines.insert(top, Line::new());
            }
        }
    }

    fn put_char(&mut self, c: char) {
        self.ensure_col(self.cursor_col);
        self.lines[self.cursor_row].cells[self.cursor_col] = Cell {
            c,
            style: self.current_style,
        };
        self.cursor_col += 1;
        // Don't auto-wrap: let lines grow as needed, truncate at render time.
        // This prevents content corruption when terminal is resized narrower.
    }

    fn clear_line_from(&mut self, col: usize) {
        self.ensure_row(self.cursor_row);
        let line = &mut self.lines[self.cursor_row];
        if col < line.cells.len() {
            line.cells.truncate(col);
        }
    }

    fn clear_screen_from_cursor(&mut self) {
        self.clear_line_from(self.cursor_col);
        // Clear all lines below
        while self.lines.len() > self.cursor_row + 1 {
            self.lines.pop_back();
        }
    }

    fn clear_screen(&mut self) {
        self.lines.clear();
        self.lines.push_back(Line::new());
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_region = None;
    }

    fn parse_sgr(&mut self, params: &Params) {
        let params: Vec<u16> = params.iter().flat_map(|p| p.iter().copied()).collect();

        if params.is_empty() {
            self.current_style = Style::default();
            return;
        }

        let mut i = 0;
        while i < params.len() {
            match params[i] {
                0 => self.current_style = Style::default(),
                1 => self.current_style = self.current_style.add_modifier(Modifier::BOLD),
                2 => self.current_style = self.current_style.add_modifier(Modifier::DIM),
                3 => self.current_style = self.current_style.add_modifier(Modifier::ITALIC),
                4 => self.current_style = self.current_style.add_modifier(Modifier::UNDERLINED),
                5 | 6 => self.current_style = self.current_style.add_modifier(Modifier::SLOW_BLINK),
                7 => self.current_style = self.current_style.add_modifier(Modifier::REVERSED),
                8 => self.current_style = self.current_style.add_modifier(Modifier::HIDDEN),
                9 => self.current_style = self.current_style.add_modifier(Modifier::CROSSED_OUT),
                22 => self.current_style = self.current_style.remove_modifier(Modifier::BOLD | Modifier::DIM),
                23 => self.current_style = self.current_style.remove_modifier(Modifier::ITALIC),
                24 => self.current_style = self.current_style.remove_modifier(Modifier::UNDERLINED),
                25 => self.current_style = self.current_style.remove_modifier(Modifier::SLOW_BLINK),
                27 => self.current_style = self.current_style.remove_modifier(Modifier::REVERSED),
                28 => self.current_style = self.current_style.remove_modifier(Modifier::HIDDEN),
                29 => self.current_style = self.current_style.remove_modifier(Modifier::CROSSED_OUT),
                30..=37 => {
                    self.current_style = self.current_style.fg(ansi_to_color(params[i] - 30));
                }
                38 => {
                    if i + 2 < params.len() && params[i + 1] == 5 {
                        self.current_style = self.current_style.fg(Color::Indexed(params[i + 2] as u8));
                        i += 2;
                    } else if i + 4 < params.len() && params[i + 1] == 2 {
                        self.current_style = self.current_style.fg(Color::Rgb(
                            params[i + 2] as u8,
                            params[i + 3] as u8,
                            params[i + 4] as u8,
                        ));
                        i += 4;
                    }
                }
                39 => self.current_style = self.current_style.fg(Color::Reset),
                40..=47 => {
                    self.current_style = self.current_style.bg(ansi_to_color(params[i] - 40));
                }
                48 => {
                    if i + 2 < params.len() && params[i + 1] == 5 {
                        self.current_style = self.current_style.bg(Color::Indexed(params[i + 2] as u8));
                        i += 2;
                    } else if i + 4 < params.len() && params[i + 1] == 2 {
                        self.current_style = self.current_style.bg(Color::Rgb(
                            params[i + 2] as u8,
                            params[i + 3] as u8,
                            params[i + 4] as u8,
                        ));
                        i += 4;
                    }
                }
                49 => self.current_style = self.current_style.bg(Color::Reset),
                90..=97 => {
                    self.current_style = self.current_style.fg(bright_ansi_to_color(params[i] - 90));
                }
                100..=107 => {
                    self.current_style = self.current_style.bg(bright_ansi_to_color(params[i] - 100));
                }
                _ => {}
            }
            i += 1;
        }
    }
}

fn ansi_to_color(n: u16) -> Color {
    match n {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::White,
        _ => Color::White,
    }
}

fn bright_ansi_to_color(n: u16) -> Color {
    match n {
        0 => Color::DarkGray,
        1 => Color::LightRed,
        2 => Color::LightGreen,
        3 => Color::LightYellow,
        4 => Color::LightBlue,
        5 => Color::LightMagenta,
        6 => Color::LightCyan,
        7 => Color::White,
        _ => Color::White,
    }
}

impl Perform for TerminalState {
    fn print(&mut self, c: char) {
        self.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x08 => {
                // Backspace
                if self.cursor_col > 0 {
                    self.cursor_col -= 1;
                }
            }
            0x09 => {
                // Tab
                let next_tab = (self.cursor_col + 8) & !7;
                self.cursor_col = next_tab.min(MAX_LINE_WIDTH - 1);
            }
            0x0A | 0x0B | 0x0C => {
                // LF, VT, FF
                self.newline();
            }
            0x0D => {
                // CR
                self.cursor_col = 0;
            }
            _ => {}
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        let params_vec: Vec<u16> = params.iter().flat_map(|p| p.iter().copied()).collect();
        let get_param = |i: usize, default: u16| -> u16 {
            params_vec.get(i).copied().filter(|&v| v != 0).unwrap_or(default)
        };

        match action {
            'A' => {
                // Cursor up
                let n = get_param(0, 1) as usize;
                if let Some((top, bottom)) = self.scroll_region {
                    if self.cursor_row >= top && self.cursor_row <= bottom {
                        // Inside scroll region: clamp to top
                        self.cursor_row = self.cursor_row.saturating_sub(n).max(top);
                    } else {
                        self.cursor_row = self.cursor_row.saturating_sub(n);
                    }
                } else {
                    self.cursor_row = self.cursor_row.saturating_sub(n);
                }
            }
            'B' => {
                // Cursor down
                let n = get_param(0, 1) as usize;
                if let Some((top, bottom)) = self.scroll_region {
                    if self.cursor_row >= top && self.cursor_row <= bottom {
                        self.cursor_row = (self.cursor_row + n).min(bottom);
                    } else if self.alternate_screen && self.rows > 0 {
                        self.cursor_row = (self.cursor_row + n).min(self.rows - 1);
                    } else {
                        self.cursor_row += n;
                    }
                } else if self.alternate_screen && self.rows > 0 {
                    self.cursor_row = (self.cursor_row + n).min(self.rows - 1);
                } else {
                    self.cursor_row += n;
                }
                self.ensure_row(self.cursor_row);
            }
            'C' => {
                // Cursor forward
                let n = get_param(0, 1) as usize;
                self.cursor_col = (self.cursor_col + n).min(MAX_LINE_WIDTH - 1);
            }
            'D' => {
                // Cursor back
                let n = get_param(0, 1) as usize;
                self.cursor_col = self.cursor_col.saturating_sub(n);
            }
            'E' => {
                // Cursor next line
                let n = get_param(0, 1) as usize;
                self.cursor_row += n;
                if self.alternate_screen && self.rows > 0 {
                    self.cursor_row = self.cursor_row.min(self.rows - 1);
                }
                self.cursor_col = 0;
                self.ensure_row(self.cursor_row);
            }
            'F' => {
                // Cursor previous line
                let n = get_param(0, 1) as usize;
                self.cursor_row = self.cursor_row.saturating_sub(n);
                self.cursor_col = 0;
            }
            'G' => {
                // Cursor horizontal absolute
                let col = get_param(0, 1).saturating_sub(1) as usize;
                self.cursor_col = col.min(MAX_LINE_WIDTH - 1);
            }
            'H' | 'f' => {
                // Cursor position
                let mut row = get_param(0, 1).saturating_sub(1) as usize;
                let col = get_param(1, 1).saturating_sub(1) as usize;
                if self.alternate_screen && self.rows > 0 {
                    row = row.min(self.rows - 1);
                }
                self.cursor_row = row;
                self.cursor_col = col.min(MAX_LINE_WIDTH - 1);
                self.ensure_row(self.cursor_row);
            }
            'J' => {
                // Erase in display
                let mode = get_param(0, 0);
                match mode {
                    0 => self.clear_screen_from_cursor(),
                    1 => {
                        // Clear from start to cursor (rarely used)
                    }
                    2 | 3 => self.clear_screen(),
                    _ => {}
                }
            }
            'K' => {
                // Erase in line
                let mode = get_param(0, 0);
                match mode {
                    0 => self.clear_line_from(self.cursor_col),
                    1 => {
                        // Clear from start to cursor
                        self.ensure_row(self.cursor_row);
                        for i in 0..=self.cursor_col {
                            if i < self.lines[self.cursor_row].cells.len() {
                                self.lines[self.cursor_row].cells[i] = Cell {
                                    c: ' ',
                                    style: Style::default(),
                                };
                            }
                        }
                    }
                    2 => {
                        // Clear entire line
                        self.ensure_row(self.cursor_row);
                        self.lines[self.cursor_row].cells.clear();
                    }
                    _ => {}
                }
            }
            'r' => {
                // DECSTBM - Set Scrolling Region
                // CSI top ; bottom r  (1-indexed, default = full screen)
                let top = get_param(0, 1).saturating_sub(1) as usize;
                let bottom = get_param(1, self.rows as u16).saturating_sub(1) as usize;
                let max_row = self.rows.saturating_sub(1);
                let top = top.min(max_row);
                let bottom = bottom.min(max_row);
                if top < bottom {
                    self.scroll_region = Some((top, bottom));
                } else {
                    self.scroll_region = None;
                }
                // DECSTBM also homes the cursor
                self.cursor_row = 0;
                self.cursor_col = 0;
            }
            'S' => {
                // Scroll Up (SU) - scroll content up within scroll region
                if let Some((top, bottom)) = self.scroll_region {
                    let n = get_param(0, 1) as usize;
                    self.scroll_region_up(top, bottom, n);
                }
            }
            'T' => {
                // Scroll Down (SD) - scroll content down within scroll region
                if let Some((top, bottom)) = self.scroll_region {
                    let n = get_param(0, 1) as usize;
                    self.scroll_region_down(top, bottom, n);
                }
            }
            'L' => {
                // Insert Lines (IL) - insert blank lines at cursor, push down within region
                if let Some((_top, bottom)) = self.scroll_region {
                    let n = get_param(0, 1) as usize;
                    self.ensure_row(bottom);
                    for _ in 0..n {
                        if self.cursor_row <= bottom && bottom < self.lines.len() {
                            self.lines.remove(bottom);
                            self.lines.insert(self.cursor_row, Line::new());
                        }
                    }
                }
            }
            'M' => {
                // Delete Lines (DL) - delete lines at cursor, pull up within region
                if let Some((_top, bottom)) = self.scroll_region {
                    let n = get_param(0, 1) as usize;
                    self.ensure_row(bottom);
                    for _ in 0..n {
                        if self.cursor_row <= bottom && self.cursor_row < self.lines.len() {
                            self.lines.remove(self.cursor_row);
                            self.lines.insert(bottom, Line::new());
                        }
                    }
                }
            }
            'm' => {
                // SGR - Select Graphic Rendition
                self.parse_sgr(params);
            }
            's' => {
                // Save cursor
                self.saved_cursor = Some((self.cursor_row, self.cursor_col));
            }
            'u' => {
                // Restore cursor
                if let Some((row, col)) = self.saved_cursor {
                    self.cursor_row = row;
                    self.cursor_col = col;
                }
            }
            'c' => {
                // Device Attributes (DA) - respond as VT100 with AVO
                // Apps like glow query this and timeout if no response
                self.pending_responses.push(b"\x1b[?1;2c".to_vec());
            }
            'n' => {
                // Device Status Report (DSR)
                let mode = get_param(0, 0);
                match mode {
                    5 => {
                        // Status report - respond "OK"
                        self.pending_responses.push(b"\x1b[0n".to_vec());
                    }
                    6 => {
                        // Cursor position report
                        let response = format!("\x1b[{};{}R", self.cursor_row + 1, self.cursor_col + 1);
                        self.pending_responses.push(response.into_bytes());
                    }
                    _ => {}
                }
            }
            't' => {
                // Window manipulation (XTWINOPS)
                let mode = get_param(0, 0);
                match mode {
                    18 => {
                        // Report terminal size in characters
                        // Response: CSI 8 ; rows ; cols t
                        let response = format!("\x1b[8;{};{}t", self.rows, self.cols);
                        self.pending_responses.push(response.into_bytes());
                    }
                    _ => {}
                }
            }
            'h' | 'l' => {
                // DEC Private Mode Set/Reset (CSI ? ... h/l)
                if intermediates == b"?" {
                    let mode = get_param(0, 0);
                    match mode {
                        1049 | 1047 | 47 => {
                            // Alternate screen buffer
                            if action == 'h' {
                                self.alternate_screen = true;
                                self.clear_screen();
                            } else {
                                self.alternate_screen = false;
                                self.scroll_region = None;
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}
