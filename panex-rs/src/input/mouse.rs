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
                    scroll_down(process, SCROLL_AMOUNT, visible_height);
                }
            }
        }
        MouseEventKind::Down(_) => {
            // Check if click is in process list area (first 20 columns)
            if event.column < 20 {
                let index = event.row as usize;
                if index < pm.process_count() {
                    app.selected_index = index;
                }
            }
        }
        _ => {}
    }
}
