use crate::input::clipboard::copy_to_clipboard;
use crate::input::selection::{
    clamp_pos, expand_to_word, extract_selected_text, screen_to_buffer, screen_to_buffer_wrapped,
    visual_to_buffer, BufferPos, SelectionPhase,
};
use crate::process::ProcessManager;
use crate::ui::app::DragEdge;
use crate::ui::output_panel::{scroll_down, scroll_up};
use crate::ui::{App, InputMode};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};

const SCROLL_AMOUNT: usize = 3;
/// Minimum cell distance before a drag becomes a selection
const DRAG_THRESHOLD: u16 = 2;

/// First column of the gutter (between process list and output panel)
fn gutter_start(panel_cols: u16) -> u16 {
    panel_cols.saturating_sub(1)
}

/// Left edge of output panel (process list width + delimiter)
fn output_panel_x(panel_cols: u16) -> u16 {
    panel_cols + 1
}

/// Encode a mouse event as SGR (mode 1006) escape sequence for forwarding to child PTY.
/// Coordinates are translated so the output panel's top-left is (1,1).
fn mouse_to_sgr(event: &MouseEvent, visible_height: usize, panel_cols: u16) -> Option<Vec<u8>> {
    let opx = output_panel_x(panel_cols);
    // Only forward events in the output panel area
    if event.column < opx || event.row as usize >= visible_height {
        return None;
    }
    let col = event.column - opx + 1; // 1-based
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
    panel_cols: u16,
) {
    let gutter = gutter_start(panel_cols);
    let opx = output_panel_x(panel_cols);
    // Help popup: scroll or close on click
    if app.show_help {
        match event.kind {
            MouseEventKind::ScrollUp => {
                app.help_scroll = app.help_scroll.saturating_sub(SCROLL_AMOUNT as u16);
            }
            MouseEventKind::ScrollDown => {
                app.help_scroll = app.help_scroll.saturating_add(SCROLL_AMOUNT as u16);
            }
            MouseEventKind::Down(_) => {
                app.show_help = false;
            }
            _ => {}
        }
        return;
    }

    // In Focus mode, forward non-scroll mouse events to the child PTY.
    // Scroll is handled uniformly below (forwarded if alternate screen, else viewport).
    if app.mode == InputMode::Focus
        && !matches!(
            event.kind,
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
        )
    {
        // Click on process list or gutter exits focus
        if matches!(event.kind, MouseEventKind::Down(MouseButton::Left)) && event.column < opx {
            if event.column < gutter {
                let index = event.row as usize;
                if index < pm.process_count() {
                    app.selected_index = index;
                }
            }
            app.exit_focus();
            return;
        }
        // Click on status bar exits focus
        if matches!(event.kind, MouseEventKind::Down(MouseButton::Left))
            && event.row as usize >= visible_height
        {
            app.exit_focus();
            return;
        }

        let selected_name = pm.process_names().get(app.selected_index).cloned();
        if let Some(name) = selected_name {
            if let Some(bytes) = mouse_to_sgr(&event, visible_height, panel_cols) {
                let _ = pm.write_to_process(&name, &bytes);
            }
        }
        return;
    }

    let selected_name = pm.process_names().get(app.selected_index).cloned();

    match event.kind {
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
            if let Some(name) = &selected_name {
                if let Some(process) = pm.get_process(name) {
                    if process.buffer.is_alternate_screen() {
                        // TUI app: forward scroll to child PTY
                        if let Some(bytes) = mouse_to_sgr(&event, visible_height, panel_cols) {
                            let _ = pm.write_to_process(name, &bytes);
                        }
                    } else if matches!(event.kind, MouseEventKind::ScrollUp) {
                        if let Some(process) = pm.get_process_mut(name) {
                            scroll_up(process, SCROLL_AMOUNT);
                        }
                    } else if let Some(process) = pm.get_process_mut(name) {
                        scroll_down(process, SCROLL_AMOUNT, visible_height, viewport_width);
                    }
                }
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            let is_status_bar = event.row as usize >= visible_height;

            if is_status_bar {
                app.exit_focus();
                app.selection.clear();
                app.pending_click = None;
            } else if event.column < gutter {
                // Click on process list (col 0–18) - select process
                let index = event.row as usize;
                if index < pm.process_count() {
                    app.selected_index = index;
                }
                app.exit_focus();
                app.selection.clear();
                app.pending_click = None;
            } else if event.column < opx {
                // Click on gutter (col 19–20) - start line selection
                app.selection.clear();
                app.pending_click = None;
                if let Some(name) = &selected_name {
                    if let Some(process) = pm.get_process(name) {
                        let pos = if process.wrap_enabled {
                            screen_to_buffer_wrapped(
                                opx,
                                event.row,
                                opx,
                                process.scroll_offset,
                                process.buffer.get_all_lines(),
                                viewport_width,
                            )
                        } else {
                            screen_to_buffer(
                                opx,
                                event.row,
                                opx,
                                process.scroll_offset,
                                viewport_width,
                            )
                        };
                        let pos = clamp_pos(pos, process.buffer.get_all_lines());
                        app.selection.start_visual(pos, true);
                    }
                }
            } else {
                // Click on gutter or output panel - compute buffer position NOW
                // (before auto-scroll can shift scroll_offset between click and drag)
                app.selection.clear();

                let alt = event.modifiers.contains(KeyModifiers::ALT);
                if let Some(name) = &selected_name {
                    if let Some(process) = pm.get_process(name) {
                        let raw_pos = if process.wrap_enabled {
                            screen_to_buffer_wrapped(
                                event.column.max(opx),
                                event.row,
                                opx,
                                process.scroll_offset,
                                process.buffer.get_all_lines(),
                                viewport_width,
                            )
                        } else {
                            screen_to_buffer(
                                event.column.max(opx),
                                event.row,
                                opx,
                                process.scroll_offset,
                                viewport_width,
                            )
                        };
                        // Box selection keeps raw column; char/line selection clamps
                        let pos = if alt {
                            raw_pos
                        } else {
                            clamp_pos(raw_pos, process.buffer.get_all_lines())
                        };
                        // Save screen coords (for drag threshold) + buffer pos (for anchor)
                        app.pending_click = Some((event.column, event.row, pos, alt));

                        if !alt {
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
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some((click_col, click_row, anchor_pos, box_select)) = app.pending_click {
                // Check drag threshold
                let dx = (event.column as i32 - click_col as i32).unsigned_abs() as u16;
                let dy = (event.row as i32 - click_row as i32).unsigned_abs() as u16;
                if dx < DRAG_THRESHOLD && dy < DRAG_THRESHOLD {
                    return; // Not far enough yet
                }

                // Threshold exceeded: use buffer position captured at click time
                // (immune to auto-scroll changes between click and drag)
                app.pending_click = None;
                if box_select {
                    app.selection.begin_box_drag(anchor_pos);
                } else {
                    app.selection.begin_drag(anchor_pos);
                }
            }

            if app.selection.phase != SelectionPhase::Selecting {
                return;
            }
            if let Some(name) = &selected_name {
                let row = event.row as usize;

                // Set edge-scroll state (main loop handles timed scrolling)
                let at_edge = if row == 0 {
                    Some(DragEdge::Top)
                } else if row >= visible_height {
                    Some(DragEdge::Bottom)
                } else {
                    None
                };

                if let Some(edge) = at_edge {
                    if app.drag_edge != Some(edge) {
                        // Just entered edge — compute adaptive interval from approach velocity
                        app.edge_scroll_interval =
                            if let Some((prev_row, prev_time)) = app.last_drag_row {
                                let dy = (event.row as i32 - prev_row as i32).unsigned_abs().max(1);
                                let dt_ms = prev_time.elapsed().as_millis().max(1) as u32;
                                // ms per row of cursor movement; clamp interval to 30..300ms
                                let ms_per_row = dt_ms / dy;
                                Duration::from_millis((ms_per_row * 2).clamp(30, 300) as u64)
                            } else {
                                EDGE_SCROLL_BASE
                            };

                        // Immediate first scroll
                        if let Some(process) = pm.get_process_mut(name) {
                            match edge {
                                DragEdge::Top => scroll_up(process, 1),
                                DragEdge::Bottom => {
                                    scroll_down(process, 1, visible_height, viewport_width)
                                }
                            }
                        }
                        app.last_edge_scroll = Some(Instant::now());
                    }
                    app.drag_edge = Some(edge);
                    app.last_drag_row = None; // stop tracking while at edge
                } else {
                    app.drag_edge = None;
                    app.last_drag_row = Some((event.row, Instant::now()));
                }

                if let Some(process) = pm.get_process(name) {
                    let last_row = visible_height.saturating_sub(1);
                    let clamped_row = row.min(last_row) as u16;

                    let pos = if row >= visible_height {
                        // Dragging onto/past status bar = end of last visible line
                        if process.wrap_enabled {
                            let visual_row = last_row + process.scroll_offset;
                            let mut p = visual_to_buffer(
                                visual_row,
                                0,
                                process.buffer.get_all_lines(),
                                viewport_width,
                            );
                            p.col = usize::MAX;
                            p
                        } else {
                            let buf_row = last_row + process.scroll_offset;
                            BufferPos::new(buf_row, usize::MAX)
                        }
                    } else if event.column < opx {
                        // Dragging to gutter = end of previous line
                        if process.wrap_enabled {
                            let visual_row = clamped_row as usize + process.scroll_offset;
                            let p = visual_to_buffer(
                                visual_row,
                                0,
                                process.buffer.get_all_lines(),
                                viewport_width,
                            );
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
                    } else {
                        let raw = if process.wrap_enabled {
                            screen_to_buffer_wrapped(
                                event.column,
                                clamped_row,
                                opx,
                                process.scroll_offset,
                                process.buffer.get_all_lines(),
                                viewport_width,
                            )
                        } else {
                            screen_to_buffer(
                                event.column,
                                clamped_row,
                                opx,
                                process.scroll_offset,
                                viewport_width,
                            )
                        };
                        // Box selection keeps raw columns for rectangular shape
                        if app.selection.mode == crate::input::selection::SelectionMode::Box {
                            raw
                        } else {
                            clamp_pos(raw, process.buffer.get_all_lines())
                        }
                    };
                    app.selection.update_mouse_drag(pos);
                }
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            app.drag_edge = None;
            app.last_drag_row = None;

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
                        let text =
                            extract_selected_text(&app.selection, process.buffer.get_all_lines());
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

const EDGE_SCROLL_BASE: Duration = Duration::from_millis(300);

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
        .map(|t| t.elapsed() >= app.edge_scroll_interval)
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
                DragEdge::Top => visual_to_buffer(
                    process.scroll_offset,
                    0,
                    process.buffer.get_all_lines(),
                    viewport_width,
                ),
                DragEdge::Bottom => {
                    let visual_row = visible_height.saturating_sub(1) + process.scroll_offset;
                    let mut p = visual_to_buffer(
                        visual_row,
                        0,
                        process.buffer.get_all_lines(),
                        viewport_width,
                    );
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
