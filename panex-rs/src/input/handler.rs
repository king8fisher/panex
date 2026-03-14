use crate::input::clipboard::copy_to_clipboard;
use crate::input::selection::{
    extract_selected_text, visual_row_to_buffer_row, BufferPos, SelectionPhase,
};
use crate::process::ProcessManager;
use crate::ui::output_panel::{scroll_down, scroll_to_bottom, scroll_to_top, scroll_up};
use crate::ui::search::{find_matches, nearest_match_index};
use crate::ui::{App, InputMode};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

/// Returns Some((cols, rows)) if a resize event was received (for debouncing in main loop)
pub fn handle_event(
    event: Event,
    app: &mut App,
    pm: &mut ProcessManager,
    visible_height: usize,
    viewport_width: usize,
    panel_cols: u16,
) -> Option<(u16, u16)> {
    match event {
        Event::Key(key) => {
            handle_key(key, app, pm, visible_height, viewport_width);
            None
        }
        Event::Mouse(mouse) => {
            super::mouse::handle_mouse(mouse, app, pm, visible_height, viewport_width, panel_cols);
            None
        }
        Event::Resize(cols, rows) => Some((cols, rows)),
        _ => None,
    }
}

fn handle_key(
    key: KeyEvent,
    app: &mut App,
    pm: &mut ProcessManager,
    visible_height: usize,
    viewport_width: usize,
) {
    // Help popup: scroll or close
    if app.show_help {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                app.help_scroll = app.help_scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.help_scroll = app.help_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                app.help_scroll = app.help_scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                app.help_scroll = app.help_scroll.saturating_sub(10);
            }
            _ => {
                app.show_help = false;
            }
        }
        return;
    }

    match app.mode {
        InputMode::Browse => handle_browse_key(key, app, pm, visible_height, viewport_width),
        InputMode::Focus => handle_focus_key(key, app, pm),
    }
}

fn handle_browse_key(
    key: KeyEvent,
    app: &mut App,
    pm: &mut ProcessManager,
    visible_height: usize,
    viewport_width: usize,
) {
    let count = pm.process_count();
    let selected_name = pm.process_names().get(app.selected_index).cloned();

    // Handle search mode keys first
    if app.search.is_typing() {
        handle_search_typing(key, app, pm, visible_height, viewport_width);
        return;
    }
    if app.search.is_active() {
        handle_search_active(key, app, pm, visible_height, viewport_width);
        return;
    }

    // Handle selection mode keys first
    if app.selection.is_active() {
        // Ctrl-C with active selection = copy (not quit)
        let is_copy = matches!(key.code, KeyCode::Char('y') | KeyCode::Enter)
            || (matches!(key.code, KeyCode::Char('c'))
                && key.modifiers.contains(KeyModifiers::CONTROL));

        match key.code {
            KeyCode::Esc => {
                app.selection.clear();
                return;
            }
            _ if is_copy => {
                // Copy selection to clipboard
                if let Some(name) = &selected_name {
                    if let Some(process) = pm.get_process(name) {
                        let text =
                            extract_selected_text(&app.selection, process.buffer.get_all_lines());
                        if !text.is_empty() && copy_to_clipboard(&text) {
                            app.set_status("Copied!");
                        }
                    }
                }
                app.selection.clear();
                return;
            }
            // Movement keys while in visual select
            KeyCode::Up | KeyCode::Char('k') => {
                if app.selection.phase == SelectionPhase::Selecting {
                    let cur = app.selection.cursor;
                    if cur.row > 0 {
                        app.selection
                            .move_cursor(BufferPos::new(cur.row - 1, cur.col));
                    }
                }
                return;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.selection.phase == SelectionPhase::Selecting {
                    let cur = app.selection.cursor;
                    app.selection
                        .move_cursor(BufferPos::new(cur.row + 1, cur.col));
                }
                return;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if app.selection.phase == SelectionPhase::Selecting {
                    let cur = app.selection.cursor;
                    if cur.col > 0 {
                        app.selection
                            .move_cursor(BufferPos::new(cur.row, cur.col - 1));
                    }
                }
                return;
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if app.selection.phase == SelectionPhase::Selecting {
                    let cur = app.selection.cursor;
                    app.selection
                        .move_cursor(BufferPos::new(cur.row, cur.col + 1));
                }
                return;
            }
            _ => {
                // Any other key cancels selection
                app.selection.clear();
            }
        }
    }

    match key.code {
        // Quit
        KeyCode::Char('q') => app.quit(),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => app.quit(),

        // Navigation
        KeyCode::Up => app.select_prev(count),
        KeyCode::Down => app.select_next(count),

        // Focus
        KeyCode::Enter | KeyCode::Tab => app.enter_focus(),

        // Restart
        KeyCode::Char('r') => {
            if let Some(name) = selected_name {
                let _ = pm.restart_process(&name);
            }
        }
        KeyCode::Char('A') => {
            let _ = pm.restart_all();
        }

        // Kill
        KeyCode::Char('x') => {
            if let Some(name) = selected_name {
                let _ = pm.kill_process(&name);
            }
        }

        // Toggle wrap
        KeyCode::Char('w') => {
            if let Some(name) = selected_name {
                if let Some(process) = pm.get_process_mut(&name) {
                    process.wrap_enabled = !process.wrap_enabled;
                }
            }
        }

        // Visual select
        KeyCode::Char('v') => {
            if let Some(name) = &selected_name {
                if let Some(process) = pm.get_process(name) {
                    // Start char-wise visual at top-left of visible area
                    let pos = if process.wrap_enabled {
                        visual_row_to_buffer_row(
                            process.scroll_offset,
                            process.buffer.get_all_lines(),
                            viewport_width,
                        )
                    } else {
                        BufferPos::new(process.scroll_offset, 0)
                    };
                    app.selection.start_visual(pos, false);
                }
            }
        }
        KeyCode::Char('V') => {
            if let Some(name) = &selected_name {
                if let Some(process) = pm.get_process(name) {
                    // Start line-wise visual at current scroll position
                    let pos = if process.wrap_enabled {
                        visual_row_to_buffer_row(
                            process.scroll_offset,
                            process.buffer.get_all_lines(),
                            viewport_width,
                        )
                    } else {
                        BufferPos::new(process.scroll_offset, 0)
                    };
                    app.selection.start_visual(pos, true);
                }
            }
        }

        // Scrolling
        KeyCode::Char('g') => {
            // Toggle pin (auto_scroll)
            if let Some(name) = selected_name {
                if let Some(process) = pm.get_process_mut(&name) {
                    if process.auto_scroll {
                        process.auto_scroll = false;
                    } else {
                        scroll_to_bottom(process, visible_height, viewport_width);
                    }
                }
            }
        }
        KeyCode::Char('t') => {
            if let Some(name) = selected_name {
                if let Some(process) = pm.get_process_mut(&name) {
                    scroll_to_top(process);
                }
            }
        }
        KeyCode::Char('b') => {
            if let Some(name) = selected_name {
                if let Some(process) = pm.get_process_mut(&name) {
                    scroll_to_bottom(process, visible_height, viewport_width);
                }
            }
        }
        KeyCode::PageUp => {
            if let Some(name) = selected_name {
                if let Some(process) = pm.get_process_mut(&name) {
                    scroll_up(process, visible_height);
                }
            }
        }
        KeyCode::PageDown => {
            if let Some(name) = selected_name {
                if let Some(process) = pm.get_process_mut(&name) {
                    scroll_down(process, visible_height, visible_height, viewport_width);
                }
            }
        }

        // Search
        KeyCode::Char('/') => {
            if let Some(name) = &selected_name {
                if let Some(process) = pm.get_process(name) {
                    app.search = crate::ui::search::SearchState::new_typing(process.scroll_offset);
                }
            }
        }

        // Help
        KeyCode::Char('?') => app.toggle_help(),

        _ => {}
    }
}

fn handle_search_typing(
    key: KeyEvent,
    app: &mut App,
    pm: &mut ProcessManager,
    visible_height: usize,
    viewport_width: usize,
) {
    let selected_name = pm.process_names().get(app.selected_index).cloned();

    match key.code {
        KeyCode::Esc => {
            // Cancel search and restore scroll position
            if let Some(scroll) = app.search.cancel() {
                if let Some(name) = &selected_name {
                    if let Some(process) = pm.get_process_mut(name) {
                        process.scroll_offset = scroll;
                        process.auto_scroll = false;
                    }
                }
            }
        }
        KeyCode::Enter => {
            // Execute the search
            let query = app.search.query().to_string();
            if query.is_empty() {
                app.search.cancel();
                return;
            }
            if let Some(name) = &selected_name {
                if let Some(process) = pm.get_process(name) {
                    let matches = find_matches(&query, process.buffer.get_all_lines());
                    let saved_scroll = app.search.saved_scroll().unwrap_or(0);
                    if matches.is_empty() {
                        app.search = crate::ui::search::SearchState::new_active(
                            query,
                            vec![],
                            0,
                            saved_scroll,
                        );
                        app.set_status("No matches");
                    } else {
                        let current = nearest_match_index(&matches, process.scroll_offset);
                        // Scroll to show the current match
                        let match_row = matches[current].row;
                        app.search = crate::ui::search::SearchState::new_active(
                            query,
                            matches,
                            current,
                            saved_scroll,
                        );
                        if let Some(process) = pm.get_process_mut(name) {
                            scroll_to_match(process, match_row, visible_height, viewport_width);
                        }
                    }
                }
            }
        }
        KeyCode::Backspace => {
            app.search.pop_char();
        }
        KeyCode::Char(c) => {
            app.search.push_char(c);
        }
        _ => {}
    }
}

fn handle_search_active(
    key: KeyEvent,
    app: &mut App,
    pm: &mut ProcessManager,
    visible_height: usize,
    viewport_width: usize,
) {
    let selected_name = pm.process_names().get(app.selected_index).cloned();

    match key.code {
        KeyCode::Esc => {
            // Cancel search and restore scroll position
            if let Some(scroll) = app.search.cancel() {
                if let Some(name) = &selected_name {
                    if let Some(process) = pm.get_process_mut(name) {
                        process.scroll_offset = scroll;
                        process.auto_scroll = false;
                    }
                }
            }
        }
        KeyCode::Enter => {
            // Confirm search — stay at current match position
            app.search.confirm();
        }
        KeyCode::Char('n') => {
            app.search.next_match();
            if let Some(m) = app.search.current_match() {
                let row = m.row;
                if let Some(name) = &selected_name {
                    if let Some(process) = pm.get_process_mut(name) {
                        scroll_to_match(process, row, visible_height, viewport_width);
                    }
                }
            }
        }
        KeyCode::Char('N') => {
            app.search.prev_match();
            if let Some(m) = app.search.current_match() {
                let row = m.row;
                if let Some(name) = &selected_name {
                    if let Some(process) = pm.get_process_mut(name) {
                        scroll_to_match(process, row, visible_height, viewport_width);
                    }
                }
            }
        }
        _ => {
            // Any other key exits search mode
            app.search.confirm();
        }
    }
}

/// Scroll the output so that the given buffer row is visible
fn scroll_to_match(
    process: &mut crate::process::ManagedProcess,
    match_row: usize,
    visible_height: usize,
    _viewport_width: usize,
) {
    process.auto_scroll = false;
    // Center the match in the viewport if possible
    let half = visible_height / 2;
    if match_row > half {
        process.scroll_offset = match_row - half;
    } else {
        process.scroll_offset = 0;
    }
}

fn handle_focus_key(key: KeyEvent, app: &mut App, pm: &mut ProcessManager) {
    let selected_name = pm.process_names().get(app.selected_index).cloned();

    // Check per-process no_shift_tab (overrides global)
    let proc_no_shift_tab = selected_name
        .as_ref()
        .and_then(|n| pm.get_process(n))
        .map(|p| p.config.no_shift_tab)
        .unwrap_or(false);
    let no_shift_tab = app.no_shift_tab || proc_no_shift_tab;

    // Check for exit keys (unless no_shift_tab is set, then only mouse click exits)
    match key.code {
        KeyCode::Esc if !no_shift_tab => {
            app.exit_focus();
            return;
        }
        KeyCode::BackTab if !no_shift_tab => {
            app.exit_focus();
            return;
        }
        _ => {}
    }

    // Forward key to PTY
    if let Some(name) = selected_name {
        if let Some(bytes) = key_to_bytes(key) {
            let _ = pm.write_to_process(&name, &bytes);
        }
    }
}

fn key_to_bytes(key: KeyEvent) -> Option<Vec<u8>> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    let bytes = match key.code {
        KeyCode::Char(c) => {
            if ctrl {
                // Ctrl+A = 0x01, Ctrl+Z = 0x1A
                let ctrl_byte = (c.to_ascii_lowercase() as u8).saturating_sub(b'a' - 1);
                if alt {
                    vec![0x1b, ctrl_byte]
                } else {
                    vec![ctrl_byte]
                }
            } else if alt {
                vec![0x1b, c as u8]
            } else {
                c.to_string().into_bytes()
            }
        }
        KeyCode::Enter => vec![0x0d],
        KeyCode::Tab => vec![0x09],
        KeyCode::BackTab => vec![0x1b, b'[', b'Z'], // Shift-Tab (CSI Z)
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Delete => vec![0x1b, b'[', b'3', b'~'],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => vec![0x1b, b'[', b'A'],
        KeyCode::Down => vec![0x1b, b'[', b'B'],
        KeyCode::Right => vec![0x1b, b'[', b'C'],
        KeyCode::Left => vec![0x1b, b'[', b'D'],
        KeyCode::Home => vec![0x1b, b'[', b'H'],
        KeyCode::End => vec![0x1b, b'[', b'F'],
        KeyCode::PageUp => vec![0x1b, b'[', b'5', b'~'],
        KeyCode::PageDown => vec![0x1b, b'[', b'6', b'~'],
        KeyCode::Insert => vec![0x1b, b'[', b'2', b'~'],
        KeyCode::F(n) => match n {
            1 => vec![0x1b, b'O', b'P'],
            2 => vec![0x1b, b'O', b'Q'],
            3 => vec![0x1b, b'O', b'R'],
            4 => vec![0x1b, b'O', b'S'],
            5 => vec![0x1b, b'[', b'1', b'5', b'~'],
            6 => vec![0x1b, b'[', b'1', b'7', b'~'],
            7 => vec![0x1b, b'[', b'1', b'8', b'~'],
            8 => vec![0x1b, b'[', b'1', b'9', b'~'],
            9 => vec![0x1b, b'[', b'2', b'0', b'~'],
            10 => vec![0x1b, b'[', b'2', b'1', b'~'],
            11 => vec![0x1b, b'[', b'2', b'3', b'~'],
            12 => vec![0x1b, b'[', b'2', b'4', b'~'],
            _ => return None,
        },
        _ => return None,
    };

    Some(bytes)
}
