use crate::ui::InputMode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
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
        let (mode_text, mode_color, hints) = match self.mode {
            InputMode::Normal => (
                " NORMAL ",
                Color::Blue,
                "↑↓:nav  Enter/Tab:focus  r:restart  A:all  x:kill  g:pin  t/b:top/bot  ?:help  q:quit",
            ),
            InputMode::Focus => {
                let exit_key = if self.no_shift_tab {
                    "Esc:exit"
                } else {
                    "Esc/Shift-Tab:exit"
                };
                (" FOCUS ", Color::Green, exit_key)
            }
        };

        let line = Line::from(vec![
            Span::styled(
                mode_text,
                Style::default()
                    .fg(Color::Black)
                    .bg(mode_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(hints, Style::default().fg(Color::DarkGray)),
        ]);

        Paragraph::new(line).render(area, buf);
    }
}
