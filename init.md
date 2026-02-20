# Timerr - Time Tracking CLI Application

## Overview

Timerr is a simple command-line interface (CLI) application for tracking time spent on different projects. It uses a local SQLite database to store project information and time tracking instances.

## Purpose

The application allows users to:
- Start timers for different projects
- Stop running timers
- List all projects with their total tracked time

This provides a lightweight, offline-first solution for time tracking without the overhead of complex time tracking systems.

## Architecture

### Technology Stack
- **Language**: Rust
- **Database**: SQLite
- **CLI Framework**: clap (Rust command-line argument parser)
- **Database Access**: rusqlite

### Database Schema

The application uses two main tables:

#### `projects` Table
| Column     | Type    | Description                           |
|------------|---------|---------------------------------------|
| id         | INTEGER | Primary key, auto-incrementing        |
| name       | TEXT    | Project name (unique)                 |
| time_sum   | INTEGER | Total accumulated time in seconds     |

#### `instances` Table
| Column      | Type     | Description                                |
|-------------|----------|--------------------------------------------|
| id          | INTEGER  | Primary key, auto-incrementing             |
| project_id  | INTEGER  | Foreign key referencing projects(id)       |
| start_time  | TEXT     | ISO 8601 timestamp when timer started      |
| stop_time   | TEXT     | ISO 8601 timestamp when timer stopped (NULL if running) |

## Commands

### `timerr start <project_name>`

Starts a timer for the specified project.

**Behavior:**
1. Creates the project if it doesn't exist (upsert operation)
2. Creates a new instance record with the current timestamp as `start_time`
3. Returns an error if a timer for this project is already running (has an instance without `stop_time`)

**Example:**
```bash
timerr start my_project
# Output: Timer started for project 'my_project'
```

**Error Cases:**
```bash
timerr start my_project
# When already running:
# Output: Error: Project 'my_project' already has a running timer. Stop it first.
```

### `timerr stop <project_name>`

Stops the currently running timer for the specified project.

**Behavior:**
1. Finds the active instance (one without `stop_time`) for the project
2. Sets the `stop_time` to the current timestamp
3. Updates the project's `time_sum` with the duration
4. Returns success message or indicates no active timer exists

**Example:**
```bash
timerr stop my_project
# Output: Timer stopped for project 'my_project'. Duration: 1h 23m 45s
```

**No Active Timer:**
```bash
timerr stop my_project
# Output: No current timer for project 'my_project'
```

### `timerr list`

Lists all projects with their total tracked time.

**Behavior:**
1. Queries all projects from the database
2. Displays each project with its total accumulated time
3. Shows time in a human-readable format (hours, minutes, seconds)

**Example:**
```bash
timerr list
# Output:
# Projects:
# 
#   my_project - 5h 30m 15s
#   another_project - 2h 15m
#   quick_task - 45s
# 
# Total time tracked: 8h 0m 45s
```

## General Plan

### Project Structure

```
timerr/
├── src/
│   ├── main.rs           # CLI entry point and command routing
│   ├── db.rs             # Database initialization and operations
│   ├── models.rs         # Project and Instance data structures
│   ├── commands/
│   │   ├── mod.rs        # Command module exports
│   │   ├── start.rs      # Start command implementation
│   │   ├── stop.rs       # Stop command implementation
│   │   └── list.rs       # List command implementation
│   └── utils.rs          # Helper functions (time formatting, etc.)
├── Cargo.toml            # Rust project dependencies
├── init.md               # This file
├── project_plan.md       # Detailed project plan and tickets
└── README.md             # User-facing documentation
```

### Development Phases

1. **Phase 1: Foundation** - Set up project structure, database schema, and basic CLI framework
2. **Phase 2: Core Commands** - Implement start, stop, and list commands
3. **Phase 3: Error Handling** - Add comprehensive error handling and validation
4. **Phase 4: User Experience** - Improve output formatting and add helpful messages
5. **Phase 5: Testing** - Add unit and integration tests
6. **Phase 6: Documentation** - Complete README and usage examples

## Design Decisions

### Why SQLite?
- Lightweight and requires no separate server
- Perfect for local, single-user applications
- Well-supported in Rust ecosystem
- ACID compliant for data integrity

### Why Rust?
- Fast, compiled language for responsive CLI experience
- Strong type system prevents many runtime errors
- Excellent cross-platform support
- Growing ecosystem for CLI applications

### Database Location
The SQLite database file will be stored in:
- Linux/macOS: `~/.local/share/timerr/timerr.db`
- Windows: `%APPDATA%\timerr\timerr.db`

This follows platform conventions for application data storage.

## Future Enhancements

Potential features for future versions:
- Edit or delete projects
- Generate reports (daily, weekly, monthly)
- Export data to CSV/JSON
- Tag support for categorizing projects
- Multiple timer support (track multiple projects simultaneously)
- Integration with external time tracking services
- Terminal UI (TUI) for better interactivity
