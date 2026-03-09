use anyhow::{bail, Result};
use chrono::{Local, Utc};
use rusqlite::Connection;

use crate::db::{get_instances_for_project, get_project_by_id};
use crate::utils::format_duration;

pub fn execute(conn: &Connection, project_id: i64) -> Result<()> {
    let project = match get_project_by_id(conn, project_id)? {
        Some(p) => p,
        None => bail!("Project with ID {} not found", project_id),
    };

    let instances = get_instances_for_project(conn, project_id)?;

    if instances.is_empty() {
        println!(
            "No instances found for project '{}' (ID: {})",
            project.name, project.id
        );
        return Ok(());
    }

    println!("Project: {} (ID: {})", project.name, project.id);
    println!();
    println!(
        "{:<12} {:<26} {:<14} {}",
        "INSTANCE ID", "START TIME", "DURATION", "STATUS"
    );
    println!("{}", "-".repeat(60));

    let now = Utc::now();
    for instance in &instances {
        let start_local = instance.start_time.with_timezone(&Local);
        let start_str = start_local.format("%Y-%m-%d %H:%M:%S").to_string();

        let (duration_str, status) = if instance.is_running() {
            let elapsed = (now - instance.start_time).num_seconds();
            (format_duration(elapsed), "running")
        } else {
            let duration = instance.duration().unwrap_or(0);
            (format_duration(duration), "stopped")
        };

        println!(
            "{:<12} {:<26} {:<14} {}",
            instance.id, start_str, duration_str, status
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_instance, stop_timer, upsert_project};
    use chrono::{Duration, Utc};
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
    fn test_instance_list_project_not_found() {
        let conn = setup_in_memory_db();
        let result = execute(&conn, 999);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Project with ID 999 not found"));
    }

    #[test]
    fn test_instance_list_no_instances() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "empty").unwrap();
        // Should succeed and print a "no instances" message without error
        let result = execute(&conn, project.id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_instance_list_with_stopped_instance() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "myproject").unwrap();
        let start = Utc::now() - Duration::seconds(300);
        create_instance(&conn, project.id, start).unwrap();
        stop_timer(&conn, project.id, Utc::now()).unwrap();

        let result = execute(&conn, project.id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_instance_list_with_running_instance() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "running").unwrap();
        let start = Utc::now() - Duration::seconds(60);
        create_instance(&conn, project.id, start).unwrap();

        let result = execute(&conn, project.id);
        assert!(result.is_ok());
    }
}
