use crate::ui::InputMode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

pub struct StatusBar<'a> {
    mode: InputMode,
    no_shift_tab: bool,
    proc_no_shift_tab: bool,
    status_message: Option<&'a str>,
}

impl<'a> StatusBar<'a> {
    pub fn new(mode: InputMode, no_shift_tab: bool, proc_no_shift_tab: bool, status_message: Option<&'a str>) -> Self {
        Self { mode, no_shift_tab, proc_no_shift_tab, status_message }
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let no_shift_tab = self.no_shift_tab || self.proc_no_shift_tab;
        let hints = match self.mode {
            InputMode::Browse => "↑↓:nav  Enter/Tab:focus  r:restart  A:all  x:kill  g:pin  t/b:top/bot  v/V:select  ?:help  q:quit",
            InputMode::Focus => {
                if no_shift_tab {
                    "Click LPanel:exit"
                } else {
                    "Esc/Shift-Tab:exit"
                }
            }
        };

        // Show COPIED badge in place of mode label when status message is active
        let mode_badge = if self.status_message.is_some() {
            Span::styled(
                " COPIED ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            self.mode.styled_label()
        };

        let line = Line::from(vec![
            mode_badge,
            Span::raw(" "),
            Span::styled(hints, Style::default().fg(Color::DarkGray)),
        ]);

        Paragraph::new(line).render(area, buf);
    }
}
