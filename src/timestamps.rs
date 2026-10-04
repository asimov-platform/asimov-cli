// This is free and unencumbered software released into the public domain.

use jiff::{Unit, Zoned};

#[tracing::instrument]
pub fn format_ts_diff(a: &Zoned, b: &Zoned) -> Result<String, jiff::Error> {
    if a.timestamp() <= b.timestamp() {
        return Ok("just now".into());
    }
    let span = a.since(b)?;

    tracing::trace!(?span);

    let years = span.total((Unit::Year, a))?.floor() as i64;
    tracing::trace!(?years);
    if years >= 2 {
        return Ok(format!("{years} years ago"));
    }
    if years == 1 {
        return Ok("one year ago".into());
    }

    let months = span.total((Unit::Month, a))?.floor() as i64;
    tracing::trace!(?months);
    if months >= 2 {
        return Ok(format!("{months} months ago"));
    }
    if months == 1 {
        return Ok("one month ago".into());
    }

    let weeks = span.total((Unit::Week, a))?.floor() as i64;
    tracing::trace!(?weeks);
    if weeks >= 2 {
        return Ok(format!("{weeks} weeks ago"));
    }
    if weeks == 1 {
        return Ok("one week ago".into());
    }

    let days = span.total((Unit::Day, a))?.floor() as i64;
    tracing::trace!(?days);
    if days >= 2 {
        return Ok(format!("{days} days ago"));
    }
    if days == 1 {
        return Ok("one day ago".into());
    }

    let hours = span.total((Unit::Hour, a))?.floor() as i64;
    tracing::trace!(?hours);
    if hours >= 2 {
        return Ok(format!("{hours} hours ago"));
    }
    if hours == 1 {
        return Ok("one hour ago".into());
    }

    let minutes = span.total((Unit::Minute, a))?.floor() as i64;
    tracing::trace!(?minutes);
    if minutes >= 2 {
        return Ok(format!("{minutes} minutes ago"));
    }
    if minutes == 1 {
        return Ok("one minute ago".into());
    }

    Ok("just now".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::ToSpan;

    #[test]
    fn test_format_ts_diff() {
        let now = "2026-10-04T12:00:00Z"
            .parse::<jiff::Timestamp>()
            .unwrap()
            .to_zoned(jiff::tz::TimeZone::UTC);
        let cases = [
            (3.years(), "3 years ago"),
            (1.year(), "one year ago"),
            (9.months().weeks(2), "9 months ago"),
            (6.weeks(), "one month ago"),
            (3.weeks(), "3 weeks ago"),
            (2.weeks().days(3), "2 weeks ago"),
            (1.week(), "one week ago"),
            (6.days(), "6 days ago"),
            (1.day(), "one day ago"),
            (10.hours(), "10 hours ago"),
            (1.hour(), "one hour ago"),
            (30.minutes(), "30 minutes ago"),
            (1.minute(), "one minute ago"),
            (59.seconds(), "just now"),
            (1.seconds(), "just now"),
        ];

        for case in cases {
            let then = &now - case.0;
            assert_eq!(
                format_ts_diff(&now, &then).unwrap(),
                case.1,
                "input: {}",
                case.0
            );
        }
    }

    #[test]
    fn fixed_calendar_boundaries_and_future_times() {
        for (now, then, expected) in [
            (
                "2024-03-31T12:00:00Z",
                "2024-02-29T12:00:00Z",
                "one month ago",
            ),
            ("2024-03-01T12:00:00Z", "2024-02-28T12:00:00Z", "2 days ago"),
            (
                "2024-03-10T03:30:00-04:00",
                "2024-03-10T01:30:00-05:00",
                "one hour ago",
            ),
            (
                "2024-11-03T01:30:00-05:00",
                "2024-11-03T01:30:00-04:00",
                "one hour ago",
            ),
            ("2026-10-04T12:00:00Z", "2026-10-04T12:00:00Z", "just now"),
            ("2026-10-04T12:00:00Z", "2026-10-04T12:01:00Z", "just now"),
        ] {
            // Explicit rules avoid relying on a system time-zone database.
            let zone = if now.ends_with('Z') {
                jiff::tz::TimeZone::UTC
            } else {
                jiff::tz::TimeZone::posix("EST5EDT,M3.2.0,M11.1.0").unwrap()
            };
            let now = now
                .parse::<jiff::Timestamp>()
                .unwrap()
                .to_zoned(zone.clone());
            let then = then.parse::<jiff::Timestamp>().unwrap().to_zoned(zone);
            assert_eq!(
                format_ts_diff(&now, &then).unwrap(),
                expected,
                "{now} / {then}"
            );
        }
    }
}
