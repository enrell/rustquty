//! Shared utility functions.

/// Returns the current UTC time as an ISO-8601 string (YYYY-MM-DDTHH:MM:SSZ).
pub fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let secs = now.as_secs();
    let (year, month, day, hour, min, sec) = unix_to_datetime(secs);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, min, sec
    )
}

/// Converts a Unix timestamp (seconds since 1970-01-01T00:00:00Z) to
/// (year, month, day, hour, minute, second) in UTC.
pub fn unix_to_datetime(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hour = time_of_day / 3600;
    let min = (time_of_day % 3600) / 60;
    let sec = time_of_day % 60;

    let mut y = 1970u64;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let leap = is_leap(y);
    let month_days: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 1u64;
    for &d in &month_days {
        if remaining < d {
            break;
        }
        remaining -= d;
        m += 1;
    }
    let d = remaining + 1;
    (y, m, d, hour, min, sec)
}

/// Returns true if `year` is a leap year in the Gregorian calendar.
pub fn is_leap(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chrono_now_returns_iso8601() {
        let ts = chrono_now();
        assert!(ts.ends_with('Z'));
        assert_eq!(ts.len(), 20);
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
    }

    #[test]
    fn test_chrono_now_year_is_reasonable() {
        let ts = chrono_now();
        let year: u32 = ts[..4].parse().unwrap();
        assert!((2025..=2100).contains(&year));
    }

    #[test]
    fn test_unix_to_datetime_epoch() {
        let (y, m, d, h, min, s) = unix_to_datetime(0);
        assert_eq!((y, m, d, h, min, s), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn test_unix_to_datetime_known_value() {
        let (y, m, d, h, min, s) = unix_to_datetime(1704067200);
        assert_eq!((y, m, d, h, min, s), (2024, 1, 1, 0, 0, 0));
    }

    #[test]
    fn test_unix_to_datetime_with_time() {
        let (y, m, d, h, min, s) = unix_to_datetime(1718451045);
        assert_eq!((y, m, d, h, min, s), (2024, 6, 15, 11, 30, 45));
    }

    #[test]
    fn test_is_leap_known_years() {
        assert!(!is_leap(2023));
        assert!(is_leap(2024));
        assert!(!is_leap(1900));
        assert!(is_leap(2000));
        assert!(!is_leap(2100));
    }
}
