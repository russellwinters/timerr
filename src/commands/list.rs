use anyhow::Result;
use rusqlite::Connection;

use crate::db::get_all_projects;
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
        println!("  {} - {}", project.name, format_duration(project.time_sum));
        total_time += project.time_sum;
    }

    println!();
    println!("Total time tracked: {}", format_duration(total_time));

    Ok(())
}
