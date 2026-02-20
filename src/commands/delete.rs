use anyhow::{bail, Result};
use rusqlite::Connection;

use crate::db::{delete_project, get_project_by_name, has_instances};

pub fn execute(conn: &Connection, project_name: &str) -> Result<()> {
    let project_name = project_name.trim();
    if project_name.is_empty() {
        bail!("Project name cannot be empty");
    }

    // Look up the active project
    let project = match get_project_by_name(conn, project_name)? {
        Some(p) => p,
        None => {
            bail!("Project '{}' not found", project_name);
        }
    };

    // Block deletion if the project has associated instances
    if has_instances(conn, project.id)? {
        bail!(
            "Cannot delete project '{}': it has time-tracking entries. \
             Stop any running timers before deleting.",
            project_name
        );
    }

    delete_project(conn, project.id)?;

    println!("✓ Project '{}' has been deleted", project_name);

    Ok(())
}
