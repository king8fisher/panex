# Panex

Terminal UI process manager — runs multiple commands in parallel with interactive split-pane interface.
Written in Rust (ratatui + crossterm), distributed via npm with prebuilt binaries.

## Project Layout

```
panex-rs/           # Rust crate (main codebase)
  src/
    main.rs         # Entry + tokio runtime
    config.rs       # CLI parsing (clap)
    ...
  Cargo.toml
  INTERNALS.md      # Deep-dive implementation docs

package.json        # npm wrapper (bin.js → prebuilt binary)
install.js          # postinstall downloads platform binary
bin.js              # npm bin entry point
mise.toml           # Task runner (build, dev, release)
```

## Commands

- **ci**:
  1. `cd panex-rs && cargo clippy -- -D warnings` — lint with zero warnings
  2. `cd panex-rs && cargo fmt -- --check` — format check
  3. `cd panex-rs && cargo test` — run all tests
- **build**: `cd panex-rs && cargo build --release` — full release build
- **build-debug**: `cd panex-rs && cargo build` — debug build
- **test**: `cd panex-rs && cargo test` — run all tests
- **test-file**: `cd panex-rs && cargo test <name>` — run a specific test
- **lint**: `cd panex-rs && cargo clippy -- -D warnings` — lint with zero warnings
- **format**: `cd panex-rs && cargo fmt` — auto-format
- **dev**: `mise run dev` — build + launch with demo processes

## Conventions

- Rust 2021 edition, stable toolchain
- No `unsafe` except for `libc::kill` in process group termination
- PTY-spawned processes use VTE parser for terminal emulation
- Lines stored at full width, truncated at render time
- Generation counters on processes to avoid restart race conditions
- Process name suffixes: `!` for key passthrough, `:w` for line wrap
