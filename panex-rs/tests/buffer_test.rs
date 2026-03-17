use panex::process::buffer::TerminalBuffer;

/// Helper: create a buffer, write data, return test string
fn run(cols: usize, rows: usize, input: &[u8]) -> String {
    let mut buf = TerminalBuffer::new(cols, rows);
    buf.write(input);
    buf.to_test_string()
}

#[test]
fn simple_text() {
    let output = run(80, 24, b"hello\nworld");
    insta::assert_snapshot!(output, @r"
    hello
    world
    ");
}

#[test]
fn ansi_color_codes() {
    // Red text followed by reset — to_test_string strips styles, shows plain text
    let output = run(80, 24, b"\x1b[31mred\x1b[0m normal");
    insta::assert_snapshot!(output, @"red normal");
}

#[test]
fn cursor_movement() {
    // Move cursor to row 2 col 5 (1-indexed: ESC[2;5H) then print
    let output = run(80, 24, b"\x1b[2;5H*");
    // Row 1 (0-indexed) is empty, row 2 has * at column 5 (4 spaces + *)
    assert_eq!(output, "\n    *");
}

#[test]
fn carriage_return_and_line_clear() {
    // Write text, CR back to col 0, clear line (ESC[2K), write new text
    let output = run(80, 24, b"old text\r\x1b[2Knew text");
    insta::assert_snapshot!(output, @"new text");
}

#[test]
fn alternate_screen_clears() {
    // Write text, enter alternate screen (clears), write new content
    let output = run(80, 24, b"before\x1b[?1049hafter");
    insta::assert_snapshot!(output, @"after");
}

// --- Mouse mode tracking tests (Step 8) ---

#[test]
fn mouse_mode_default_off() {
    let buf = TerminalBuffer::new(80, 24);
    assert!(!buf.wants_mouse(), "fresh buffer should not want mouse");
}

#[test]
fn mouse_mode_normal_tracking() {
    let mut buf = TerminalBuffer::new(80, 24);
    buf.write(b"\x1b[?1000h");
    assert!(buf.wants_mouse(), "DECSET 1000 should enable mouse");
}

#[test]
fn mouse_mode_normal_tracking_off() {
    let mut buf = TerminalBuffer::new(80, 24);
    buf.write(b"\x1b[?1000h");
    assert!(buf.wants_mouse());
    buf.write(b"\x1b[?1000l");
    assert!(!buf.wants_mouse(), "DECRST 1000 should disable mouse");
}

#[test]
fn mouse_mode_any_event_tracking() {
    let mut buf = TerminalBuffer::new(80, 24);
    buf.write(b"\x1b[?1003h");
    assert!(buf.wants_mouse(), "DECSET 1003 should enable mouse");
}

#[test]
fn mouse_mode_x10_tracking() {
    let mut buf = TerminalBuffer::new(80, 24);
    buf.write(b"\x1b[?9h");
    assert!(buf.wants_mouse(), "DECSET 9 should enable mouse");
}

#[test]
fn mouse_mode_button_event_tracking() {
    let mut buf = TerminalBuffer::new(80, 24);
    buf.write(b"\x1b[?1002h");
    assert!(buf.wants_mouse(), "DECSET 1002 should enable mouse");
}

#[test]
fn line_wrapping_at_boundary() {
    // Buffer with 5 cols, write 8 chars — should wrap
    let output = run(5, 24, b"abcdefgh");
    // In panex, lines don't auto-wrap in the buffer — they grow as needed.
    // to_test_string should show the full line content.
    insta::assert_snapshot!(output, @"abcdefgh");
}

// --- Special key tracking tests (Step 9) ---

#[test]
fn special_keys_default_off() {
    let buf = TerminalBuffer::new(80, 24);
    assert!(
        !buf.wants_special_keys(),
        "fresh buffer should not want special keys"
    );
}

#[test]
fn special_keys_decckm_on() {
    let mut buf = TerminalBuffer::new(80, 24);
    buf.write(b"\x1b[?1h"); // DECSET DECCKM — application cursor keys
    assert!(
        buf.wants_special_keys(),
        "DECSET DECCKM should enable special keys"
    );
}

#[test]
fn special_keys_decckm_on_then_off() {
    let mut buf = TerminalBuffer::new(80, 24);
    buf.write(b"\x1b[?1h"); // DECSET DECCKM
    assert!(buf.wants_special_keys());
    buf.write(b"\x1b[?1l"); // DECRST DECCKM
    assert!(
        !buf.wants_special_keys(),
        "DECRST DECCKM should disable special keys"
    );
}

#[test]
fn special_keys_alternate_screen() {
    let mut buf = TerminalBuffer::new(80, 24);
    buf.write(b"\x1b[?1049h"); // Enter alternate screen
    assert!(
        buf.wants_special_keys(),
        "alternate screen should enable special keys"
    );
}

#[test]
fn special_keys_mouse_mode() {
    let mut buf = TerminalBuffer::new(80, 24);
    buf.write(b"\x1b[?1000h"); // Enable mouse tracking
    assert!(
        buf.wants_special_keys(),
        "mouse mode should enable special keys"
    );
}

// --- DECTCEM cursor visibility tests (Step 10) ---

#[test]
fn special_keys_cursor_hidden() {
    let mut buf = TerminalBuffer::new(80, 24);
    buf.write(b"\x1b[?25l"); // Hide cursor (DECTCEM disable)
    assert!(
        buf.wants_special_keys(),
        "hidden cursor should enable special keys"
    );
}

#[test]
fn special_keys_cursor_hidden_then_shown() {
    let mut buf = TerminalBuffer::new(80, 24);
    buf.write(b"\x1b[?25l"); // Hide cursor
    assert!(buf.wants_special_keys());
    buf.write(b"\x1b[?25h"); // Show cursor (DECTCEM enable)
    assert!(
        !buf.wants_special_keys(),
        "showing cursor should disable special keys"
    );
}

#[test]
fn special_keys_cursor_hidden_plus_alternate_screen() {
    let mut buf = TerminalBuffer::new(80, 24);
    buf.write(b"\x1b[?25l"); // Hide cursor
    buf.write(b"\x1b[?1049h"); // Enter alternate screen
    assert!(
        buf.wants_special_keys(),
        "both signals active should enable special keys"
    );
}

#[test]
fn special_keys_cursor_shown_but_decckm_on() {
    let mut buf = TerminalBuffer::new(80, 24);
    buf.write(b"\x1b[?25l"); // Hide cursor
    buf.write(b"\x1b[?25h"); // Show cursor
    buf.write(b"\x1b[?1h"); // Enable DECCKM
    assert!(
        buf.wants_special_keys(),
        "DECCKM alone is sufficient even after cursor shown"
    );
}
