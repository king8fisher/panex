use crate::ui::InputMode;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

pub struct HelpPopup;

impl HelpPopup {
    pub fn new() -> Self {
        Self
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
            Line::from(vec![Span::styled(
                "Mouse",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Scroll    ", Style::default().fg(Color::Yellow)),
                Span::raw("Scroll output"),
            ]),
            Line::from(vec![
                Span::styled("Click     ", Style::default().fg(Color::Yellow)),
                Span::raw("Select process"),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Press any key to close",
                Style::default().fg(Color::DarkGray),
            )]),
        ];

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Help ");

        Paragraph::new(help_text)
            .block(block)
            .render(popup_area, buf);
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
