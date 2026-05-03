pub mod config;
pub mod duration;
pub mod errors;
pub mod framing;
pub mod sources;
pub mod summary;
pub mod time;

pub use config::{ServiceConfig, DEFAULT_CONFIG_PATH, DEFAULT_SOCKET_MODE, DEFAULT_SOCKET_PATH};
pub use duration::parse_duration;
pub use errors::{Result, WattchError};
pub use framing::{
    decode_frame, encode_frame, read_frame_async, write_frame_async, MAX_FRAME_SIZE,
};
pub use sources::powercap::{
    compute_delta_j, discover_powercap_sources, microjoules_to_joules, PowercapSource,
    PRODUCTION_POWER_CAP_ROOT,
};
pub use summary::{SourceSummary, SummaryAggregator};

pub const MIN_INTERVAL_NS: u64 = 100_000;

pub fn validate_interval_ns(interval_ns: u64) -> Result<()> {
    if interval_ns < MIN_INTERVAL_NS {
        return Err(WattchError::IntervalTooLow {
            interval_ns,
            min_interval_ns: MIN_INTERVAL_NS,
        });
    }

    Ok(())
}

pub fn validate_source_ids(source_ids: &[u32], available_sources: &[PowercapSource]) -> Result<()> {
    for source_id in source_ids {
        match available_sources
            .iter()
            .find(|source| source.source_id == *source_id)
        {
            Some(source) if source.available => {}
            Some(_) => return Err(WattchError::SourceUnavailable(*source_id)),
            None => return Err(WattchError::SourceNotFound(*source_id)),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn source(source_id: u32) -> PowercapSource {
        PowercapSource {
            source_id,
            name: format!("rapl:{source_id}"),
            kind: "rapl".to_string(),
            unit: "joule".to_string(),
            available: true,
            path: PathBuf::from("/fake"),
            max_energy_j: 10.0,
        }
    }

    #[test]
    fn validate_interval_accepts_100us() {
        validate_interval_ns(100_000).expect("100 us should be accepted");
    }

    #[test]
    fn validate_interval_rejects_below_100us() {
        let error = validate_interval_ns(99_999).expect_err("below 100 us should fail");
        assert!(matches!(error, WattchError::IntervalTooLow { .. }));
    }

    #[test]
    fn validate_source_ids_accepts_existing_ids() {
        let sources = vec![source(1), source(2)];
        validate_source_ids(&[1, 2], &sources).expect("existing ids should be accepted");
    }

    #[test]
    fn validate_source_ids_rejects_missing_ids() {
        let sources = vec![source(1)];
        let error = validate_source_ids(&[2], &sources).expect_err("missing id should fail");
        assert!(matches!(error, WattchError::SourceNotFound(2)));
    }
}
