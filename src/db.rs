use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::PathBuf;

use crate::models::{Instance, Project};

/// Get the database file path for the current platform
pub fn get_db_path() -> Result<PathBuf> {
    let data_dir = dirs::data_dir().context("Failed to get data directory")?;

    let timerr_dir = data_dir.join("timerr");
    std::fs::create_dir_all(&timerr_dir).context("Failed to create timerr data directory")?;

    Ok(timerr_dir.join("timerr.db"))
}

/// Initialize the database and create tables if they don't exist
pub fn init_database() -> Result<Connection> {
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path).context("Failed to open database")?;

    // Enable foreign key support
    conn.execute("PRAGMA foreign_keys = ON", [])
        .context("Failed to enable foreign keys")?;

    // Create projects table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            time_sum INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )
    .context("Failed to create projects table")?;

    // Create instances table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS instances (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id INTEGER NOT NULL,
            start_time TEXT NOT NULL,
            stop_time TEXT,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        )",
        [],
    )
    .context("Failed to create instances table")?;

    // Create indexes for better query performance
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_instances_project_id ON instances(project_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_instances_stop_time ON instances(stop_time)",
        [],
    )?;

    Ok(conn)
}

/// Get or create a project by name
pub fn upsert_project(conn: &Connection, name: &str) -> Result<Project> {
    // Try to insert, if it exists, fetch it
    conn.execute(
        "INSERT OR IGNORE INTO projects (name, time_sum) VALUES (?1, 0)",
        params![name],
    )?;

    let mut stmt = conn.prepare("SELECT id, name, time_sum FROM projects WHERE name = ?1")?;
    let project = stmt.query_row(params![name], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            time_sum: row.get(2)?,
        })
    })?;

    Ok(project)
}

/// Check if a project has a running timer
pub fn has_running_timer(conn: &Connection, project_id: i64) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM instances WHERE project_id = ?1 AND stop_time IS NULL",
        params![project_id],
        |row| row.get(0),
    )?;

    Ok(count > 0)
}

/// Create a new time tracking instance
pub fn create_instance(
    conn: &Connection,
    project_id: i64,
    start_time: DateTime<Utc>,
) -> Result<Instance> {
    conn.execute(
        "INSERT INTO instances (project_id, start_time) VALUES (?1, ?2)",
        params![project_id, start_time.to_rfc3339()],
    )?;

    let id = conn.last_insert_rowid();

    Ok(Instance {
        id,
        project_id,
        start_time,
        stop_time: None,
    })
}

/// Stop the running timer for a project
pub fn stop_timer(
    conn: &Connection,
    project_id: i64,
    stop_time: DateTime<Utc>,
) -> Result<Option<i64>> {
    // Find the running instance
    let mut stmt = conn.prepare(
        "SELECT id, start_time FROM instances WHERE project_id = ?1 AND stop_time IS NULL",
    )?;

    let result: Result<(i64, String), _> =
        stmt.query_row(params![project_id], |row| Ok((row.get(0)?, row.get(1)?)));

    match result {
        Ok((instance_id, start_time_str)) => {
            let start_time = DateTime::parse_from_rfc3339(&start_time_str)
                .context("Failed to parse start time")?
                .with_timezone(&Utc);

            let duration = (stop_time - start_time).num_seconds();

            // Update the instance with stop time
            conn.execute(
                "UPDATE instances SET stop_time = ?1 WHERE id = ?2",
                params![stop_time.to_rfc3339(), instance_id],
            )?;

            // Update project's total time
            conn.execute(
                "UPDATE projects SET time_sum = time_sum + ?1 WHERE id = ?2",
                params![duration, project_id],
            )?;

            Ok(Some(duration))
        }
        Err(_) => Ok(None),
    }
}

/// Get all projects
pub fn get_all_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare("SELECT id, name, time_sum FROM projects ORDER BY name")?;

    let projects = stmt
        .query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                time_sum: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(projects)
}

/// Get the start time of the active (running) instance for a project, if any
pub fn get_active_instance_start_time(
    conn: &Connection,
    project_id: i64,
) -> Result<Option<DateTime<Utc>>> {
    let mut stmt = conn.prepare(
        "SELECT start_time FROM instances WHERE project_id = ?1 AND stop_time IS NULL",
    )?;

    let result: Result<String, _> = stmt.query_row(params![project_id], |row| row.get(0));

    match result {
        Ok(start_time_str) => {
            let start_time = DateTime::parse_from_rfc3339(&start_time_str)
                .context("Failed to parse start time")?
                .with_timezone(&Utc);
            Ok(Some(start_time))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Get a project by name
pub fn get_project_by_name(conn: &Connection, name: &str) -> Result<Option<Project>> {
    let mut stmt = conn.prepare("SELECT id, name, time_sum FROM projects WHERE name = ?1")?;

    let result = stmt.query_row(params![name], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            time_sum: row.get(2)?,
        })
    });

    match result {
        Ok(project) => Ok(Some(project)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn setup_in_memory_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("PRAGMA foreign_keys = ON", []).unwrap();
        conn.execute(
            "CREATE TABLE projects (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                time_sum INTEGER NOT NULL DEFAULT 0
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
    fn test_get_active_instance_start_time_no_active() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "test").unwrap();

        let result = get_active_instance_start_time(&conn, project.id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_active_instance_start_time_with_active() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "test").unwrap();
        let start = Utc::now() - Duration::seconds(120);
        create_instance(&conn, project.id, start).unwrap();

        let result = get_active_instance_start_time(&conn, project.id).unwrap();
        assert!(result.is_some());
        let returned_start = result.unwrap();
        // Allow 1 second tolerance for rfc3339 round-trip
        assert!((returned_start - start).num_seconds().abs() <= 1);
    }

    #[test]
    fn test_get_active_instance_start_time_stopped_instance() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "test").unwrap();
        let start = Utc::now() - Duration::seconds(300);
        create_instance(&conn, project.id, start).unwrap();
        stop_timer(&conn, project.id, Utc::now()).unwrap();

        let result = get_active_instance_start_time(&conn, project.id).unwrap();
        assert!(result.is_none());
    }
}
