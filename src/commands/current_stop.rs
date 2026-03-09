use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;

use crate::db::{get_active_running_projects, stop_timer};
use crate::utils::format_duration;

pub fn execute(conn: &Connection) -> Result<()> {
    let projects = get_active_running_projects(conn)?;

    if projects.is_empty() {
        println!("No projects are currently being tracked.");
        return Ok(());
    }

    let stop_time = Utc::now();
    for project in &projects {
        match stop_timer(conn, project.id, stop_time)? {
            Some(duration) => {
                println!(
                    "✓ Timer stopped for project '{}'. Duration: {}",
                    project.name,
                    format_duration(duration)
                );
            }
            None => {
                // Defensive fallback: get_active_running_projects guarantees an active instance,
                // but guard against any unexpected race or data inconsistency.
                println!("No current timer for project '{}'", project.name);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_instance, get_active_running_projects, upsert_project};

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
    fn test_current_stop_no_running_timers() {
        let conn = setup_in_memory_db();
        let result = execute(&conn);
        assert!(result.is_ok());
    }

    #[test]
    fn test_current_stop_stops_single_running_timer() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "myproject").unwrap();
        create_instance(&conn, project.id, Utc::now()).unwrap();

        let result = execute(&conn);
        assert!(result.is_ok());

        // No more running timers after stop
        let running = get_active_running_projects(&conn).unwrap();
        assert!(running.is_empty());
    }

    #[test]
    fn test_current_stop_stops_multiple_running_timers() {
        let conn = setup_in_memory_db();

        let project_a = upsert_project(&conn, "alpha").unwrap();
        create_instance(&conn, project_a.id, Utc::now()).unwrap();

        let project_b = upsert_project(&conn, "beta").unwrap();
        create_instance(&conn, project_b.id, Utc::now()).unwrap();

        let result = execute(&conn);
        assert!(result.is_ok());

        // All running timers should be stopped
        let running = get_active_running_projects(&conn).unwrap();
        assert!(running.is_empty());
    }

    #[test]
    fn test_current_stop_updates_project_time_sum() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "myproject").unwrap();
        let start_time = Utc::now() - chrono::Duration::seconds(60);
        create_instance(&conn, project.id, start_time).unwrap();

        execute(&conn).unwrap();

        // time_sum should have been updated
        let updated: i64 = conn
            .query_row(
                "SELECT time_sum FROM projects WHERE id = ?1",
                rusqlite::params![project.id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(updated >= 60);
    }
}
