use std::collections::VecDeque;

use crate::process::buffer::Line;

/// A single search match location in the buffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    pub row: usize,
    /// Start column (inclusive)
    pub col_start: usize,
    /// End column (exclusive)
    pub col_end: usize,
}

/// Search mode state machine
#[derive(Debug, Clone, Default)]
pub enum SearchState {
    /// No active search
    #[default]
    Inactive,
    /// User is typing a search query
    Typing { query: String, saved_scroll: usize },
    /// Search is active with results
    Active {
        query: String,
        matches: Vec<SearchMatch>,
        current: usize,
        saved_scroll: usize,
    },
}

impl SearchState {
    #[allow(dead_code)]
    pub fn new_inactive() -> Self {
        Self::Inactive
    }

    pub fn new_typing(saved_scroll: usize) -> Self {
        Self::Typing {
            query: String::new(),
            saved_scroll,
        }
    }

    pub fn new_active(
        query: String,
        matches: Vec<SearchMatch>,
        current: usize,
        saved_scroll: usize,
    ) -> Self {
        Self::Active {
            query,
            matches,
            current,
            saved_scroll,
        }
    }

    #[allow(dead_code)]
    pub fn is_inactive(&self) -> bool {
        matches!(self, Self::Inactive)
    }

    pub fn is_typing(&self) -> bool {
        matches!(self, Self::Typing { .. })
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active { .. })
    }

    /// Get the current query string (for typing or active states)
    pub fn query(&self) -> &str {
        match self {
            Self::Typing { query, .. } | Self::Active { query, .. } => query,
            Self::Inactive => "",
        }
    }

    /// Get saved scroll position
    pub fn saved_scroll(&self) -> Option<usize> {
        match self {
            Self::Typing { saved_scroll, .. } | Self::Active { saved_scroll, .. } => {
                Some(*saved_scroll)
            }
            Self::Inactive => None,
        }
    }

    /// Push a character while in typing mode
    pub fn push_char(&mut self, c: char) {
        if let Self::Typing { query, .. } = self {
            query.push(c);
        }
    }

    /// Pop a character while in typing mode
    pub fn pop_char(&mut self) {
        if let Self::Typing { query, .. } = self {
            query.pop();
        }
    }

    /// Number of matches (0 if not in active state)
    pub fn match_count(&self) -> usize {
        match self {
            Self::Active { matches, .. } => matches.len(),
            _ => 0,
        }
    }

    /// Current match index (None if not active or no matches)
    pub fn current_index(&self) -> Option<usize> {
        match self {
            Self::Active {
                matches, current, ..
            } if !matches.is_empty() => Some(*current),
            _ => None,
        }
    }

    /// Get the current match (None if not active or no matches)
    pub fn current_match(&self) -> Option<&SearchMatch> {
        match self {
            Self::Active {
                matches, current, ..
            } if !matches.is_empty() => matches.get(*current),
            _ => None,
        }
    }

    /// Move to the next match (wraps around)
    pub fn next_match(&mut self) {
        if let Self::Active {
            matches, current, ..
        } = self
        {
            if !matches.is_empty() {
                *current = (*current + 1) % matches.len();
            }
        }
    }

    /// Move to the previous match (wraps around)
    pub fn prev_match(&mut self) {
        if let Self::Active {
            matches, current, ..
        } = self
        {
            if !matches.is_empty() {
                *current = (*current + matches.len() - 1) % matches.len();
            }
        }
    }

    /// Cancel the search. Returns the saved scroll position to restore.
    pub fn cancel(&mut self) -> Option<usize> {
        let scroll = self.saved_scroll();
        *self = Self::Inactive;
        scroll
    }

    /// Confirm the search. Returns the current match (for scroll position).
    /// Transitions to Inactive.
    pub fn confirm(&mut self) -> Option<SearchMatch> {
        let result = self.current_match().cloned();
        *self = Self::Inactive;
        result
    }

    /// Check if a cell at (row, col) is within any match
    pub fn contains_any_match(&self, row: usize, col: usize) -> bool {
        match self {
            Self::Active { matches, .. } => matches
                .iter()
                .any(|m| m.row == row && col >= m.col_start && col < m.col_end),
            _ => false,
        }
    }

    /// Check if a cell at (row, col) is within the current (highlighted) match
    pub fn is_current_match(&self, row: usize, col: usize) -> bool {
        match self {
            Self::Active {
                matches, current, ..
            } if !matches.is_empty() => {
                let m = &matches[*current];
                m.row == row && col >= m.col_start && col < m.col_end
            }
            _ => false,
        }
    }
}

/// Find all occurrences of `query` in the buffer lines (case-insensitive).
pub fn find_matches(query: &str, buffer: &VecDeque<Line>) -> Vec<SearchMatch> {
    if query.is_empty() {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for (row, line) in buffer.iter().enumerate() {
        // Build the line text from cells
        let text: String = line.cells.iter().map(|c| c.c).collect();
        let text_lower = text.to_lowercase();

        let mut start = 0;
        while let Some(pos) = text_lower[start..].find(&query_lower) {
            let col_start = start + pos;
            let col_end = col_start + query_lower.len();
            results.push(SearchMatch {
                row,
                col_start,
                col_end,
            });
            start = col_start + 1; // Allow overlapping matches? No, advance past start
        }
    }

    results
}

/// Find the match index closest to a given scroll position
pub fn nearest_match_index(matches: &[SearchMatch], scroll_offset: usize) -> usize {
    if matches.is_empty() {
        return 0;
    }
    // Find the first match at or after the scroll offset
    for (i, m) in matches.iter().enumerate() {
        if m.row >= scroll_offset {
            return i;
        }
    }
    // All matches are above scroll offset — return the last one
    matches.len() - 1
}
