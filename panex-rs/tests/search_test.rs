use panex::process::buffer::TerminalBuffer;
use panex::search::{find_matches, SearchMatch, SearchState};

/// Helper: create a buffer with given text lines
fn buffer_with(lines: &[&str]) -> TerminalBuffer {
    let mut buf = TerminalBuffer::new(80, 24);
    let text = lines.join("\n");
    buf.write(text.as_bytes());
    buf
}

// --- find_matches tests ---

#[test]
fn search_finds_two_matches() {
    let buf = buffer_with(&["hello world", "foo bar", "hello again"]);
    let matches = find_matches("hello", buf.get_all_lines());
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].row, 0);
    assert_eq!(matches[0].col_start, 0);
    assert_eq!(matches[0].col_end, 5); // exclusive end
    assert_eq!(matches[1].row, 2);
    assert_eq!(matches[1].col_start, 0);
    assert_eq!(matches[1].col_end, 5);
}

#[test]
fn search_finds_multiple_matches_on_same_line() {
    let buf = buffer_with(&["abcabc"]);
    let matches = find_matches("abc", buf.get_all_lines());
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].col_start, 0);
    assert_eq!(matches[1].col_start, 3);
}

#[test]
fn search_case_insensitive() {
    let buf = buffer_with(&["Hello HELLO hello"]);
    let matches = find_matches("hello", buf.get_all_lines());
    assert_eq!(matches.len(), 3);
}

#[test]
fn search_no_matches_returns_empty() {
    let buf = buffer_with(&["hello world", "foo bar"]);
    let matches = find_matches("xyz", buf.get_all_lines());
    assert!(matches.is_empty());
}

#[test]
fn search_empty_query_returns_no_matches() {
    let buf = buffer_with(&["hello world"]);
    let matches = find_matches("", buf.get_all_lines());
    assert!(matches.is_empty());
}

// --- SearchState navigation tests ---

#[test]
fn next_match_cycles_forward() {
    let matches = vec![
        SearchMatch {
            row: 0,
            col_start: 0,
            col_end: 3,
        },
        SearchMatch {
            row: 1,
            col_start: 0,
            col_end: 3,
        },
        SearchMatch {
            row: 2,
            col_start: 0,
            col_end: 3,
        },
    ];
    let mut state = SearchState::new_active("abc".to_string(), matches, 0, 0);
    assert_eq!(state.current_index(), Some(0));
    state.next_match();
    assert_eq!(state.current_index(), Some(1));
    state.next_match();
    assert_eq!(state.current_index(), Some(2));
    // Wraps around
    state.next_match();
    assert_eq!(state.current_index(), Some(0));
}

#[test]
fn prev_match_cycles_backward() {
    let matches = vec![
        SearchMatch {
            row: 0,
            col_start: 0,
            col_end: 3,
        },
        SearchMatch {
            row: 1,
            col_start: 0,
            col_end: 3,
        },
        SearchMatch {
            row: 2,
            col_start: 0,
            col_end: 3,
        },
    ];
    let mut state = SearchState::new_active("abc".to_string(), matches, 0, 0);
    assert_eq!(state.current_index(), Some(0));
    // Wraps to last
    state.prev_match();
    assert_eq!(state.current_index(), Some(2));
    state.prev_match();
    assert_eq!(state.current_index(), Some(1));
}

#[test]
fn cancel_search_restores_scroll_position() {
    let mut state = SearchState::new_typing(42);
    assert_eq!(state.saved_scroll(), Some(42));
    let restored = state.cancel();
    assert_eq!(restored, Some(42));
    assert!(state.is_inactive());
}

#[test]
fn cancel_active_search_restores_scroll_position() {
    let matches = vec![SearchMatch {
        row: 10,
        col_start: 0,
        col_end: 3,
    }];
    let mut state = SearchState::new_active("abc".to_string(), matches, 0, 42);
    let restored = state.cancel();
    assert_eq!(restored, Some(42));
    assert!(state.is_inactive());
}

#[test]
fn confirm_search_stays_at_current_match() {
    let matches = vec![
        SearchMatch {
            row: 0,
            col_start: 0,
            col_end: 3,
        },
        SearchMatch {
            row: 5,
            col_start: 0,
            col_end: 3,
        },
    ];
    let mut state = SearchState::new_active("abc".to_string(), matches, 0, 0);
    state.next_match();
    assert_eq!(state.current_index(), Some(1));
    // Confirm keeps current_match info but deactivates typing
    let current = state.confirm();
    assert_eq!(current.unwrap().row, 5);
    assert!(state.is_inactive());
}

#[test]
fn no_matches_state() {
    let state = SearchState::new_active("xyz".to_string(), vec![], 0, 0);
    assert_eq!(state.match_count(), 0);
    assert!(state.current_match().is_none());
}

#[test]
fn current_match_returns_correct_match() {
    let matches = vec![
        SearchMatch {
            row: 0,
            col_start: 0,
            col_end: 5,
        },
        SearchMatch {
            row: 2,
            col_start: 0,
            col_end: 5,
        },
    ];
    let state = SearchState::new_active("hello".to_string(), matches, 1, 0);
    let m = state.current_match().unwrap();
    assert_eq!(m.row, 2);
}

#[test]
fn typing_state_tracks_query() {
    let mut state = SearchState::new_typing(0);
    state.push_char('h');
    state.push_char('i');
    assert_eq!(state.query(), "hi");
    state.pop_char();
    assert_eq!(state.query(), "h");
}

#[test]
fn contains_highlight_checks_all_matches() {
    let matches = vec![
        SearchMatch {
            row: 0,
            col_start: 0,
            col_end: 3,
        },
        SearchMatch {
            row: 2,
            col_start: 5,
            col_end: 8,
        },
    ];
    let state = SearchState::new_active("abc".to_string(), matches, 0, 0);
    // First match: row 0, cols 0..3
    assert!(state.contains_any_match(0, 0));
    assert!(state.contains_any_match(0, 2));
    assert!(!state.contains_any_match(0, 3)); // col_end is exclusive
                                              // Second match: row 2, cols 5..8
    assert!(state.contains_any_match(2, 5));
    assert!(state.contains_any_match(2, 7));
    assert!(!state.contains_any_match(2, 8));
    // No match
    assert!(!state.contains_any_match(1, 0));
}

#[test]
fn is_current_match_distinguishes_current() {
    let matches = vec![
        SearchMatch {
            row: 0,
            col_start: 0,
            col_end: 3,
        },
        SearchMatch {
            row: 2,
            col_start: 5,
            col_end: 8,
        },
    ];
    let state = SearchState::new_active("abc".to_string(), matches, 1, 0);
    // Current match is index 1 (row 2)
    assert!(!state.is_current_match(0, 0));
    assert!(state.is_current_match(2, 5));
    assert!(state.is_current_match(2, 7));
}
