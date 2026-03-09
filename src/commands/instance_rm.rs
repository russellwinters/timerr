use anyhow::{bail, Result};
use rusqlite::Connection;

use crate::db::{delete_instance, get_instance_by_id};

pub fn execute(conn: &Connection, instance_id: i64) -> Result<()> {
    let instance = match get_instance_by_id(conn, instance_id)? {
        Some(i) => i,
        None => bail!("Instance with ID {} not found", instance_id),
    };

    delete_instance(conn, &instance)?;

    println!("✓ Instance {} has been deleted", instance_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_instance, get_instance_by_id, stop_timer, upsert_project};
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
    fn test_instance_rm_not_found() {
        let conn = setup_in_memory_db();
        let result = execute(&conn, 999);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Instance with ID 999 not found"));
    }

    #[test]
    fn test_instance_rm_running_instance() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "proj").unwrap();
        let start = Utc::now() - Duration::seconds(60);
        let instance = create_instance(&conn, project.id, start).unwrap();

        let result = execute(&conn, instance.id);
        assert!(result.is_ok());

        // Instance should no longer exist
        let found = get_instance_by_id(&conn, instance.id).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_instance_rm_stopped_instance_adjusts_time_sum() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "proj").unwrap();
        let start = Utc::now() - Duration::seconds(300);
        let instance = create_instance(&conn, project.id, start).unwrap();
        stop_timer(&conn, project.id, Utc::now()).unwrap();

        // time_sum should now be ~300
        let time_sum_before: i64 = conn
            .query_row(
                "SELECT time_sum FROM projects WHERE id = ?1",
                rusqlite::params![project.id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(time_sum_before > 0);

        let result = execute(&conn, instance.id);
        assert!(result.is_ok());

        // time_sum should now be 0 (or close to 0)
        let time_sum_after: i64 = conn
            .query_row(
                "SELECT time_sum FROM projects WHERE id = ?1",
                rusqlite::params![project.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(time_sum_after, 0);

        // Instance should no longer exist
        let found = get_instance_by_id(&conn, instance.id).unwrap();
        assert!(found.is_none());
    }
}
