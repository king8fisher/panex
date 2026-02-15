use crate::input::clipboard::copy_to_clipboard;
use crate::input::selection::{
    expand_to_word, extract_selected_text, screen_to_buffer, screen_to_buffer_wrapped,
    visual_to_buffer, BufferPos, SelectionPhase,
};
use crate::process::ProcessManager;
use crate::ui::app::DragEdge;
use crate::ui::output_panel::{scroll_down, scroll_up};
use crate::ui::{App, InputMode};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};

const SCROLL_AMOUNT: usize = 3;
/// Left edge of output panel (process list width + delimiter)
const OUTPUT_PANEL_X: u16 = 21;
/// Columns that act as gutter between panels (last process list col + delimiter)
const GUTTER_START: u16 = 19;
/// Minimum cell distance before a drag becomes a selection
const DRAG_THRESHOLD: u16 = 2;

/// Encode a mouse event as SGR (mode 1006) escape sequence for forwarding to child PTY.
/// Coordinates are translated so the output panel's top-left is (1,1).
fn mouse_to_sgr(event: &MouseEvent, visible_height: usize) -> Option<Vec<u8>> {
    // Only forward events in the output panel area
    if event.column < OUTPUT_PANEL_X || event.row as usize >= visible_height {
        return None;
    }
    let col = event.column - OUTPUT_PANEL_X + 1; // 1-based
    let row = event.row + 1; // 1-based

    let (button, press) = match event.kind {
        MouseEventKind::Down(MouseButton::Left) => (0, true),
        MouseEventKind::Down(MouseButton::Middle) => (1, true),
        MouseEventKind::Down(MouseButton::Right) => (2, true),
        MouseEventKind::Up(MouseButton::Left) => (0, false),
        MouseEventKind::Up(MouseButton::Middle) => (1, false),
        MouseEventKind::Up(MouseButton::Right) => (2, false),
        MouseEventKind::Drag(MouseButton::Left) => (32, true),
        MouseEventKind::Drag(MouseButton::Middle) => (33, true),
        MouseEventKind::Drag(MouseButton::Right) => (34, true),
        MouseEventKind::ScrollUp => (64, true),
        MouseEventKind::ScrollDown => (65, true),
        MouseEventKind::Moved => (35, true),
        _ => return None,
    };

    let suffix = if press { 'M' } else { 'm' };
    Some(format!("\x1b[<{};{};{}{}", button, col, row, suffix).into_bytes())
}

pub fn handle_mouse(
    event: MouseEvent,
    app: &mut App,
    pm: &mut ProcessManager,
    visible_height: usize,
    viewport_width: usize,
) {
    // In Focus mode, forward non-scroll mouse events to the child PTY.
    // Scroll wheel always stays with panex for viewport scrolling.
    if app.mode == InputMode::Focus {
        let is_scroll = matches!(event.kind, MouseEventKind::ScrollUp | MouseEventKind::ScrollDown);

        if !is_scroll {
            // Click on process list exits focus
            if matches!(event.kind, MouseEventKind::Down(MouseButton::Left)) && event.column < GUTTER_START {
                let index = event.row as usize;
                if index < pm.process_count() {
                    app.selected_index = index;
                }
                app.exit_focus();
                return;
            }
            // Click on status bar exits focus
            if matches!(event.kind, MouseEventKind::Down(MouseButton::Left)) && event.row as usize >= visible_height {
                app.exit_focus();
                return;
            }

            let selected_name = pm.process_names().get(app.selected_index).cloned();
            if let Some(name) = selected_name {
                if let Some(bytes) = mouse_to_sgr(&event, visible_height) {
                    let _ = pm.write_to_process(&name, &bytes);
                }
            }
            return;
        }
        // Scroll events fall through to normal handling below
    }

    let selected_name = pm.process_names().get(app.selected_index).cloned();

    match event.kind {
        MouseEventKind::ScrollUp => {
            if let Some(name) = selected_name {
                if let Some(process) = pm.get_process_mut(&name) {
                    scroll_up(process, SCROLL_AMOUNT);
                }
            }
        }
        MouseEventKind::ScrollDown => {
            if let Some(name) = selected_name {
                if let Some(process) = pm.get_process_mut(&name) {
                    scroll_down(process, SCROLL_AMOUNT, visible_height, viewport_width);
                }
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            let is_status_bar = event.row as usize >= visible_height;

            if is_status_bar {
                app.exit_focus();
                app.selection.clear();
                app.pending_click = None;
            } else if event.column < GUTTER_START {
                // Click on left panel - select process
                let index = event.row as usize;
                if index < pm.process_count() {
                    app.selected_index = index;
                }
                app.exit_focus();
                app.selection.clear();
                app.pending_click = None;
            } else {
                // Click on gutter or output panel - record pending click
                app.pending_click = Some((event.column, event.row));
                app.selection.clear();

                // Handle double/triple click immediately (word/line select)
                if let Some(name) = &selected_name {
                    if let Some(process) = pm.get_process(name) {
                        let pos = if process.wrap_enabled {
                            screen_to_buffer_wrapped(
                                event.column.max(OUTPUT_PANEL_X),
                                event.row,
                                OUTPUT_PANEL_X,
                                process.scroll_offset,
                                process.buffer.get_all_lines(),
                                viewport_width,
                            )
                        } else {
                            screen_to_buffer(
                                event.column.max(OUTPUT_PANEL_X),
                                event.row,
                                OUTPUT_PANEL_X,
                                process.scroll_offset,
                                viewport_width,
                            )
                        };
                        app.selection
                            .start_mouse_select(pos, event.column, event.row);

                        // Double/triple click: selection starts immediately
                        if app.selection.phase == SelectionPhase::Selected {
                            app.pending_click = None; // Not a simple click
                            if matches!(
                                app.selection.mode,
                                crate::input::selection::SelectionMode::Char
                            ) {
                                let (start, end) =
                                    expand_to_word(pos, process.buffer.get_all_lines());
                                app.selection.anchor = start;
                                app.selection.cursor = end;
                            }
                        } else {
                            // Single click: don't start selection yet, wait for drag threshold
                            app.selection.clear();
                        }
                    }
                }
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some((click_col, click_row)) = app.pending_click {
                // Check drag threshold
                let dx = (event.column as i32 - click_col as i32).unsigned_abs() as u16;
                let dy = (event.row as i32 - click_row as i32).unsigned_abs() as u16;
                if dx < DRAG_THRESHOLD && dy < DRAG_THRESHOLD {
                    return; // Not far enough yet
                }

                // Threshold exceeded: start selection from the original click position
                app.pending_click = None;
                if let Some(name) = &selected_name {
                    if let Some(process) = pm.get_process(name) {
                        let anchor_pos = if process.wrap_enabled {
                            screen_to_buffer_wrapped(
                                click_col.max(OUTPUT_PANEL_X),
                                click_row,
                                OUTPUT_PANEL_X,
                                process.scroll_offset,
                                process.buffer.get_all_lines(),
                                viewport_width,
                            )
                        } else {
                            screen_to_buffer(
                                click_col.max(OUTPUT_PANEL_X),
                                click_row,
                                OUTPUT_PANEL_X,
                                process.scroll_offset,
                                viewport_width,
                            )
                        };
                        app.selection.begin_drag(anchor_pos);
                    }
                }
            }

            if app.selection.phase != SelectionPhase::Selecting {
                return;
            }
            if let Some(name) = &selected_name {
                let row = event.row as usize;

                // Set edge-scroll state (main loop handles timed scrolling)
                if row == 0 {
                    if app.drag_edge != Some(DragEdge::Top) {
                        if let Some(process) = pm.get_process_mut(name) {
                            scroll_up(process, 1);
                        }
                        app.last_edge_scroll = Some(Instant::now());
                    }
                    app.drag_edge = Some(DragEdge::Top);
                } else if row >= visible_height {
                    if app.drag_edge != Some(DragEdge::Bottom) {
                        if let Some(process) = pm.get_process_mut(name) {
                            scroll_down(process, 1, visible_height, viewport_width);
                        }
                        app.last_edge_scroll = Some(Instant::now());
                    }
                    app.drag_edge = Some(DragEdge::Bottom);
                } else {
                    app.drag_edge = None;
                }

                if let Some(process) = pm.get_process(name) {
                    let last_row = visible_height.saturating_sub(1);
                    let clamped_row = row.min(last_row) as u16;

                    let pos = if row >= visible_height {
                        // Dragging onto/past status bar = end of last visible line
                        if process.wrap_enabled {
                            let visual_row = last_row + process.scroll_offset;
                            let mut p = visual_to_buffer(visual_row, 0, process.buffer.get_all_lines(), viewport_width);
                            p.col = usize::MAX;
                            p
                        } else {
                            let buf_row = last_row + process.scroll_offset;
                            BufferPos::new(buf_row, usize::MAX)
                        }
                    } else if event.column <= GUTTER_START {
                        // Dragging to gutter = end of previous line
                        if process.wrap_enabled {
                            let visual_row = clamped_row as usize + process.scroll_offset;
                            let p = visual_to_buffer(visual_row, 0, process.buffer.get_all_lines(), viewport_width);
                            if p.row > 0 || p.col > 0 {
                                // Go to end of previous buffer row
                                if p.col > 0 {
                                    // We're in the middle of a wrapped line; previous visual line is same buffer row
                                    BufferPos::new(p.row, p.col.saturating_sub(1))
                                } else if p.row > 0 {
                                    BufferPos::new(p.row - 1, usize::MAX)
                                } else {
                                    BufferPos::new(0, 0)
                                }
                            } else {
                                BufferPos::new(0, 0)
                            }
                        } else {
                            let buf_row = clamped_row as usize + process.scroll_offset;
                            if buf_row > 0 {
                                BufferPos::new(buf_row - 1, usize::MAX)
                            } else {
                                BufferPos::new(0, 0)
                            }
                        }
                    } else if process.wrap_enabled {
                        screen_to_buffer_wrapped(
                            event.column,
                            clamped_row,
                            OUTPUT_PANEL_X,
                            process.scroll_offset,
                            process.buffer.get_all_lines(),
                            viewport_width,
                        )
                    } else {
                        screen_to_buffer(
                            event.column,
                            clamped_row,
                            OUTPUT_PANEL_X,
                            process.scroll_offset,
                            viewport_width,
                        )
                    };
                    app.selection.update_mouse_drag(pos);
                }
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.drag_edge = None;

            if app.pending_click.is_some() {
                // Click without exceeding drag threshold → enter focus
                app.pending_click = None;
                app.selection.clear();
                app.enter_focus();
                return;
            }

            app.selection.finish_mouse_select();

            if app.auto_copy && app.selection.is_active() {
                if let Some(name) = &selected_name {
                    if let Some(process) = pm.get_process(name) {
                        let text = extract_selected_text(
                            &app.selection,
                            process.buffer.get_all_lines(),
                        );
                        if !text.is_empty() && copy_to_clipboard(&text) {
                            app.set_status("Copied!");
                        }
                        app.selection.clear();
                    }
                }
            }
        }
        MouseEventKind::Down(_) => {
            app.selection.clear();
            app.pending_click = None;
        }
        _ => {}
    }
}

const EDGE_SCROLL_INTERVAL: Duration = Duration::from_millis(300);

/// Called from the main loop on periodic tick to continue edge-scrolling
pub fn tick_edge_scroll(
    app: &mut App,
    pm: &mut ProcessManager,
    visible_height: usize,
    viewport_width: usize,
) {
    let edge = match app.drag_edge {
        Some(e) => e,
        None => return,
    };

    let due = app
        .last_edge_scroll
        .map(|t| t.elapsed() >= EDGE_SCROLL_INTERVAL)
        .unwrap_or(true);
    if !due {
        return;
    }

    let selected_name = pm.process_names().get(app.selected_index).cloned();
    let Some(name) = selected_name else { return };

    match edge {
        DragEdge::Top => {
            if let Some(process) = pm.get_process_mut(&name) {
                scroll_up(process, 1);
            }
        }
        DragEdge::Bottom => {
            if let Some(process) = pm.get_process_mut(&name) {
                scroll_down(process, 1, visible_height, viewport_width);
            }
        }
    }
    app.last_edge_scroll = Some(Instant::now());

    // Update selection cursor to track the scroll
    if let Some(process) = pm.get_process(&name) {
        let pos = if process.wrap_enabled {
            match edge {
                DragEdge::Top => {
                    visual_to_buffer(process.scroll_offset, 0, process.buffer.get_all_lines(), viewport_width)
                }
                DragEdge::Bottom => {
                    let visual_row = visible_height.saturating_sub(1) + process.scroll_offset;
                    let mut p = visual_to_buffer(visual_row, 0, process.buffer.get_all_lines(), viewport_width);
                    p.col = usize::MAX;
                    p
                }
            }
        } else {
            match edge {
                DragEdge::Top => BufferPos::new(process.scroll_offset, 0),
                DragEdge::Bottom => {
                    let buf_row = visible_height.saturating_sub(1) + process.scroll_offset;
                    BufferPos::new(buf_row, usize::MAX)
                }
            }
        };
        app.selection.update_mouse_drag(pos);
    }
}
