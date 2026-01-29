use crate::process::ProcessManager;
use crate::ui::InputMode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Widget},
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
        let width = area.width as usize;

        let items: Vec<ListItem> = self
            .manager
            .process_names()
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let process = self.manager.get_process(name).unwrap();
                let icon = process.status.icon();
                let status_color = process.status.color();
                let pin = if !process.auto_scroll { "⇅" } else { " " };

                let is_selected = i == self.selected;
                let bg_color = if is_selected {
                    match self.mode {
                        InputMode::Normal => Color::Blue,
                        InputMode::Focus => Color::DarkGray,
                    }
                } else {
                    Color::Reset
                };

                let style = Style::default().bg(bg_color);
                let bold_style = if is_selected {
                    style.add_modifier(Modifier::BOLD)
                } else {
                    style
                };

                // Calculate padding: icon(2) + name + spaces + pin(if any)
                let icon_width = 2; // icon + space
                let pin_width = if pin.is_empty() { 0 } else { 2 }; // emoji width
                let name_max = width.saturating_sub(icon_width + pin_width);
                let display_name: String = name.chars().take(name_max).collect();
                let name_len = display_name.chars().count();
                let padding = width.saturating_sub(icon_width + name_len + pin_width);

                let pin_style = if !process.auto_scroll {
                    Style::default().fg(Color::White).bg(Color::Red)
                } else {
                    style
                };

                let line = Line::from(vec![
                    Span::styled(
                        format!("{} ", icon),
                        Style::default().fg(status_color).bg(bg_color),
                    ),
                    Span::styled(display_name, bold_style),
                    Span::styled(" ".repeat(padding), style),
                    Span::styled(pin, pin_style),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items);
        Widget::render(list, area, buf);
    }
}
