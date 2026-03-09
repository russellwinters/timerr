mod commands;
mod db;
mod models;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "timerr")]
#[command(version = "0.1.0")]
#[command(about = "A simple CLI time tracking application", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a timer for a project
    Start {
        /// Name of the project to track
        project_name: String,
    },
    /// Stop the running timer for a project
    Stop {
        /// Name of the project to stop tracking
        project_name: String,
    },
    /// List all projects with their total tracked time
    List,
    /// Show all projects with an active timer and the current instance time
    Current,
    /// Delete a project (soft-delete; project must have no time entries)
    Delete {
        /// Name of the project to delete
        project_name: String,
    },
    /// List all instances for a project
    InstanceList {
        /// ID of the project to list instances for
        project_id: i64,
    },
    /// Remove a specific instance
    InstanceRm {
        /// ID of the instance to remove
        instance_id: i64,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize database
    let conn = db::init_database()?;

    // Execute the appropriate command
    match cli.command {
        Commands::Start { project_name } => {
            commands::start::execute(&conn, &project_name)?;
        }
        Commands::Stop { project_name } => {
            commands::stop::execute(&conn, &project_name)?;
        }
        Commands::List => {
            commands::list::execute(&conn)?;
        }
        Commands::Current => {
            commands::current::execute(&conn)?;
        }
        Commands::Delete { project_name } => {
            commands::delete::execute(&conn, &project_name)?;
        }
        Commands::InstanceList { project_id } => {
            commands::instance_list::execute(&conn, project_id)?;
        }
        Commands::InstanceRm { instance_id } => {
            commands::instance_rm::execute(&conn, instance_id)?;
        }
    }

    Ok(())
}
