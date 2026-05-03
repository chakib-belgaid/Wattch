use std::time::Duration;

pub fn parse_duration_arg(value: &str) -> Result<Duration, String> {
    wattch_core::parse_duration(value).map_err(|error| error.to_string())
}
