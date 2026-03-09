use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::Connection;

use crate::db::{create_completed_instance, get_project_by_name};
use crate::utils::{format_duration, parse_duration};

pub fn execute(conn: &Connection, project_name: &str, time: &str) -> Result<()> {
    let project_name = project_name.trim();
    if project_name.is_empty() {
        bail!("Project name cannot be empty");
    }

    let duration_secs = parse_duration(time)?;

    let project = match get_project_by_name(conn, project_name)? {
        Some(p) => p,
        None => bail!(
            "No active project found with name '{}'. Start a project first with: timerr start {}",
            project_name,
            project_name
        ),
    };

    let stop_time = Utc::now();
    let start_time = stop_time - chrono::Duration::seconds(duration_secs);

    create_completed_instance(conn, project.id, start_time, stop_time)?;

    println!(
        "✓ Added {} instance to project '{}'",
        format_duration(duration_secs),
        project_name
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{get_instances_for_project, upsert_project};
    use rusqlite::Connection;

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
    fn test_instance_add_creates_stopped_instance() {
        let conn = setup_in_memory_db();
        upsert_project(&conn, "myproject").unwrap();

        let result = execute(&conn, "myproject", "1h 30m");
        assert!(result.is_ok());

        let project = get_project_by_name(&conn, "myproject").unwrap().unwrap();
        let instances = get_instances_for_project(&conn, project.id).unwrap();
        assert_eq!(instances.len(), 1);
        assert!(instances[0].stop_time.is_some(), "Instance should be stopped");
        let duration = instances[0].duration().unwrap();
        assert!((duration - 5400).abs() <= 2, "Expected ~5400s, got {duration}");
    }

    #[test]
    fn test_instance_add_updates_time_sum() {
        let conn = setup_in_memory_db();
        upsert_project(&conn, "proj").unwrap();

        execute(&conn, "proj", "30m").unwrap();

        let time_sum: i64 = conn
            .query_row(
                "SELECT time_sum FROM projects WHERE name = ?1",
                rusqlite::params!["proj"],
                |row| row.get(0),
            )
            .unwrap();
        assert!((time_sum - 1800).abs() <= 2, "Expected ~1800s, got {time_sum}");
    }

    #[test]
    fn test_instance_add_project_not_found_fails() {
        let conn = setup_in_memory_db();
        let result = execute(&conn, "nonexistent", "1h");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No active project found"));
    }

    #[test]
    fn test_instance_add_empty_project_name_fails() {
        let conn = setup_in_memory_db();
        let result = execute(&conn, "", "1h");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot be empty"));
    }

    #[test]
    fn test_instance_add_invalid_duration_fails() {
        let conn = setup_in_memory_db();
        upsert_project(&conn, "proj").unwrap();
        let result = execute(&conn, "proj", "invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_instance_add_zero_duration_fails() {
        let conn = setup_in_memory_db();
        upsert_project(&conn, "proj").unwrap();
        let result = execute(&conn, "proj", "0s");
        assert!(result.is_err());
    }
}
