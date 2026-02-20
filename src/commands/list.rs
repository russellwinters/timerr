use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;

use crate::db::{get_active_instance_start_time, get_all_projects};
use crate::utils::format_duration;

pub fn execute(conn: &Connection) -> Result<()> {
    let projects = get_all_projects(conn)?;

    if projects.is_empty() {
        println!("No projects yet. Start tracking with: timerr start <project_name>");
        return Ok(());
    }

    println!("Projects:");
    println!();

    let mut total_time = 0i64;

    for project in &projects {
        let active_instance_seconds = if let Some(start_time) =
            get_active_instance_start_time(conn, project.id)?
        {
            (Utc::now() - start_time).num_seconds().max(0)
        } else {
            0
        };

        let display_time = project.time_sum + active_instance_seconds;
        println!("  {} - {}", project.name, format_duration(display_time));
        total_time += display_time;
    }

    println!();
    println!("Total time tracked: {}", format_duration(total_time));

    Ok(())
}
