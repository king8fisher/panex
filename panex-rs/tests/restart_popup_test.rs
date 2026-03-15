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
fn restart_popup_displays_all_with_count() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let size = f.area();
            f.render_widget(RestartPopup::new(&RestartAction::All(3)), size);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buf);
    assert!(
        content.contains("Restarting 3 processes..."),
        "Expected 'Restarting 3 processes...' in popup, got:\n{}",
        content
    );
}

#[test]
fn restart_popup_displays_all_singular() {
    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let size = f.area();
            f.render_widget(RestartPopup::new(&RestartAction::All(1)), size);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buf);
    assert!(
        content.contains("Restarting 1 process..."),
        "Expected 'Restarting 1 process...' (singular) in popup, got:\n{}",
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
fn restart_popup_persists_while_active_and_clears_after_phase_transition() {
    // Simulate the new phase-based lifecycle:
    // Pending → Active(now) → None (after timeout)

    let action = RestartAction::All(2);

    let backend = TestBackend::new(40, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    // Phase: Pending — popup should render
    let mut restarting: Option<(RestartAction, &str)> = Some((action.clone(), "Pending"));

    terminal
        .draw(|f| {
            let size = f.area();
            if let Some((ref act, _)) = restarting {
                f.render_widget(RestartPopup::new(act), size);
            }
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buf);
    assert!(
        content.contains("Restarting 2 processes..."),
        "Pending phase should show popup"
    );

    // Transition to Active (simulate restart execution completed)
    restarting = Some((RestartAction::All(2), "Active"));

    // Frame 2: popup still visible during Active
    terminal
        .draw(|f| {
            let size = f.area();
            if let Some((ref act, _)) = restarting {
                f.render_widget(RestartPopup::new(act), size);
            }
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buf);
    assert!(
        content.contains("Restarting 2 processes..."),
        "Active phase should still show popup"
    );

    // Transition to None (timeout elapsed)
    restarting = None;

    // Frame 3: popup gone
    terminal
        .draw(|f| {
            let size = f.area();
            if let Some((ref act, _)) = restarting {
                f.render_widget(RestartPopup::new(act), size);
            }
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buf);
    assert!(
        !content.contains("Restarting"),
        "After clearing state, popup should be gone"
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
