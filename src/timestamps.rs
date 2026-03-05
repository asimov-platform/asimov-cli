// This is free and unencumbered software released into the public domain.

use jiff::{Unit, Zoned};

#[tracing::instrument]
pub fn format_ts_diff(a: &Zoned, b: &Zoned) -> Result<String, jiff::Error> {
    let span = a - b;

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
        tracing_subscriber::fmt::init();

        let now = jiff::Zoned::now();

        // There is a decent chance that the days>=1 tests are flaky.
        // Could try adding extra buffer if something starts failing.
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
}
