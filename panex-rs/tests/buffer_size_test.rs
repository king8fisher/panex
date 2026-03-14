use panex::process::buffer::TerminalBuffer;

/// Helper: create a buffer with a custom max_scrollback, write data, return test string
fn run_with_limit(cols: usize, rows: usize, max_scrollback: usize, input: &[u8]) -> String {
    let mut buf = TerminalBuffer::with_max_scrollback(cols, rows, max_scrollback);
    buf.write(input);
    buf.to_test_string()
}

/// Helper: create a buffer with default max_scrollback, write data, return line count
fn line_count_default(cols: usize, rows: usize, input: &[u8]) -> usize {
    let mut buf = TerminalBuffer::new(cols, rows);
    buf.write(input);
    buf.content_line_count()
}

#[test]
fn buffer_truncates_to_configured_limit() {
    // Write 200 lines into a buffer limited to 100
    let input: Vec<u8> = (1..=200)
        .map(|i| format!("line {i}\n"))
        .collect::<String>()
        .into_bytes();
    let output = run_with_limit(80, 24, 100, &input);
    let lines: Vec<&str> = output.lines().collect();
    // Should have at most 100 lines; oldest lines discarded
    assert!(
        lines.len() <= 100,
        "expected <= 100 lines, got {}",
        lines.len()
    );
    // The last line should be "line 200"
    assert_eq!(lines.last().unwrap(), &"line 200");
    // "line 1" should have been evicted
    assert!(
        !output.contains("line 1\n"),
        "oldest line should have been discarded"
    );
}

#[test]
fn default_buffer_uses_10000_line_limit() {
    // Write 10_050 lines, default limit is 10_000
    let input: Vec<u8> = (1..=10_050)
        .map(|i| format!("{i}\n"))
        .collect::<String>()
        .into_bytes();
    let count = line_count_default(80, 24, &input);
    assert!(
        count <= 10_000,
        "expected <= 10000 lines, got {count}"
    );
}

#[test]
fn oldest_lines_discarded_fifo() {
    // Buffer of 5 lines: write 8 lines, only last 5 should remain
    let input: Vec<u8> = (1..=8)
        .map(|i| format!("L{i}\n"))
        .collect::<String>()
        .into_bytes();
    let output = run_with_limit(80, 24, 5, &input);
    let lines: Vec<&str> = output.lines().collect();
    assert!(lines.len() <= 5, "expected <= 5 lines, got {}", lines.len());
    // L1, L2, L3 should be gone
    assert!(!output.contains("L1"), "L1 should have been evicted");
    assert!(!output.contains("L2"), "L2 should have been evicted");
    assert!(!output.contains("L3"), "L3 should have been evicted");
    // L8 should be present
    assert!(output.contains("L8"), "L8 should be present");
}

#[test]
fn buffer_size_of_one_keeps_only_latest() {
    // With max_scrollback=1, only the last line should survive
    let input = b"first\nsecond\nthird\n";
    let output = run_with_limit(80, 24, 1, input);
    let lines: Vec<&str> = output.lines().collect();
    assert!(lines.len() <= 1, "expected <= 1 line, got {}", lines.len());
    assert!(
        !output.contains("first"),
        "first should have been evicted"
    );
    assert!(
        !output.contains("second"),
        "second should have been evicted"
    );
}
