use crate::input::SelectionState;
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Browse,
    Focus,
}

impl InputMode {
    pub fn label(&self) -> &'static str {
        match self {
            InputMode::Browse => " BROWSE ",
            InputMode::Focus => " FOCUS  ",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragEdge {
    Top,
    Bottom,
}

pub struct App {
    pub selected_index: usize,
    pub mode: InputMode,
    pub show_help: bool,
    pub should_quit: bool,
    pub shutting_down: bool,
    pub shutdown_start: Option<Instant>,
    pub no_shift_tab: bool,
    pub selection: SelectionState,
    pub auto_copy: bool,
    pub status_message: Option<(String, Instant)>,
    pub drag_edge: Option<DragEdge>,
    pub last_edge_scroll: Option<Instant>,
    /// Mouse-down position pending selection (col, row) — selection starts after drag threshold
    pub pending_click: Option<(u16, u16)>,
}

impl App {
    pub fn new(no_shift_tab: bool, auto_copy: bool) -> Self {
        Self {
            selected_index: 0,
            mode: InputMode::Browse,
            show_help: false,
            should_quit: false,
            shutting_down: false,
            shutdown_start: None,
            no_shift_tab,
            selection: SelectionState::new(),
            auto_copy,
            status_message: None,
            drag_edge: None,
            last_edge_scroll: None,
            pending_click: None,
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
        self.selection.clear();
    }

    pub fn exit_focus(&mut self) {
        self.mode = InputMode::Browse;
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn quit(&mut self) {
        if !self.shutting_down {
            self.shutting_down = true;
            self.shutdown_start = Some(Instant::now());
        }
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_string(), Instant::now()));
    }

    /// Returns status message if still within display duration (2s)
    pub fn active_status(&self) -> Option<&str> {
        self.status_message.as_ref().and_then(|(msg, time)| {
            if time.elapsed().as_secs() < 2 {
                Some(msg.as_str())
            } else {
                None
            }
        })
    }
}
