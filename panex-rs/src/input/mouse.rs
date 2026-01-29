use crate::process::ProcessManager;
use crate::ui::output_panel::{scroll_down, scroll_up};
use crate::ui::App;
use crossterm::event::{MouseEvent, MouseEventKind};

const SCROLL_AMOUNT: usize = 3;

pub fn handle_mouse(
    event: MouseEvent,
    app: &mut App,
    pm: &mut ProcessManager,
    visible_height: usize,
    viewport_width: usize,
) {
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
        MouseEventKind::Down(_) => {
            let is_status_bar = event.row as usize >= visible_height;

            if is_status_bar {
                // Click on status bar - exit focus mode
                app.exit_focus();
            } else if event.column < 20 {
                // Click on left panel
                let index = event.row as usize;
                // Select process only if clicking on a valid row
                if index < pm.process_count() {
                    app.selected_index = index;
                }
                // Always exit focus when clicking left panel
                app.exit_focus();
            } else if event.column >= 21 {
                // Click on right panel (output) - enter focus mode
                app.enter_focus();
            }
        }
        _ => {}
    }
}
