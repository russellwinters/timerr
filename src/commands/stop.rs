use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;

use crate::db::{get_project_by_name, stop_timer};
use crate::utils::format_duration;

pub fn execute(conn: &Connection, project_name: &str) -> Result<()> {
    let project_name = project_name.trim();

    // Get the project
    let project = match get_project_by_name(conn, project_name)? {
        Some(p) => p,
        None => {
            println!("No current timer for project '{}'", project_name);
            return Ok(());
        }
    };

    // Stop the timer
    let stop_time = Utc::now();
    match stop_timer(conn, project.id, stop_time)? {
        Some(duration) => {
            println!(
                "✓ Timer stopped for project '{}'. Duration: {}",
                project_name,
                format_duration(duration)
            );
        }
        None => {
            println!("No current timer for project '{}'", project_name);
        }
    }

    Ok(())
}
