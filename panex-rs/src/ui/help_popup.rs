use crate::ui::app::RestartAction;
use crate::ui::InputMode;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

pub struct HelpPopup {
    scroll: u16,
}

impl HelpPopup {
    pub fn new(scroll: u16) -> Self {
        Self { scroll }
    }
}

impl Widget for HelpPopup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let popup_area = centered_rect(60, 70, area);

        // Clear the area behind popup
        Clear.render(popup_area, buf);

        let help_text = vec![
            Line::from(vec![InputMode::Browse.styled_label()]),
            Line::from(""),
            Line::from(vec![
                Span::styled("↑      ", Style::default().fg(Color::Yellow)),
                Span::raw("Select previous process"),
            ]),
            Line::from(vec![
                Span::styled("↓      ", Style::default().fg(Color::Yellow)),
                Span::raw("Select next process"),
            ]),
            Line::from(vec![
                Span::styled("Enter  ", Style::default().fg(Color::Yellow)),
                Span::raw("Focus selected process"),
            ]),
            Line::from(vec![
                Span::styled("Tab    ", Style::default().fg(Color::Yellow)),
                Span::raw("Focus selected process"),
            ]),
            Line::from(vec![
                Span::styled("r      ", Style::default().fg(Color::Yellow)),
                Span::raw("Restart selected process"),
            ]),
            Line::from(vec![
                Span::styled("A      ", Style::default().fg(Color::Yellow)),
                Span::raw("Restart all processes"),
            ]),
            Line::from(vec![
                Span::styled("x      ", Style::default().fg(Color::Yellow)),
                Span::raw("Kill selected process"),
            ]),
            Line::from(vec![
                Span::styled("g      ", Style::default().fg(Color::Yellow)),
                Span::raw("Toggle auto-scroll"),
            ]),
            Line::from(vec![
                Span::styled("t      ", Style::default().fg(Color::Yellow)),
                Span::raw("Scroll to top"),
            ]),
            Line::from(vec![
                Span::styled("b      ", Style::default().fg(Color::Yellow)),
                Span::raw("Scroll to bottom"),
            ]),
            Line::from(vec![
                Span::styled("PgUp   ", Style::default().fg(Color::Yellow)),
                Span::raw("Page up"),
            ]),
            Line::from(vec![
                Span::styled("PgDown ", Style::default().fg(Color::Yellow)),
                Span::raw("Page down"),
            ]),
            Line::from(vec![
                Span::styled("/      ", Style::default().fg(Color::Yellow)),
                Span::raw("Search in output"),
            ]),
            Line::from(vec![
                Span::styled("n      ", Style::default().fg(Color::Yellow)),
                Span::raw("Next search match"),
            ]),
            Line::from(vec![
                Span::styled("N      ", Style::default().fg(Color::Yellow)),
                Span::raw("Previous search match"),
            ]),
            Line::from(vec![
                Span::styled("?      ", Style::default().fg(Color::Yellow)),
                Span::raw("Toggle help"),
            ]),
            Line::from(vec![
                Span::styled("q      ", Style::default().fg(Color::Yellow)),
                Span::raw("Quit"),
            ]),
            Line::from(vec![
                Span::styled("Ctrl+c ", Style::default().fg(Color::Yellow)),
                Span::raw("Quit"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Mouse ", Style::default().add_modifier(Modifier::BOLD)),
                InputMode::Browse.styled_label(),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Scroll     ", Style::default().fg(Color::Yellow)),
                Span::raw("Scroll output"),
            ]),
            Line::from(vec![
                Span::styled("Click left ", Style::default().fg(Color::Yellow)),
                Span::raw("Select process"),
            ]),
            Line::from(vec![
                Span::styled("Click right", Style::default().fg(Color::Yellow)),
                Span::raw(" Enter focus mode"),
            ]),
            Line::from(vec![
                Span::styled("Drag       ", Style::default().fg(Color::Yellow)),
                Span::raw("Select text"),
            ]),
            Line::from(vec![
                Span::styled("Alt/⌥+Drag ", Style::default().fg(Color::Yellow)),
                Span::raw("Box select"),
            ]),
            Line::from(""),
            Line::from(vec![InputMode::Focus.styled_label()]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Esc       ", Style::default().fg(Color::Yellow)),
                Span::raw("Exit focus mode"),
            ]),
            Line::from(vec![
                Span::styled("Shift-Tab ", Style::default().fg(Color::Yellow)),
                Span::raw("Exit focus mode"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Mouse ", Style::default().add_modifier(Modifier::BOLD)),
                InputMode::Focus.styled_label(),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Click/Drag ", Style::default().fg(Color::Yellow)),
                Span::raw("Forwarded to child process"),
            ]),
            Line::from(vec![
                Span::styled("Scroll     ", Style::default().fg(Color::Yellow)),
                Span::raw("Scroll output"),
            ]),
            Line::from(vec![
                Span::styled("Click left ", Style::default().fg(Color::Yellow)),
                Span::raw("Exit focus, select process"),
            ]),
            Line::from(vec![
                Span::styled("Click bar  ", Style::default().fg(Color::Yellow)),
                Span::raw("Exit focus mode"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Press any key to close",
                Style::default().fg(Color::DarkGray),
            )]),
        ];

        let total_lines = help_text.len() as u16;
        // Inner height = popup height - 2 (top/bottom border)
        let inner_height = popup_area.height.saturating_sub(2);
        let max_scroll = total_lines.saturating_sub(inner_height);
        let scroll = self.scroll.min(max_scroll);

        let title_right = if max_scroll > 0 {
            format!(" ↑↓ scroll ({}/{}) ", scroll + 1, max_scroll + 1)
        } else {
            String::new()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Help ")
            .title_bottom(Line::from(title_right).right_aligned());

        Paragraph::new(help_text)
            .block(block)
            .scroll((scroll, 0))
            .render(popup_area, buf);
    }
}

pub struct ShutdownPopup {
    stopped: usize,
    total: usize,
    remaining_ms: u64,
}

impl ShutdownPopup {
    pub fn new(stopped: usize, total: usize, remaining_ms: u64) -> Self {
        Self {
            stopped,
            total,
            remaining_ms,
        }
    }
}

impl Widget for ShutdownPopup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let status = format!("{}/{} | {}ms", self.stopped, self.total, self.remaining_ms);
        let header = "Sending SIGTERM...";
        let popup_width = (header.len() + 4).max(status.len() + 6) as u16;
        let popup_height = 6;
        let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(x, y, popup_width, popup_height);

        Clear.render(popup_area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!(" {} ", header),
                Style::default().fg(Color::Yellow),
            )),
            Line::from(format!("  {}  ", status)),
        ];
        Paragraph::new(text).block(block).render(popup_area, buf);
    }
}

pub struct RestartPopup {
    message: String,
}

impl RestartPopup {
    pub fn new(action: &RestartAction) -> Self {
        let message = match action {
            RestartAction::One(name) => format!("Restarting {}...", name),
            RestartAction::All(count) => {
                if *count == 1 {
                    "Restarting 1 process...".to_string()
                } else {
                    format!("Restarting {} processes...", count)
                }
            }
        };
        Self { message }
    }
}

impl Widget for RestartPopup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let popup_width = (self.message.len() + 6) as u16;
        let popup_height = 5;
        let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
        let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(x, y, popup_width, popup_height);

        Clear.render(popup_area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let text = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!(" {} ", self.message),
                Style::default().fg(Color::Yellow),
            )),
        ];
        Paragraph::new(text).block(block).render(popup_area, buf);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
