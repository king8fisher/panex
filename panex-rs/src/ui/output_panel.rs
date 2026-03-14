use crate::input::SelectionState;
use crate::process::ManagedProcess;
use crate::ui::InputMode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
};

pub struct OutputPanel<'a> {
    process: Option<&'a ManagedProcess>,
    #[allow(dead_code)]
    mode: InputMode,
    selection: &'a SelectionState,
}

impl<'a> OutputPanel<'a> {
    pub fn new(process: Option<&'a ManagedProcess>, mode: InputMode, selection: &'a SelectionState) -> Self {
        Self { process, mode, selection }
    }
}

impl Widget for OutputPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear entire area first to prevent artifacts from underlying terminal
        Block::default().render(area, buf);

        let lines = match self.process {
            Some(process) => {
                let buffer = process.buffer.get_all_lines();
                let inner_height = area.height as usize;
                let inner_width = area.width as usize;

                if process.wrap_enabled && inner_width > 0 {
                    // Wrap mode: split long lines into multiple display lines
                    let content_count = content_buffer_line_count(buffer);
                    let mut wrapped_lines: Vec<Line> = Vec::new();
                    // Track buffer row for each wrapped line (for selection)
                    let mut line_map: Vec<(usize, usize)> = Vec::new(); // (buffer_row, start_col)
                    for (row_idx, line) in buffer.iter().take(content_count).enumerate() {
                        if line.cells.is_empty() {
                            wrapped_lines.push(Line::from(""));
                            line_map.push((row_idx, 0));
                        } else {
                            for (chunk_idx, chunk) in line.cells.chunks(inner_width).enumerate() {
                                let start_col = chunk_idx * inner_width;
                                let spans: Vec<Span> = chunk
                                    .iter()
                                    .enumerate()
                                    .map(|(i, cell)| {
                                        let col = start_col + i;
                                        let style = if self.selection.contains(row_idx, col) {
                                            cell.style.add_modifier(Modifier::REVERSED)
                                        } else {
                                            cell.style
                                        };
                                        Span::styled(cell.c.to_string(), style)
                                    })
                                    .collect();
                                wrapped_lines.push(Line::from(spans));
                                line_map.push((row_idx, start_col));
                            }
                        }
                    }

                    let total_lines = wrapped_lines.len().max(1);
                    let start = process.scroll_offset.min(total_lines.saturating_sub(1));
                    let end = (start + inner_height).min(total_lines);

                    wrapped_lines.into_iter().skip(start).take(end - start).collect()
                } else {
                    // Normal mode: truncate lines at viewport width
                    let total_lines = process.buffer.content_line_count();
                    let start = process.scroll_offset.min(total_lines.saturating_sub(1));
                    let end = (start + inner_height).min(total_lines);

                    buffer
                        .iter()
                        .enumerate()
                        .skip(start)
                        .take(end - start)
                        .map(|(row_idx, line)| {
                            let spans: Vec<Span> = line
                                .cells
                                .iter()
                                .enumerate()
                                .take(inner_width)
                                .map(|(col, cell)| {
                                    let style = if self.selection.contains(row_idx, col) {
                                        cell.style.add_modifier(Modifier::REVERSED)
                                    } else {
                                        cell.style
                                    };
                                    Span::styled(cell.c.to_string(), style)
                                })
                                .collect();
                            Line::from(spans)
                        })
                        .collect()
                }
            }
            None => vec![Line::from("No process selected")],
        };

        let paragraph = Paragraph::new(lines);
        paragraph.render(area, buf);
    }
}

/// Compute total display lines accounting for wrap mode, excluding trailing empty lines
fn display_line_count(process: &ManagedProcess, viewport_width: usize) -> usize {
    if process.wrap_enabled && viewport_width > 0 {
        let buffer = process.buffer.get_all_lines();
        // Exclude trailing empty lines (consistent with content_line_count)
        let content_count = content_buffer_line_count(buffer);
        buffer.iter().take(content_count).map(|line| {
            if line.cells.is_empty() {
                1
            } else {
                line.cells.len().div_ceil(viewport_width)
            }
        }).sum::<usize>().max(1)
    } else {
        process.buffer.content_line_count()
    }
}

/// Count buffer lines excluding trailing empty ones
fn content_buffer_line_count(buffer: &std::collections::VecDeque<crate::process::buffer::Line>) -> usize {
    let mut count = buffer.len();
    while count > 0 && buffer[count - 1].cells.is_empty() {
        count -= 1;
    }
    count.max(1)
}

pub fn scroll_up(process: &mut ManagedProcess, amount: usize) {
    process.scroll_offset = process.scroll_offset.saturating_sub(amount);
    process.auto_scroll = false;
}

pub fn scroll_down(process: &mut ManagedProcess, amount: usize, visible_height: usize, viewport_width: usize) {
    let total = display_line_count(process, viewport_width);
    let max_scroll = total.saturating_sub(visible_height);
    process.scroll_offset = (process.scroll_offset + amount).min(max_scroll);

    // Re-enable auto scroll if at bottom
    if process.scroll_offset >= max_scroll {
        process.auto_scroll = true;
    }
}

pub fn scroll_to_top(process: &mut ManagedProcess) {
    process.scroll_offset = 0;
    process.auto_scroll = false;
}

pub fn scroll_to_bottom(process: &mut ManagedProcess, visible_height: usize, viewport_width: usize) {
    let total = display_line_count(process, viewport_width);
    let max_scroll = total.saturating_sub(visible_height);
    process.scroll_offset = max_scroll;
    process.auto_scroll = true;
}
