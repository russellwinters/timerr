use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::Connection;

use crate::db::{create_instance, get_project_by_name, has_running_timer, upsert_project};

pub fn execute(conn: &Connection, project_name: &str) -> Result<()> {
    // Validate project name
    let project_name = project_name.trim();
    if project_name.is_empty() {
        bail!("Project name cannot be empty");
    }

    if project_name.len() > 100 {
        bail!("Project name is too long (max 100 characters)");
    }

    // Check if the project already exists before upserting
    let is_existing = get_project_by_name(conn, project_name)?.is_some();

    // Get or create the project
    let project = upsert_project(conn, project_name)?;

    // Check if there's already a running timer
    if has_running_timer(conn, project.id)? {
        bail!(
            "Project '{}' already has a running timer. Stop it first with: timerr stop {}",
            project_name,
            project_name
        );
    }

    // Create a new instance
    let start_time = Utc::now();
    create_instance(conn, project.id, start_time)?;

    println!(
        "✓ Timer started for {} project '{}'",
        if is_existing { "existing" } else { "new" },
        project_name
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::stop_timer;

    fn setup_in_memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("PRAGMA foreign_keys = ON", []).unwrap();
        conn.execute(
            "CREATE TABLE projects (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                time_sum INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'active'
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE instances (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER NOT NULL,
                start_time TEXT NOT NULL,
                stop_time TEXT,
                FOREIGN KEY (project_id) REFERENCES projects(id)
            )",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_start_new_project_succeeds() {
        let conn = setup_in_memory_db();
        let result = execute(&conn, "myproject");
        assert!(result.is_ok());
    }

    #[test]
    fn test_start_existing_project_succeeds() {
        let conn = setup_in_memory_db();
        // First start creates the project
        execute(&conn, "myproject").unwrap();
        // Stop the timer so we can start again
        let project = get_project_by_name(&conn, "myproject").unwrap().unwrap();
        stop_timer(&conn, project.id, Utc::now()).unwrap();
        // Second start reuses the existing project
        let result = execute(&conn, "myproject");
        assert!(result.is_ok());
    }

    #[test]
    fn test_start_already_running_timer_fails() {
        let conn = setup_in_memory_db();
        execute(&conn, "myproject").unwrap();
        let result = execute(&conn, "myproject");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already has a running timer"));
    }

    #[test]
    fn test_start_empty_name_fails() {
        let conn = setup_in_memory_db();
        let result = execute(&conn, "");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_start_new_vs_existing_project_detection() {
        let conn = setup_in_memory_db();
        // Project does not exist yet
        let is_existing_before = get_project_by_name(&conn, "proj").unwrap().is_some();
        assert!(!is_existing_before);
        // Start creates the project
        execute(&conn, "proj").unwrap();
        // Project now exists
        let is_existing_after = get_project_by_name(&conn, "proj").unwrap().is_some();
        assert!(is_existing_after);
    }
}
