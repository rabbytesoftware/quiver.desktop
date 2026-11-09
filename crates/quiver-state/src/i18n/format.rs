//! Locale formatting of numbers, percentages, dates and relative times. English only for now:
//! the catalogue ships one locale. Dates are rendered in UTC; a time zone is a view concern.

/// `1234.5` -> `1,234.5` (grouping, at most 3 fraction digits like `Intl.NumberFormat`).
pub fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return if value.is_nan() {
            "NaN".into()
        } else if value > 0.0 {
            "∞".into()
        } else {
            "-∞".into()
        };
    }
    let rounded = format!("{:.3}", value.abs());
    let (int_part, frac) = rounded.split_once('.').unwrap_or((&rounded, ""));
    let frac = frac.trim_end_matches('0');
    let grouped = group_thousands(int_part);
    let negative = value < 0.0 && (int_part != "0" || !frac.is_empty());
    let sign = if negative { "-" } else { "" };
    if frac.is_empty() {
        format!("{sign}{grouped}")
    } else {
        format!("{sign}{grouped}.{frac}")
    }
}

/// Fixed number of fraction digits (`Intl` `minimumFractionDigits`).
pub fn format_number_fixed(value: f64, fraction_digits: usize) -> String {
    let text = format!("{:.*}", fraction_digits, value.abs());
    let (int_part, frac) = text.split_once('.').unwrap_or((&text, ""));
    let sign = if value < 0.0 { "-" } else { "" };
    if frac.is_empty() {
        format!("{sign}{}", group_thousands(int_part))
    } else {
        format!("{sign}{}.{frac}", group_thousands(int_part))
    }
}

fn group_thousands(digits: &str) -> String {
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Takes a fraction: `0.4` -> `40%` (and `40` -> `4,000%`, as `Intl` does).
pub fn format_percent(fraction: f64) -> String {
    format!("{}%", format_number((fraction * 100.0).round()))
}

const MINUTE: i64 = 60_000;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;

const UNITS: [(&str, i64); 7] = [
    ("year", 365 * DAY),
    ("month", 30 * DAY),
    ("week", 7 * DAY),
    ("day", DAY),
    ("hour", HOUR),
    ("minute", MINUTE),
    ("second", 1000),
];

/// "3 hours ago", "in 3 hours", "yesterday", "last week", "now": the largest unit that fits.
pub fn format_relative_time(value_ms: i64, now_ms: i64) -> String {
    let delta = value_ms - now_ms;
    for (unit, ms) in UNITS {
        if delta.abs() >= ms {
            return relative(delta / ms, unit);
        }
    }
    "now".into()
}

fn relative(amount: i64, unit: &str) -> String {
    let word = match (amount, unit) {
        (-1, "day") => Some("yesterday"),
        (1, "day") => Some("tomorrow"),
        (-1, _) => Some(match unit {
            "year" => "last year",
            "month" => "last month",
            "week" => "last week",
            "hour" => "1 hour ago",
            "minute" => "1 minute ago",
            _ => "1 second ago",
        }),
        (1, _) => Some(match unit {
            "year" => "next year",
            "month" => "next month",
            "week" => "next week",
            "hour" => "in 1 hour",
            "minute" => "in 1 minute",
            _ => "in 1 second",
        }),
        _ => None,
    };
    if let Some(word) = word {
        return word.into();
    }
    let n = amount.abs();
    let plural = if n == 1 { "" } else { "s" };
    if amount < 0 {
        format!("{} {unit}{plural} ago", format_number(n as f64))
    } else {
        format!("in {} {unit}{plural}", format_number(n as f64))
    }
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Civil date (UTC) from epoch milliseconds, via Howard Hinnant's days-to-civil algorithm.
fn civil(ms: i64) -> (i64, usize, i64, i64, i64) {
    let secs = ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, (month - 1) as usize, day, rem / 3600, rem % 3600 / 60)
}

/// `Mar 5, 2026` (UTC).
pub fn format_date(ms: i64) -> String {
    let (year, month, day, _, _) = civil(ms);
    format!("{} {day}, {year}", MONTHS[month])
}

/// `Mar 5, 2026, 2:30 PM` (UTC).
pub fn format_date_time(ms: i64) -> String {
    let (_, _, _, hour, minute) = civil(ms);
    let meridiem = if hour < 12 { "AM" } else { "PM" };
    let twelve = if hour % 12 == 0 { 12 } else { hour % 12 };
    format!("{}, {twelve}:{minute:02} {meridiem}", format_date(ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_use_the_separators_the_reader_expects() {
        assert_eq!(format_number(1234.5), "1,234.5");
        assert_eq!(format_number(1234.0), "1,234");
        assert_eq!(format_number(-1234567.0), "-1,234,567");
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number_fixed(0.5, 2), "0.50");
    }

    #[test]
    fn percent_takes_a_fraction_not_a_percentage() {
        assert_eq!(format_percent(0.4), "40%");
        assert_eq!(format_percent(40.0), "4,000%");
    }

    #[test]
    fn dates_render_in_utc_from_any_epoch() {
        let noon = 1_772_721_000_000; // 2026-03-05T14:30:00Z
        assert_eq!(format_date(noon), "Mar 5, 2026");
        assert_eq!(format_date_time(noon), "Mar 5, 2026, 2:30 PM");
        assert_eq!(format_date(0), "Jan 1, 1970");
        assert_eq!(format_date_time(0), "Jan 1, 1970, 12:00 AM");
    }

    fn ago(delta_ms: i64) -> String {
        format_relative_time(-delta_ms, 0)
    }

    #[test]
    fn relative_time_uses_the_largest_unit_that_fits() {
        assert_eq!(ago(45_000), "45 seconds ago");
        assert_eq!(ago(90_000), "1 minute ago");
        assert_eq!(ago(3 * HOUR), "3 hours ago");
        assert_eq!(ago(10 * DAY), "last week");
        assert_eq!(ago(60 * DAY), "2 months ago");
        assert_eq!(ago(400 * DAY), "last year");
    }

    #[test]
    fn relative_time_keeps_the_direction_and_prefers_words() {
        assert_eq!(ago(-3 * HOUR), "in 3 hours");
        assert_eq!(ago(DAY), "yesterday");
        assert_eq!(ago(-DAY), "tomorrow");
        assert_eq!(ago(0), "now");
        assert_eq!(ago(500), "now");
    }
}
