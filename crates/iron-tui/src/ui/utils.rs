//! Shared UI utility functions
//! 
//! Common helpers used across multiple views.

use chrono::{DateTime, Utc};

/// Format a DateTime as a relative time string (e.g., "3 days ago", "never")
pub fn format_relative_time(time: Option<DateTime<Utc>>) -> String {
    match time {
        Some(dt) => {
            let now = Utc::now();
            let duration = now.signed_duration_since(dt);

            if duration.num_minutes() < 1 {
                "just now".to_string()
            } else if duration.num_minutes() < 60 {
                let mins = duration.num_minutes();
                format!("{} min{} ago", mins, if mins == 1 { "" } else { "s" })
            } else if duration.num_hours() < 24 {
                let hours = duration.num_hours();
                format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
            } else if duration.num_days() < 7 {
                let days = duration.num_days();
                format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
            } else if duration.num_weeks() < 4 {
                let weeks = duration.num_weeks();
                format!("{} week{} ago", weeks, if weeks == 1 { "" } else { "s" })
            } else {
                let months = duration.num_days() / 30;
                if months < 12 {
                    format!("{} month{} ago", months, if months == 1 { "" } else { "s" })
                } else {
                    "over a year ago".to_string()
                }
            }
        }
        None => "never".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_relative_time_none() {
        assert_eq!(format_relative_time(None), "never");
    }

    #[test]
    fn test_format_relative_time_just_now() {
        let now = Utc::now();
        assert_eq!(format_relative_time(Some(now)), "just now");
    }

    #[test]
    fn test_format_relative_time_minutes() {
        let time = Utc::now() - chrono::Duration::minutes(5);
        assert_eq!(format_relative_time(Some(time)), "5 mins ago");
    }

    #[test]
    fn test_format_relative_time_hours() {
        let time = Utc::now() - chrono::Duration::hours(3);
        assert_eq!(format_relative_time(Some(time)), "3 hours ago");
    }

    #[test]
    fn test_format_relative_time_days() {
        let time = Utc::now() - chrono::Duration::days(2);
        assert_eq!(format_relative_time(Some(time)), "2 days ago");
    }

    #[test]
    fn test_format_relative_time_weeks() {
        let time = Utc::now() - chrono::Duration::weeks(2);
        assert_eq!(format_relative_time(Some(time)), "2 weeks ago");
    }

    #[test]
    fn test_format_relative_time_months() {
        let time = Utc::now() - chrono::Duration::days(60);
        assert_eq!(format_relative_time(Some(time)), "2 months ago");
    }

    #[test]
    fn test_format_relative_time_over_year() {
        let time = Utc::now() - chrono::Duration::days(400);
        assert_eq!(format_relative_time(Some(time)), "over a year ago");
    }
}
