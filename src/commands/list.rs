use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, Local, TimeZone, Utc};
use rusqlite::Connection;

use crate::db::{get_all_projects, get_project_time_in_range};
use crate::utils::format_duration;

pub fn execute(conn: &Connection) -> Result<()> {
    let projects = get_all_projects(conn)?;

    if projects.is_empty() {
        println!("No projects yet. Start tracking with: timerr start <project_name>");
        return Ok(());
    }

    let now = Utc::now();

    // Start of today in local time, converted to UTC
    let today_start = {
        let today = Local::now().date_naive();
        let midnight = today.and_hms_opt(0, 0, 0).unwrap();
        Local
            .from_local_datetime(&midnight)
            .earliest()
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| midnight.and_utc())
    };

    // Start of the most recent Sunday in local time, converted to UTC
    let week_start = {
        let today = Local::now().date_naive();
        let days_since_sunday = today.weekday().num_days_from_sunday() as i64;
        let sunday = today - Duration::days(days_since_sunday);
        let sunday_midnight = sunday.and_hms_opt(0, 0, 0).unwrap();
        Local
            .from_local_datetime(&sunday_midnight)
            .earliest()
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| sunday_midnight.and_utc())
    };

    println!("Projects:");
    println!();

    let mut total_daily = 0i64;
    let mut total_weekly = 0i64;
    let mut total_all = 0i64;

    for project in &projects {
        let daily = get_project_time_in_range(conn, project.id, today_start, now)?;
        let weekly = get_project_time_in_range(conn, project.id, week_start, now)?;
        let total = get_project_time_in_range(conn, project.id, DateTime::UNIX_EPOCH, now)?;

        total_daily += daily;
        total_weekly += weekly;
        total_all += total;

        println!(
            "{}: Daily: {}   ---   Weekly: {}   ---   Total: {}",
            project.name,
            format_duration(daily),
            format_duration(weekly),
            format_duration(total)
        );
    }

    println!();
    println!("Totals:");
    println!(
        "Daily: {}   ---   Weekly: {}   ---   Total: {}",
        format_duration(total_daily),
        format_duration(total_weekly),
        format_duration(total_all)
    );

    Ok(())
}
