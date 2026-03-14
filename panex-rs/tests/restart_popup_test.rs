use panex::restart::{RestartAction, RestartPopup};
use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

#[test]
fn restart_popup_displays_single_process_name() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let size = f.area();
            f.render_widget(
                RestartPopup::new(&RestartAction::One("api".to_string())),
                size,
            );
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buf);
    assert!(
        content.contains("Restarting api..."),
        "Expected 'Restarting api...' in popup, got:\n{}",
        content
    );
}

#[test]
fn restart_popup_displays_all() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let size = f.area();
            f.render_widget(RestartPopup::new(&RestartAction::All), size);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buf);
    assert!(
        content.contains("Restarting all..."),
        "Expected 'Restarting all...' in popup, got:\n{}",
        content
    );
}

#[test]
fn restart_action_none_means_no_popup() {
    let restarting: Option<RestartAction> = None;

    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let size = f.area();
            if let Some(ref action) = restarting {
                f.render_widget(RestartPopup::new(action), size);
            }
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buf);
    assert!(
        !content.contains("Restarting"),
        "Expected no 'Restarting' text when state is None, got:\n{}",
        content
    );
}

#[test]
fn restart_popup_auto_dismisses_after_one_frame() {
    let mut restarting: Option<RestartAction> = Some(RestartAction::One("web".to_string()));

    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    // Frame 1: popup visible
    terminal
        .draw(|f| {
            let size = f.area();
            if let Some(ref action) = restarting {
                f.render_widget(RestartPopup::new(action), size);
            }
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buf);
    assert!(
        content.contains("Restarting web..."),
        "Frame 1 should show popup"
    );

    // After render, clear the state (as main.rs does)
    if restarting.is_some() {
        restarting = None;
    }

    // Frame 2: popup gone
    terminal
        .draw(|f| {
            let size = f.area();
            if let Some(ref action) = restarting {
                f.render_widget(RestartPopup::new(action), size);
            }
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buf);
    assert!(
        !content.contains("Restarting"),
        "Frame 2 should not show popup after clearing state"
    );
}

fn buffer_to_string(buf: &Buffer) -> String {
    let mut s = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = &buf[(x, y)];
            s.push_str(cell.symbol());
        }
        s.push('\n');
    }
    s
}
