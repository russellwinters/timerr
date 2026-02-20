use anyhow::{bail, Result};
use chrono::Utc;
use rusqlite::Connection;

use crate::db::{create_instance, has_running_timer, upsert_project};

pub fn execute(conn: &Connection, project_name: &str) -> Result<()> {
    // Validate project name
    let project_name = project_name.trim();
    if project_name.is_empty() {
        bail!("Project name cannot be empty");
    }

    if project_name.len() > 100 {
        bail!("Project name is too long (max 100 characters)");
    }

    // Get or create the project
    let project = upsert_project(conn, project_name)?;

    // Check if there's already a running timer
    if has_running_timer(conn, project.id)? {
        bail!(
            "Project '{}' already has a running timer. Stop it first with: timerr stop {}",
            project_name,
            project_name
        );
    }

    // Create a new instance
    let start_time = Utc::now();
    create_instance(conn, project.id, start_time)?;

    println!("✓ Timer started for project '{}'", project_name);

    Ok(())
}
