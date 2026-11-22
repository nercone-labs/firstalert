use serde::{Deserialize, Serialize};
use std::fmt;

/// An instant, in milliseconds since the Unix epoch (UTC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Time(pub i64);

impl Time {
    /// The offset of Japan Standard Time from UTC: nine hours, with no daylight saving time.
    pub const JST_OFFSET_MILLIS: i64 = 9 * 3600 * 1000;

    pub fn millis(&self) -> i64 {
        self.0
    }

    /// Reads a time of JMA's relays, given in Japan Standard Time: `YYYY/MM/DD HH:MM`, with
    /// optional seconds and fraction (`:SS`, `:SS.fff`). A `-` may stand for the `/`.
    pub fn parse_jst(text: &str) -> Option<Time> {
        let text = text.trim();
        let (date, time) = text.split_once(' ')?;
        let mut date_parts = date.split(['/', '-']);
        let year: i64 = date_parts.next()?.parse().ok()?;
        let month: u32 = date_parts.next()?.parse().ok()?;
        let day: u32 = date_parts.next()?.parse().ok()?;
        if date_parts.next().is_some() {
            return None;
        }
        let (hour, minute, millis) = parse_clock(time.trim())?;
        let days = days_from_civil(year, month, day)?;
        let local = ((days * 24 + hour) * 60 + minute) * 60_000 + millis;
        Some(Time(local - Self::JST_OFFSET_MILLIS))
    }

    /// Reads an RFC 3339 time in UTC (`2026-11-06T10:53:00.000Z`) or with a numeric offset
    /// (`+09:00`).
    pub fn parse_rfc3339(text: &str) -> Option<Time> {
        let text = text.trim();
        let (date, rest) = text.split_once(['T', 't', ' '])?;
        let mut date_parts = date.split('-');
        let year: i64 = date_parts.next()?.parse().ok()?;
        let month: u32 = date_parts.next()?.parse().ok()?;
        let day: u32 = date_parts.next()?.parse().ok()?;
        if date_parts.next().is_some() {
            return None;
        }
        let (clock, offset_millis) = if let Some(clock) = rest.strip_suffix(['Z', 'z']) {
            (clock, 0)
        } else {
            let position = rest.rfind(['+', '-'])?;
            let (clock, offset) = rest.split_at(position);
            let sign = if offset.starts_with('-') { -1 } else { 1 };
            let (h, m) = offset[1..].split_once(':')?;
            let (h, m): (i64, i64) = (h.parse().ok()?, m.parse().ok()?);
            if h > 23 || m > 59 {
                return None;
            }
            (clock, sign * (h * 60 + m) * 60_000)
        };
        let (hour, minute, millis) = parse_clock(clock)?;
        let days = days_from_civil(year, month, day)?;
        Some(Time(((days * 24 + hour) * 60 + minute) * 60_000 + millis - offset_millis))
    }

    /// The civil date and time of this instant in Japan Standard Time:
    /// `(year, month, day, hour, minute, second)`.
    pub fn jst_fields(&self) -> (i64, u32, u32, u32, u32, u32) {
        let local = self.0 + Self::JST_OFFSET_MILLIS;
        let days = local.div_euclid(86_400_000);
        let in_day = local.rem_euclid(86_400_000) / 1000;
        let (year, month, day) = civil_from_days(days);
        (year, month, day, (in_day / 3600) as u32, (in_day / 60 % 60) as u32, (in_day % 60) as u32)
    }

    /// This instant in Japan Standard Time, written as the relays write it: `YYYY/MM/DD HH:MM:SS`.
    pub fn format_jst(&self) -> String {
        let (y, mo, d, h, mi, s) = self.jst_fields();
        format!("{y:04}/{mo:02}/{d:02} {h:02}:{mi:02}:{s:02}")
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format_jst())
    }
}

/// `HH:MM`, `HH:MM:SS` or `HH:MM:SS.fff` as hours, minutes and milliseconds into the minute.
fn parse_clock(text: &str) -> Option<(i64, i64, i64)> {
    let mut parts = text.split(':');
    let hour: i64 = parts.next()?.parse().ok()?;
    let minute: i64 = parts.next()?.parse().ok()?;
    let millis = match parts.next() {
        None => 0,
        Some(seconds) => {
            let (whole, fraction) = seconds.split_once('.').unwrap_or((seconds, ""));
            let whole: i64 = whole.parse().ok()?;
            if whole > 60 || !fraction.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            // Only the first three digits of the fraction are milliseconds.
            let digits: String = fraction.chars().chain("000".chars()).take(3).collect();
            whole * 1000 + digits.parse::<i64>().ok()?
        }
    };
    if parts.next().is_some() || hour > 23 || minute > 59 || hour < 0 || minute < 0 {
        return None;
    }
    Some((hour, minute, millis))
}

/// The number of days from 1970-01-01 to a date of the proleptic Gregorian calendar
/// (Howard Hinnant's `days_from_civil`). An impossible date is `None`.
fn days_from_civil(year: i64, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = month as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        _ => 28,
    }
}
