use crate::process::ProcessManager;
use crate::ui::output_panel::{scroll_down, scroll_to_bottom, scroll_to_top, scroll_up};
use crate::ui::{App, InputMode};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

pub fn handle_event(
    event: Event,
    app: &mut App,
    pm: &mut ProcessManager,
    visible_height: usize,
) {
    match event {
        Event::Key(key) => handle_key(key, app, pm, visible_height),
        Event::Mouse(mouse) => super::mouse::handle_mouse(mouse, app, pm, visible_height),
        Event::Resize(cols, rows) => {
            // Output panel: total - process list (20) - delimiter (1), -1 row for status bar
            pm.resize(cols.saturating_sub(21), rows.saturating_sub(1));
        }
        _ => {}
    }
}

fn handle_key(key: KeyEvent, app: &mut App, pm: &mut ProcessManager, visible_height: usize) {
    // Close help on any key
    if app.show_help {
        app.show_help = false;
        return;
    }

    match app.mode {
        InputMode::Browse => handle_browse_key(key, app, pm, visible_height),
        InputMode::Focus => handle_focus_key(key, app, pm),
    }
}

fn handle_browse_key(key: KeyEvent, app: &mut App, pm: &mut ProcessManager, visible_height: usize) {
    let count = pm.process_count();
    let selected_name = pm.process_names().get(app.selected_index).cloned();

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

        // Scrolling
        KeyCode::Char('g') => {
            // Toggle pin (auto_scroll)
            if let Some(name) = selected_name {
                if let Some(process) = pm.get_process_mut(&name) {
                    if process.auto_scroll {
                        process.auto_scroll = false;
                    } else {
                        scroll_to_bottom(process, visible_height);
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
                    scroll_to_bottom(process, visible_height);
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
                    scroll_down(process, visible_height, visible_height);
                }
            }
        }

        // Help
        KeyCode::Char('?') => app.toggle_help(),

        _ => {}
    }
}

fn handle_focus_key(key: KeyEvent, app: &mut App, pm: &mut ProcessManager) {
    // Check for exit keys
    match key.code {
        KeyCode::Esc => {
            app.exit_focus();
            return;
        }
        KeyCode::BackTab if !app.no_shift_tab => {
            app.exit_focus();
            return;
        }
        _ => {}
    }

    // Forward key to PTY
    let selected_name = pm.process_names().get(app.selected_index).cloned();
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
        KeyCode::F(n) => {
            let seq = match n {
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
            };
            seq
        }
        _ => return None,
    };

    Some(bytes)
}
