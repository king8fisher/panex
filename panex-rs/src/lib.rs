/// Library crate for integration testing.
/// Exposes buffer and config modules; the full app is in main.rs.
pub mod process {
    pub mod buffer;
}

pub mod config;

/// Search types and logic for testing.
/// Uses `include!` to share the source with the binary crate's `ui::search`.
pub mod search {
    include!("ui/search.rs");
}
