use crate::ui::InputMode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub struct StatusBar {
    mode: InputMode,
    no_shift_tab: bool,         // Global flag
    proc_no_shift_tab: bool,    // Per-process flag
}

impl StatusBar {
    pub fn new(mode: InputMode, no_shift_tab: bool, proc_no_shift_tab: bool) -> Self {
        Self { mode, no_shift_tab, proc_no_shift_tab }
    }
}

impl Widget for StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let no_shift_tab = self.no_shift_tab || self.proc_no_shift_tab;
        let hints = match self.mode {
            InputMode::Browse => "↑↓:nav  Enter/Tab:focus  r:restart  A:all  x:kill  g:pin  t/b:top/bot  ?:help  q:quit",
            InputMode::Focus => {
                if no_shift_tab {
                    "Click LPanel:exit"
                } else {
                    "Esc/Shift-Tab:exit"
                }
            }
        };

        let line = Line::from(vec![
            self.mode.styled_label(),
            Span::raw(" "),
            Span::styled(hints, Style::default().fg(Color::DarkGray)),
        ]);

        Paragraph::new(line).render(area, buf);
    }
}
