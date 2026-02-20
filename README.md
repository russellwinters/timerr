# Timerr ⏱️

A simple, lightweight command-line time tracking application written in Rust.

## Overview

Timerr helps you track time spent on different projects using a straightforward CLI interface. All data is stored locally in a SQLite database, making it fast, private, and offline-first.

## Features

- ⏰ Start and stop timers for different projects
- 📊 View total time tracked per project
- 💾 Local SQLite database - no cloud, no tracking, no accounts
- 🚀 Fast and lightweight Rust implementation
- 🔒 Privacy-first - all data stays on your machine

## Installation

### Prerequisites

- Rust 1.70 or higher (for building from source)

### Building from Source

```bash
git clone https://github.com/russellwinters/career.git
cd career/projects/timerr
cargo build --release
```

The compiled binary will be in `target/release/timerr`.

### Installing Globally

```bash
cargo install --path .
```

This will install `timerr` to your Cargo bin directory (usually `~/.cargo/bin`), which should be in your PATH.

## Usage

### Start a Timer

Start tracking time for a project:

```bash
timerr start my_project
```

If the project doesn't exist, it will be created automatically. If a timer is already running for this project, you'll see an error message.

### Stop a Timer

Stop the currently running timer for a project:

```bash
timerr stop my_project
```

This will record the stop time and add the duration to the project's total time.

### List All Projects

View all projects and their total tracked time:

```bash
timerr list
```

Example output:
```
Projects:

  my_project - 5h 30m 15s
  another_project - 2h 15m
  quick_task - 45s

Total time tracked: 8h 0m 45s
```

## How It Works

Timerr uses a local SQLite database to store your time tracking data:

- **Projects Table**: Stores project names and total accumulated time
- **Instances Table**: Records individual start/stop events for each project

The database is stored in a platform-appropriate location:
- **Linux/macOS**: `~/.local/share/timerr/timerr.db`
- **Windows**: `%APPDATA%\timerr\timerr.db`

## Command Reference

| Command | Description | Example |
|---------|-------------|---------|
| `timerr start <project_name>` | Start a timer for a project | `timerr start website-redesign` |
| `timerr stop <project_name>` | Stop the running timer for a project | `timerr stop website-redesign` |
| `timerr list` | List all projects with total time | `timerr list` |
| `timerr --help` | Show help information | `timerr --help` |
| `timerr --version` | Show version information | `timerr --version` |

## Development

### Project Structure

```
timerr/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── db.rs             # Database operations
│   ├── models.rs         # Data structures
│   ├── commands/         # Command implementations
│   │   ├── start.rs
│   │   ├── stop.rs
│   │   └── list.rs
│   └── utils.rs          # Helper functions
├── Cargo.toml
├── README.md
├── init.md               # Technical documentation
└── project_plan.md       # Development roadmap
```

### Running Tests

```bash
cargo test
```

### Code Quality

```bash
# Run linter
cargo clippy

# Format code
cargo fmt
```

## Roadmap

See [project_plan.md](project_plan.md) for detailed development plans.

Future enhancements may include:
- Edit/delete entries
- Export to CSV/JSON
- Reports by date range
- Project tags/categories
- Terminal UI (TUI) mode

## Contributing

This is a personal project, but suggestions and bug reports are welcome! Please open an issue if you find any problems or have ideas for improvements.

## License

See the LICENSE file in the repository root.

## Author

Russell Winters - [GitHub](https://github.com/russellwinters)

## Acknowledgments

Built with:
- [Rust](https://www.rust-lang.org/) - Systems programming language
- [clap](https://github.com/clap-rs/clap) - Command line argument parsing
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite bindings
- [chrono](https://github.com/chronotope/chrono) - Date and time handling
