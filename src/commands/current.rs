use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;

use crate::db::{get_active_instance_start_time, get_active_running_projects};
use crate::utils::format_duration;

pub fn execute(conn: &Connection) -> Result<()> {
    let projects = get_active_running_projects(conn)?;

    if projects.is_empty() {
        println!("No projects are currently being tracked.");
        return Ok(());
    }

    for project in &projects {
        let active_seconds =
            if let Some(start_time) = get_active_instance_start_time(conn, project.id)? {
                (Utc::now() - start_time).num_seconds().max(0)
            } else {
                // Defensive fallback: get_active_running_projects guarantees an active instance,
                // but guard against any unexpected race or data inconsistency.
                0
            };

        println!("{} - {}", project.name, format_duration(active_seconds));
    }

    Ok(())
}
