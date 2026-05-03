use std::time::Duration;

use crate::errors::{Result, WattchError};

pub fn parse_duration(value: &str) -> Result<Duration> {
    if value.is_empty() {
        return Err(WattchError::BadRequest(
            "duration must not be empty".to_string(),
        ));
    }

    if let Some(number) = value.strip_suffix("ms") {
        return parse_number(number, value).map(Duration::from_millis);
    }

    if let Some(number) = value.strip_suffix('s') {
        return parse_number(number, value).map(Duration::from_secs);
    }

    if let Some(number) = value.strip_suffix('m') {
        return parse_number(number, value).map(|minutes| Duration::from_secs(minutes * 60));
    }

    Err(WattchError::BadRequest(format!(
        "invalid duration {value:?}: expected suffix ms, s, or m"
    )))
}

fn parse_number(number: &str, original: &str) -> Result<u64> {
    if number.is_empty() {
        return Err(WattchError::BadRequest(format!(
            "invalid duration {original:?}: missing number"
        )));
    }

    number
        .parse::<u64>()
        .map_err(|error| WattchError::BadRequest(format!("invalid duration {original:?}: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_ms() {
        assert_eq!(
            parse_duration("100ms").expect("duration"),
            Duration::from_millis(100)
        );
    }

    #[test]
    fn parse_duration_seconds() {
        assert_eq!(
            parse_duration("5s").expect("duration"),
            Duration::from_secs(5)
        );
    }

    #[test]
    fn parse_duration_minutes() {
        assert_eq!(
            parse_duration("2m").expect("duration"),
            Duration::from_secs(120)
        );
    }

    #[test]
    fn parse_duration_rejects_missing_unit() {
        assert!(parse_duration("10").is_err());
    }

    #[test]
    fn parse_duration_rejects_unknown_unit() {
        assert!(parse_duration("1h").is_err());
    }
}
