use anyhow::Result;
use rusqlite::Connection;

use crate::db::get_archived_projects;
use crate::utils::format_duration;

pub fn execute(conn: &Connection) -> Result<()> {
    let projects = get_archived_projects(conn)?;

    if projects.is_empty() {
        println!("No archived projects.");
        return Ok(());
    }

    println!("Archived Projects:");
    println!();

    for project in &projects {
        println!("{}: Total: {}", project.name, format_duration(project.time_sum));
    }

    Ok(())
}
