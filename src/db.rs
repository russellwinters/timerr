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
            time_sum INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'active'
        )",
        [],
    )
    .context("Failed to create projects table")?;

    // Migrate: add status column if it doesn't exist (safe to run on existing DBs)
    let status_exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('projects') WHERE name = 'status'",
        [],
        |row| row.get(0),
    )?;
    if status_exists == 0 {
        conn.execute(
            "ALTER TABLE projects ADD COLUMN status TEXT NOT NULL DEFAULT 'active'",
            [],
        )
        .context("Failed to add status column to projects table")?;
    }

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
    // Try to insert; if the project already exists (even if inactive), re-activate it
    conn.execute(
        "INSERT INTO projects (name, time_sum) VALUES (?1, 0)
         ON CONFLICT(name) DO UPDATE SET status = 'active'",
        params![name],
    )?;

    let mut stmt =
        conn.prepare("SELECT id, name, time_sum, status FROM projects WHERE name = ?1")?;
    let project = stmt.query_row(params![name], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            time_sum: row.get(2)?,
            status: row.get(3)?,
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

/// Get all active projects
pub fn get_all_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, time_sum, status FROM projects WHERE status = 'active' ORDER BY name",
    )?;

    let projects = stmt
        .query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                time_sum: row.get(2)?,
                status: row.get(3)?,
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
    let mut stmt = conn
        .prepare("SELECT start_time FROM instances WHERE project_id = ?1 AND stop_time IS NULL")?;

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

/// Get an active project by name
pub fn get_project_by_name(conn: &Connection, name: &str) -> Result<Option<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, time_sum, status FROM projects WHERE name = ?1 AND status = 'active'",
    )?;

    let result = stmt.query_row(params![name], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            time_sum: row.get(2)?,
            status: row.get(3)?,
        })
    });

    match result {
        Ok(project) => Ok(Some(project)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Check if a project has any active instances
pub fn has_active_instances(conn: &Connection, project_id: i64) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM instances WHERE project_id = ?1 AND stop_time IS NULL",
        params![project_id],
        |row| row.get(0),
    )?;

    Ok(count > 0)
}

/// Get all active projects that have a currently running instance (no stop time)
pub fn get_active_running_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.time_sum, p.status
         FROM projects p
         INNER JOIN instances i ON i.project_id = p.id
         WHERE p.status = 'active' AND i.stop_time IS NULL
         ORDER BY p.name",
    )?;

    let projects = stmt
        .query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                time_sum: row.get(2)?,
                status: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(projects)
}

/// Get the total seconds tracked for a project within a given UTC time range.
/// Active instances (no stop_time) are counted up to the current moment.
/// Each instance's contribution is clamped to [range_start, range_end].
pub fn get_project_time_in_range(
    conn: &Connection,
    project_id: i64,
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
) -> Result<i64> {
    let now = Utc::now();

    let mut stmt = conn.prepare(
        "SELECT start_time, stop_time FROM instances
         WHERE project_id = ?1
           AND start_time < ?3
           AND (stop_time > ?2 OR stop_time IS NULL)",
    )?;

    let rows = stmt
        .query_map(
            params![project_id, range_start.to_rfc3339(), range_end.to_rfc3339()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )?
        .collect::<Result<Vec<_>, _>>()?;

    let mut total_seconds = 0i64;
    for (start_str, stop_str) in rows {
        let start = DateTime::parse_from_rfc3339(&start_str)
            .context("Failed to parse start time")?
            .with_timezone(&Utc);
        let stop = match stop_str {
            Some(s) => DateTime::parse_from_rfc3339(&s)
                .context("Failed to parse stop time")?
                .with_timezone(&Utc),
            None => now,
        };
        let clamped_start = start.max(range_start);
        let clamped_stop = stop.min(range_end);
        total_seconds += (clamped_stop - clamped_start).num_seconds().max(0);
    }

    Ok(total_seconds)
}

/// Get all archived (inactive) projects
pub fn get_archived_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, time_sum, status FROM projects WHERE status = 'inactive' ORDER BY name",
    )?;

    let projects = stmt
        .query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                time_sum: row.get(2)?,
                status: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(projects)
}

/// Get all instances for a project, ordered by start time descending
pub fn get_instances_for_project(conn: &Connection, project_id: i64) -> Result<Vec<Instance>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, start_time, stop_time FROM instances
         WHERE project_id = ?1
         ORDER BY start_time DESC",
    )?;

    let rows = stmt
        .query_map(params![project_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut instances = Vec::new();
    for (id, proj_id, start_str, stop_str) in rows {
        let start_time = DateTime::parse_from_rfc3339(&start_str)
            .context("Failed to parse start time")?
            .with_timezone(&Utc);
        let stop_time = match stop_str {
            Some(s) => Some(
                DateTime::parse_from_rfc3339(&s)
                    .context("Failed to parse stop time")?
                    .with_timezone(&Utc),
            ),
            None => None,
        };
        instances.push(Instance {
            id,
            project_id: proj_id,
            start_time,
            stop_time,
        });
    }

    Ok(instances)
}

/// Get an instance by ID
pub fn get_instance_by_id(conn: &Connection, instance_id: i64) -> Result<Option<Instance>> {
    let mut stmt =
        conn.prepare("SELECT id, project_id, start_time, stop_time FROM instances WHERE id = ?1")?;

    let result = stmt.query_row(params![instance_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    });

    match result {
        Ok((id, project_id, start_str, stop_str)) => {
            let start_time = DateTime::parse_from_rfc3339(&start_str)
                .context("Failed to parse start time")?
                .with_timezone(&Utc);
            let stop_time = match stop_str {
                Some(s) => Some(
                    DateTime::parse_from_rfc3339(&s)
                        .context("Failed to parse stop time")?
                        .with_timezone(&Utc),
                ),
                None => None,
            };
            Ok(Some(Instance {
                id,
                project_id,
                start_time,
                stop_time,
            }))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Delete an instance by ID.
/// If the instance was stopped, its duration is subtracted from the project's time_sum.
/// MAX(0, ...) guards against time_sum going negative due to any data inconsistency.
pub fn delete_instance(conn: &Connection, instance: &Instance) -> Result<()> {
    if let Some(duration) = instance.duration() {
        conn.execute(
            "UPDATE projects SET time_sum = MAX(0, time_sum - ?1) WHERE id = ?2",
            params![duration, instance.project_id],
        )?;
    }

    conn.execute("DELETE FROM instances WHERE id = ?1", params![instance.id])?;

    Ok(())
}

/// Soft-delete a project by setting its status to 'inactive'
pub fn delete_project(conn: &Connection, project_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE projects SET status = 'inactive' WHERE id = ?1",
        params![project_id],
    )?;

    Ok(())
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
    fn test_get_active_running_projects_none_running() {
        let conn = setup_in_memory_db();
        upsert_project(&conn, "proj1").unwrap();

        let result = get_active_running_projects(&conn).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_active_running_projects_with_running() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "proj1").unwrap();
        let start = Utc::now() - Duration::seconds(60);
        create_instance(&conn, project.id, start).unwrap();

        let result = get_active_running_projects(&conn).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "proj1");
    }

    #[test]
    fn test_get_active_running_projects_stopped_not_included() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "proj1").unwrap();
        let start = Utc::now() - Duration::seconds(60);
        create_instance(&conn, project.id, start).unwrap();
        stop_timer(&conn, project.id, Utc::now()).unwrap();

        let result = get_active_running_projects(&conn).unwrap();
        assert!(result.is_empty());
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

    #[test]
    fn test_get_project_time_in_range_fully_inside() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "test").unwrap();
        let range_start = Utc::now() - Duration::seconds(3600);
        let start = Utc::now() - Duration::seconds(600);
        let stop = Utc::now() - Duration::seconds(300);
        create_instance(&conn, project.id, start).unwrap();
        stop_timer(&conn, project.id, stop).unwrap();

        let result = get_project_time_in_range(&conn, project.id, range_start, Utc::now()).unwrap();
        // Duration is ~300s; allow 2s tolerance for timing
        assert!((result - 300).abs() <= 2, "Expected ~300s, got {result}");
    }

    #[test]
    fn test_get_project_time_in_range_outside_range_returns_zero() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "test").unwrap();
        let start = Utc::now() - Duration::seconds(3600);
        let stop = Utc::now() - Duration::seconds(3000);
        create_instance(&conn, project.id, start).unwrap();
        stop_timer(&conn, project.id, stop).unwrap();

        // Query a range that doesn't include the instance
        let range_start = Utc::now() - Duration::seconds(100);
        let result = get_project_time_in_range(&conn, project.id, range_start, Utc::now()).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_get_project_time_in_range_active_instance() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "test").unwrap();
        let range_start = Utc::now() - Duration::seconds(3600);
        // Start an instance 120s ago with no stop time
        let start = Utc::now() - Duration::seconds(120);
        create_instance(&conn, project.id, start).unwrap();

        let result = get_project_time_in_range(&conn, project.id, range_start, Utc::now()).unwrap();
        // Active instance from 120s ago; allow 2s tolerance
        assert!((result - 120).abs() <= 2, "Expected ~120s, got {result}");
    }

    #[test]
    fn test_get_archived_projects_empty() {
        let conn = setup_in_memory_db();
        upsert_project(&conn, "active_proj").unwrap();

        let result = get_archived_projects(&conn).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_archived_projects_after_delete() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "myproject").unwrap();
        delete_project(&conn, project.id).unwrap();

        let result = get_archived_projects(&conn).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "myproject");
        assert_eq!(result[0].status, "inactive");
    }

    #[test]
    fn test_get_archived_projects_excludes_active() {
        let conn = setup_in_memory_db();
        upsert_project(&conn, "active_proj").unwrap();
        let project = upsert_project(&conn, "archived_proj").unwrap();
        delete_project(&conn, project.id).unwrap();

        let result = get_archived_projects(&conn).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "archived_proj");
    }

    #[test]
    fn test_get_archived_projects_time_sum_preserved() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "timed_proj").unwrap();
        let start = Utc::now() - Duration::seconds(500);
        create_instance(&conn, project.id, start).unwrap();
        stop_timer(&conn, project.id, Utc::now()).unwrap();
        delete_project(&conn, project.id).unwrap();

        let result = get_archived_projects(&conn).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].time_sum >= 499 && result[0].time_sum <= 502);
    }

    #[test]
    fn test_upsert_project_reactivates_inactive_project() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "myproject").unwrap();
        delete_project(&conn, project.id).unwrap();

        // Verify project is now inactive
        let inactive = get_project_by_name(&conn, "myproject").unwrap();
        assert!(
            inactive.is_none(),
            "Project should be inactive after deletion"
        );

        // Upserting again should re-activate the project
        let reactivated = upsert_project(&conn, "myproject").unwrap();
        assert_eq!(reactivated.status, "active");
        assert_eq!(reactivated.id, project.id);
    }

    #[test]
    fn test_get_project_time_in_range_clamped() {
        let conn = setup_in_memory_db();
        let project = upsert_project(&conn, "test").unwrap();
        // Instance spans 600s, but range only covers last 200s of it
        let start = Utc::now() - Duration::seconds(600);
        let stop = Utc::now() - Duration::seconds(100);
        create_instance(&conn, project.id, start).unwrap();
        stop_timer(&conn, project.id, stop).unwrap();

        // Range starts 300s ago, so only 200s of the instance falls inside
        let range_start = Utc::now() - Duration::seconds(300);
        let result = get_project_time_in_range(&conn, project.id, range_start, Utc::now()).unwrap();
        assert!((result - 200).abs() <= 2, "Expected ~200s, got {result}");
    }
}
