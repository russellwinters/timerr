use chrono::{DateTime, Utc};

/// Represents a project with accumulated time
#[derive(Debug, Clone)]
pub struct Project {
    pub id: i64,
    pub name: String,
    #[allow(dead_code)]
    pub time_sum: i64, // Total time in seconds
    #[allow(dead_code)]
    // Status is stored and used in SQL filtering; field reserved for future use
    pub status: String, // 'active' or 'inactive'
}

/// Represents a time tracking instance for a project
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields are used for future functionality
pub struct Instance {
    pub id: i64,
    pub project_id: i64,
    pub start_time: DateTime<Utc>,
    pub stop_time: Option<DateTime<Utc>>,
}

impl Instance {
    /// Calculate the duration of this instance in seconds
    /// Returns None if the instance is still running (stop_time is None)
    #[allow(dead_code)] // Method reserved for future use
    pub fn duration(&self) -> Option<i64> {
        self.stop_time
            .map(|stop| (stop - self.start_time).num_seconds())
    }

    /// Check if this instance is currently running
    #[allow(dead_code)] // Method reserved for future use
    pub fn is_running(&self) -> bool {
        self.stop_time.is_none()
    }
}
