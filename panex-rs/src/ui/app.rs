use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Browse,
    Focus,
}

impl InputMode {
    pub fn label(&self) -> &'static str {
        match self {
            InputMode::Browse => " BROWSE ",
            InputMode::Focus => " FOCUS ",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            InputMode::Browse => Color::Blue,
            InputMode::Focus => Color::Green,
        }
    }

    pub fn styled_label(&self) -> Span<'static> {
        Span::styled(
            self.label(),
            Style::default()
                .fg(Color::Black)
                .bg(self.color())
                .add_modifier(Modifier::BOLD),
        )
    }
}

pub struct App {
    pub selected_index: usize,
    pub mode: InputMode,
    pub show_help: bool,
    pub should_quit: bool,
    pub shutting_down: bool,
    pub no_shift_tab: bool,
}

impl App {
    pub fn new(no_shift_tab: bool) -> Self {
        Self {
            selected_index: 0,
            mode: InputMode::Browse,
            show_help: false,
            should_quit: false,
            shutting_down: false,
            no_shift_tab,
        }
    }

    pub fn select_next(&mut self, count: usize) {
        if count > 0 {
            self.selected_index = (self.selected_index + 1) % count;
        }
    }

    pub fn select_prev(&mut self, count: usize) {
        if count > 0 {
            self.selected_index = (self.selected_index + count - 1) % count;
        }
    }

    pub fn enter_focus(&mut self) {
        self.mode = InputMode::Focus;
    }

    pub fn exit_focus(&mut self) {
        self.mode = InputMode::Browse;
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn quit(&mut self) {
        self.shutting_down = true;
    }
}
