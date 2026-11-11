//! Wall clock for summaries and export file names (UTC, no extra dependencies).

use std::time::{SystemTime, UNIX_EPOCH};

use soulscale_core::Clock;

/// (year, month, day, hour, minute, second) in UTC.
pub fn utc_now() -> (i64, u32, u32, u32, u32, u32) {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d, (rem / 3600) as u32, (rem % 3600 / 60) as u32, (rem % 60) as u32)
}

/// "YYYY-MM-DD HH:MM:SS UTC" (Python used local time with the same layout).
pub fn stamp_human() -> String {
    let (y, mo, d, h, mi, s) = utc_now();
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02} UTC")
}

/// "YYYYMMDD_HHMMSS" for export file names.
pub fn stamp_file() -> String {
    let (y, mo, d, h, mi, s) = utc_now();
    format!("{y:04}{mo:02}{d:02}_{h:02}{mi:02}{s:02}")
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> String {
        stamp_human()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_have_expected_shape() {
        let h = stamp_human();
        assert_eq!(h.len(), "2026-11-10 22:42:00 UTC".len());
        assert!(h.ends_with(" UTC"));
        assert_eq!(stamp_file().len(), "20261110_224200".len());
        let (y, m, d, ..) = utc_now();
        assert!(y >= 2024 && (1..=12).contains(&m) && (1..=31).contains(&d));
    }
}
