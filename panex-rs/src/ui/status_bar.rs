use crate::ui::search::SearchState;
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
    search: &'a SearchState,
}

impl<'a> StatusBar<'a> {
    pub fn new(
        mode: InputMode,
        no_shift_tab: bool,
        proc_no_shift_tab: bool,
        status_message: Option<&'a str>,
        search: &'a SearchState,
    ) -> Self {
        Self {
            mode,
            no_shift_tab,
            proc_no_shift_tab,
            status_message,
            search,
        }
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Search mode overrides the normal status bar
        if self.search.is_typing() {
            let query = self.search.query();
            let line = Line::from(vec![
                Span::styled(
                    " SEARCH ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" /"),
                Span::styled(query, Style::default().fg(Color::White)),
                Span::styled("▌", Style::default().fg(Color::DarkGray)),
            ]);
            Paragraph::new(line).render(area, buf);
            return;
        }

        if self.search.is_active() {
            let count = self.search.match_count();
            let info = if count == 0 {
                "No matches".to_string()
            } else {
                let current = self.search.current_index().unwrap_or(0) + 1;
                format!("{current}/{count}")
            };
            let query = self.search.query();
            let line = Line::from(vec![
                Span::styled(
                    " SEARCH ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(" /{query}")),
                Span::raw("  "),
                Span::styled(format!("[{info}]"), Style::default().fg(Color::Yellow)),
                Span::raw("  "),
                Span::styled(
                    "n/N:next/prev  Enter:confirm  Esc:cancel",
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            Paragraph::new(line).render(area, buf);
            return;
        }

        let no_shift_tab = self.no_shift_tab || self.proc_no_shift_tab;
        let hints = match self.mode {
            InputMode::Browse => "↑↓:nav  Enter/Tab:focus  r:restart  A:all  x:kill  g:pin  t/b:top/bot  /:search  v/V:select  ?:help  q:quit",
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
