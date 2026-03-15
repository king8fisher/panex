/// Library crate for integration testing.
/// Exposes buffer and config modules; the full app is in main.rs.
pub mod process {
    pub mod buffer;
}

pub mod config;

/// Search types and logic for testing.
/// Uses `include!` to share the source with the binary crate's `ui::search`.
pub mod search {
    include!("ui/search.rs");
}

/// Restart action enum for testing the restart popup.
pub mod restart {
    #[derive(Debug, Clone)]
    pub enum RestartAction {
        One(String),
        All(usize),
    }

    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Style},
        text::{Line, Span},
        widgets::{Block, Borders, Clear, Paragraph, Widget},
    };

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
}
