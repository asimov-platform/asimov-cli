// This is free and unencumbered software released into the public domain.

use clientele::crates::clap::Args;
use std::time::Duration;

#[derive(Args, Clone, Debug, Default)]
pub struct CacheArgs {
    /// Maximum acceptable cache age (for example, 1h or 7d). Default is module-specific.
    #[arg(long, value_name = "DURATION", value_parser = parse_max_age)]
    pub max_age: Option<Duration>,

    /// Wait for fresh data, allowing at most this duration for the command (for example, 30s).
    #[arg(long, value_name = "DURATION", value_parser = humantime::parse_duration)]
    pub deadline: Option<Duration>,
}

impl CacheArgs {
    pub fn max_age_option(&self) -> Option<String> {
        self.max_age
            .map(|value| format!("--max-age={}", humantime::format_duration(value)))
    }

    pub fn deadline_option(&self) -> Option<String> {
        self.deadline
            .map(|value| format!("--deadline={}", humantime::format_duration(value)))
    }
}

fn parse_max_age(value: &str) -> Result<Duration, String> {
    let duration = humantime::parse_duration(value).map_err(|error| error.to_string())?;
    if duration.is_zero() {
        return Err("max age must be positive".into());
    }
    Ok(duration)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clientele::crates::clap::Parser;

    #[derive(Parser)]
    struct Command {
        #[command(flatten)]
        cache: CacheArgs,
    }

    #[test]
    fn forwards_only_explicit_options() {
        let args = Command::try_parse_from(["test"]).unwrap().cache;
        assert_eq!(args.max_age_option(), None);
        assert_eq!(args.deadline_option(), None);
        let args = Command::try_parse_from(["test", "--max-age", "1h", "--deadline", "30s"])
            .unwrap()
            .cache;
        assert_eq!(args.max_age_option().as_deref(), Some("--max-age=1h"));
        assert_eq!(args.deadline_option().as_deref(), Some("--deadline=30s"));
    }

    #[test]
    fn rejects_invalid_options() {
        for args in [["test", "--max-age", "0s"], ["test", "--deadline", "nope"]] {
            assert!(Command::try_parse_from(args).is_err());
        }
        assert!(Command::try_parse_from(["test", "--wait"]).is_err());
    }
}
