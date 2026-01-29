use crate::process::ManagedProcess;
use crate::ui::InputMode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub struct OutputPanel<'a> {
    process: Option<&'a ManagedProcess>,
    #[allow(dead_code)]
    mode: InputMode,
}

impl<'a> OutputPanel<'a> {
    pub fn new(process: Option<&'a ManagedProcess>, mode: InputMode) -> Self {
        Self { process, mode }
    }
}

impl Widget for OutputPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines = match self.process {
            Some(process) => {
                let buffer = process.buffer.get_all_lines();
                let inner_height = area.height as usize;
                let inner_width = area.width as usize;

                if process.wrap_enabled && inner_width > 0 {
                    // Wrap mode: split long lines into multiple display lines
                    let mut wrapped_lines: Vec<Line> = Vec::new();
                    for line in buffer.iter() {
                        if line.cells.is_empty() {
                            wrapped_lines.push(Line::from(""));
                        } else {
                            // Split line into chunks of inner_width
                            for chunk in line.cells.chunks(inner_width) {
                                let spans: Vec<Span> = chunk
                                    .iter()
                                    .map(|cell| Span::styled(cell.c.to_string(), cell.style))
                                    .collect();
                                wrapped_lines.push(Line::from(spans));
                            }
                        }
                    }

                    let total_lines = wrapped_lines.len();
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
                        .skip(start)
                        .take(end - start)
                        .map(|line| {
                            let spans: Vec<Span> = line
                                .cells
                                .iter()
                                .take(inner_width)
                                .map(|cell| Span::styled(cell.c.to_string(), cell.style))
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

/// Compute total display lines accounting for wrap mode
fn display_line_count(process: &ManagedProcess, viewport_width: usize) -> usize {
    if process.wrap_enabled && viewport_width > 0 {
        let buffer = process.buffer.get_all_lines();
        buffer.iter().map(|line| {
            if line.cells.is_empty() {
                1
            } else {
                (line.cells.len() + viewport_width - 1) / viewport_width
            }
        }).sum()
    } else {
        process.buffer.content_line_count()
    }
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
