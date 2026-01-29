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
    no_shift_tab: bool,
}

impl StatusBar {
    pub fn new(mode: InputMode, no_shift_tab: bool) -> Self {
        Self { mode, no_shift_tab }
    }
}

impl Widget for StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let hints = match self.mode {
            InputMode::Browse => "↑↓:nav  Enter/Tab:focus  r:restart  A:all  x:kill  g:pin  t/b:top/bot  ?:help  q:quit",
            InputMode::Focus => {
                if self.no_shift_tab {
                    "Esc:exit"
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
