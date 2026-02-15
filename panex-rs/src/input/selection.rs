use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    Char,
    Line,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionPhase {
    Idle,
    Selecting,
    Selected,
}

/// Buffer-relative coordinates (row = buffer line index, col = character index)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferPos {
    pub row: usize,
    pub col: usize,
}

impl BufferPos {
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

impl PartialOrd for BufferPos {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BufferPos {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.row.cmp(&other.row).then(self.col.cmp(&other.col))
    }
}

#[derive(Debug, Clone)]
pub struct SelectionState {
    pub phase: SelectionPhase,
    pub mode: SelectionMode,
    pub anchor: BufferPos,
    pub cursor: BufferPos,
    /// Track clicks for double/triple-click detection
    last_click_time: Option<Instant>,
    last_click_pos: Option<(u16, u16)>,
    click_count: u8,
}

impl SelectionState {
    pub fn new() -> Self {
        Self {
            phase: SelectionPhase::Idle,
            mode: SelectionMode::Char,
            anchor: BufferPos::new(0, 0),
            cursor: BufferPos::new(0, 0),
            last_click_time: None,
            last_click_pos: None,
            click_count: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.phase != SelectionPhase::Idle
    }

    /// Returns (start, end) in normalized order
    pub fn normalized_range(&self) -> (BufferPos, BufferPos) {
        if self.anchor <= self.cursor {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }

    /// Start a new mouse selection
    pub fn start_mouse_select(&mut self, pos: BufferPos, screen_col: u16, screen_row: u16) {
        let now = Instant::now();
        let same_pos = self
            .last_click_pos
            .map(|(c, r)| c == screen_col && r == screen_row)
            .unwrap_or(false);
        let recent = self
            .last_click_time
            .map(|t| now.duration_since(t).as_millis() < 400)
            .unwrap_or(false);

        if same_pos && recent {
            self.click_count = (self.click_count % 3) + 1;
        } else {
            self.click_count = 1;
        }
        self.last_click_time = Some(now);
        self.last_click_pos = Some((screen_col, screen_row));

        match self.click_count {
            2 => {
                // Double-click: select word (handled by caller via expand_to_word)
                self.mode = SelectionMode::Char;
                self.anchor = pos;
                self.cursor = pos;
                self.phase = SelectionPhase::Selected;
            }
            3 => {
                // Triple-click: select line
                self.mode = SelectionMode::Line;
                self.anchor = pos;
                self.cursor = pos;
                self.phase = SelectionPhase::Selected;
            }
            _ => {
                // Single click: begin char selection
                self.mode = SelectionMode::Char;
                self.anchor = pos;
                self.cursor = pos;
                self.phase = SelectionPhase::Selecting;
            }
        }
    }

    /// Begin a drag selection directly (no click-counting)
    pub fn begin_drag(&mut self, anchor: BufferPos) {
        self.mode = SelectionMode::Char;
        self.anchor = anchor;
        self.cursor = anchor;
        self.phase = SelectionPhase::Selecting;
    }

    pub fn update_mouse_drag(&mut self, pos: BufferPos) {
        if self.phase == SelectionPhase::Selecting {
            self.cursor = pos;
        }
    }

    pub fn finish_mouse_select(&mut self) {
        if self.phase == SelectionPhase::Selecting {
            if self.anchor == self.cursor {
                // No actual selection
                self.phase = SelectionPhase::Idle;
            } else {
                self.phase = SelectionPhase::Selected;
            }
        }
    }

    /// Start keyboard visual selection
    pub fn start_visual(&mut self, pos: BufferPos, line_mode: bool) {
        self.mode = if line_mode {
            SelectionMode::Line
        } else {
            SelectionMode::Char
        };
        self.anchor = pos;
        self.cursor = pos;
        self.phase = SelectionPhase::Selecting;
    }

    pub fn move_cursor(&mut self, pos: BufferPos) {
        if self.phase == SelectionPhase::Selecting {
            self.cursor = pos;
        }
    }

    pub fn clear(&mut self) {
        self.phase = SelectionPhase::Idle;
    }

    /// Check if a given buffer position is within the selection
    pub fn contains(&self, row: usize, col: usize) -> bool {
        if !self.is_active() {
            return false;
        }
        let (start, end) = self.normalized_range();
        match self.mode {
            SelectionMode::Char => {
                if row < start.row || row > end.row {
                    return false;
                }
                if start.row == end.row {
                    col >= start.col && col <= end.col
                } else if row == start.row {
                    col >= start.col
                } else if row == end.row {
                    col <= end.col
                } else {
                    true
                }
            }
            SelectionMode::Line => row >= start.row && row <= end.row,
        }
    }
}

/// Map screen coordinates to buffer position (non-wrap mode)
/// panel_x: left edge of the output panel in screen coordinates
/// scroll_offset: current scroll offset of the process buffer
pub fn screen_to_buffer(
    screen_col: u16,
    screen_row: u16,
    panel_x: u16,
    scroll_offset: usize,
    _panel_width: usize,
) -> BufferPos {
    let col = screen_col.saturating_sub(panel_x) as usize;
    let row = screen_row as usize + scroll_offset;
    BufferPos::new(row, col)
}

/// Map a visual (wrapped) line index + column to a buffer position.
/// Walks buffer lines, splitting each into ceil(len / viewport_width) visual lines.
pub fn visual_to_buffer(
    visual_row: usize,
    visual_col: usize,
    buffer: &std::collections::VecDeque<crate::process::buffer::Line>,
    viewport_width: usize,
) -> BufferPos {
    if viewport_width == 0 {
        return BufferPos::new(0, visual_col);
    }
    let content_count = content_line_count(buffer);
    let mut visual = 0usize;
    for row_idx in 0..content_count {
        let line = &buffer[row_idx];
        let n_visual = if line.cells.is_empty() {
            1
        } else {
            (line.cells.len() + viewport_width - 1) / viewport_width
        };
        if visual_row < visual + n_visual {
            let chunk_index = visual_row - visual;
            let col = chunk_index * viewport_width + visual_col;
            return BufferPos::new(row_idx, col);
        }
        visual += n_visual;
    }
    // Clamp to last buffer line
    let last = content_count.saturating_sub(1);
    BufferPos::new(last, visual_col)
}

/// Map screen coordinates to buffer position in wrap mode.
pub fn screen_to_buffer_wrapped(
    screen_col: u16,
    screen_row: u16,
    panel_x: u16,
    scroll_offset: usize,
    buffer: &std::collections::VecDeque<crate::process::buffer::Line>,
    viewport_width: usize,
) -> BufferPos {
    let col = screen_col.saturating_sub(panel_x) as usize;
    let visual_row = screen_row as usize + scroll_offset;
    visual_to_buffer(visual_row, col, buffer, viewport_width)
}

/// Map a visual row (scroll_offset-based) to the corresponding buffer row.
/// Useful for edge-scroll and keyboard visual select where column is handled separately.
pub fn visual_row_to_buffer_row(
    visual_row: usize,
    buffer: &std::collections::VecDeque<crate::process::buffer::Line>,
    viewport_width: usize,
) -> BufferPos {
    visual_to_buffer(visual_row, 0, buffer, viewport_width)
}

/// Count buffer lines excluding trailing empty ones (mirrors output_panel logic).
fn content_line_count(buffer: &std::collections::VecDeque<crate::process::buffer::Line>) -> usize {
    let mut count = buffer.len();
    while count > 0 && buffer[count - 1].cells.is_empty() {
        count -= 1;
    }
    count.max(1)
}

/// Extract selected text from buffer
pub fn extract_selected_text(
    selection: &SelectionState,
    buffer: &std::collections::VecDeque<crate::process::buffer::Line>,
) -> String {
    if !selection.is_active() {
        return String::new();
    }

    let (start, end) = selection.normalized_range();
    let mut result = String::new();

    for row in start.row..=end.row {
        if row >= buffer.len() {
            break;
        }
        let line = &buffer[row];

        let col_start = if row == start.row && selection.mode == SelectionMode::Char {
            start.col
        } else {
            0
        };
        let col_end = if row == end.row && selection.mode == SelectionMode::Char {
            end.col.saturating_add(1)
        } else {
            line.cells.len()
        };

        let col_end = col_end.min(line.cells.len());

        for col in col_start..col_end {
            result.push(line.cells[col].c);
        }

        // Trim trailing spaces from each line
        let trimmed = result.trim_end_matches(' ').len();
        result.truncate(trimmed);

        if row < end.row {
            result.push('\n');
        }
    }

    result
}

/// Expand a position to word boundaries
pub fn expand_to_word(
    pos: BufferPos,
    buffer: &std::collections::VecDeque<crate::process::buffer::Line>,
) -> (BufferPos, BufferPos) {
    if pos.row >= buffer.len() {
        return (pos, pos);
    }
    let line = &buffer[pos.row];
    if pos.col >= line.cells.len() {
        return (pos, pos);
    }

    let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
    let ch = line.cells[pos.col].c;

    if is_word_char(ch) {
        let mut start = pos.col;
        while start > 0 && is_word_char(line.cells[start - 1].c) {
            start -= 1;
        }
        let mut end = pos.col;
        while end + 1 < line.cells.len() && is_word_char(line.cells[end + 1].c) {
            end += 1;
        }
        (BufferPos::new(pos.row, start), BufferPos::new(pos.row, end))
    } else {
        (pos, pos)
    }
}
