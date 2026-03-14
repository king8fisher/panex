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

#[test]
fn line_wrapping_at_boundary() {
    // Buffer with 5 cols, write 8 chars — should wrap
    let output = run(5, 24, b"abcdefgh");
    // In panex, lines don't auto-wrap in the buffer — they grow as needed.
    // to_test_string should show the full line content.
    insta::assert_snapshot!(output, @"abcdefgh");
}
