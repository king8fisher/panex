use crate::process::ProcessManager;
use crate::ui::InputMode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Widget},
};

pub struct ProcessList<'a> {
    manager: &'a ProcessManager,
    selected: usize,
    mode: InputMode,
}

impl<'a> ProcessList<'a> {
    pub fn new(manager: &'a ProcessManager, selected: usize, mode: InputMode) -> Self {
        Self {
            manager,
            selected,
            mode,
        }
    }
}

impl Widget for ProcessList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .manager
            .process_names()
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let process = self.manager.get_process(name).unwrap();
                let icon = process.status.icon();
                let color = process.status.color();

                let is_selected = i == self.selected;
                let style = if is_selected {
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let line = Line::from(vec![
                    Span::styled(format!("{} ", icon), Style::default().fg(color)),
                    Span::styled(name.clone(), style),
                ]);

                ListItem::new(line).style(style)
            })
            .collect();

        let border_color = match self.mode {
            InputMode::Normal => Color::Blue,
            InputMode::Focus => Color::DarkGray,
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title("Processes");

        let list = List::new(items).block(block);
        Widget::render(list, area, buf);
    }
}
