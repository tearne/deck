//! Harmonised on-disk fault reports. Every incident — whatever its kind — lands
//! in one directory under the state dir, named so a plain alphabetical listing
//! reads as a chronological log:
//!
//! `YYYY-MM-DD_HHMMSS-<kind>-<label>`
//!
//! Some kinds are folders (holding preserved files), some are single text files;
//! the naming is shared either way.

use std::path::PathBuf;

pub(crate) fn dir() -> PathBuf {
    crate::xdg::state_dir().join("error_reports")
}

/// A dated, type-tagged report name. Callers use it as-is for a folder, or
/// append an extension for a file.
pub(crate) fn stamped_name(kind: &str, label: &str) -> String {
    format!("{}-{kind}-{label}", utc_stamp(now_unix()))
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `YYYY-MM-DD_HHMMSS` in UTC — a lexical sort of these equals a chronological one.
fn utc_stamp(secs: u64) -> String {
    let (year, month, day) = civil_from_days((secs / 86_400) as i64);
    let time = secs % 86_400;
    let (hour, minute, second) = (time / 3600, (time % 3600) / 60, time % 60);
    format!("{year:04}-{month:02}-{day:02}_{hour:02}{minute:02}{second:02}")
}

/// Calendar date from a Unix day number. Howard Hinnant's `civil_from_days`.
pub(crate) fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (year + if month <= 2 { 1 } else { 0 }, month, day)
}
