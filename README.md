# panex

A terminal UI for running multiple processes in parallel. Like Turborepo's TUI, without the monorepo.

```
┌──────────────┬──────────────────────────────────────────────┐
│              │ api                                          │
│ ▶ api    ●   │                                              │
│   web    ●   │ Server listening on http://localhost:3001    │
│   mobile ●   │ {"level":30,"msg":"request completed"}       │
│              │                                              │
├──────────────┴──────────────────────────────────────────────┤
│ [↑↓] select  [enter] focus  [r] restart  [q] quit           │
└─────────────────────────────────────────────────────────────┘
```

## Features

- **Split-pane TUI** - See all your processes at once
- **Full PTY support** - QR codes, colors, interactive prompts work
- **Zero config** - Just pass commands as arguments
- **Cross-platform** - macOS, Linux, Windows

## Requirements

**Bun runtime is required.** Panex uses Bun's built-in PTY support for interactive processes.

## Installation

```bash
# Run directly with bunx
bunx panex "bun run api" "bun run web"

# Or install globally
bun add -g panex
```

## Usage

### Quick Start

```bash
# Run multiple commands
panex "npm run api" "npm run web" "npm run mobile"

# With custom names
panex -n api,web,mobile "npm run api" "npm run web" "npm run mobile"
```

### Keyboard Shortcuts

| Key            | Action                           |
| -------------- | -------------------------------- |
| `↑/↓` or `j/k` | Navigate process list            |
| `Enter`        | Focus process (interactive mode) |
| `Esc`          | Exit focus mode                  |
| `r`            | Restart selected process         |
| `x`            | Kill selected process            |
| `A`            | Restart all processes            |
| `q`            | Quit panex                       |
| `?`            | Show help                        |
| `g/G`          | Scroll to top/bottom             |
| `PgUp/PgDn`    | Scroll output                    |

## Why panex?

| Feature                | panex | concurrently | mprocs | turbo |
| ---------------------- | ----- | ------------ | ------ | ----- |
| Split-pane TUI         | ✅     | ❌            | ✅      | ✅     |
| PTY support (QR codes) | ✅     | ❌            | ✅      | ✅     |
| Zero config            | ✅     | ✅            | ❌      | ❌     |
| npm install            | ✅     | ✅            | ❌      | ✅     |
| No monorepo required   | ✅     | ✅            | ✅      | ❌     |

## Development

```bash
# Clone
git clone https://github.com/king8fisher/panex
cd panex

# Install dependencies
bun install

# Run in dev mode
bun run dev "echo hello" "sleep 2 && echo world"

# Type check
bun run typecheck

# Build for npm
bun run build

# Run tests
bun test

# Test built CLI
node dist/cli.js "echo test"
```

## Tech Stack

- TypeScript + Bun
- blessed (TUI framework)
- Bun.spawn with terminal (PTY support)
- commander (CLI parsing)
- tsup (build tool)

## License

MIT
