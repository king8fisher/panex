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

                let total_lines = buffer.len();
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
            None => vec![Line::from("No process selected")],
        };

        let paragraph = Paragraph::new(lines);
        paragraph.render(area, buf);
    }
}

pub fn scroll_up(process: &mut ManagedProcess, amount: usize) {
    process.scroll_offset = process.scroll_offset.saturating_sub(amount);
    process.auto_scroll = false;
}

pub fn scroll_down(process: &mut ManagedProcess, amount: usize, visible_height: usize) {
    let max_scroll = process.buffer.line_count().saturating_sub(visible_height);
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

pub fn scroll_to_bottom(process: &mut ManagedProcess) {
    process.auto_scroll = true;
}
