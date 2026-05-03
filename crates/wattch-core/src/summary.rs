use std::collections::BTreeMap;

use wattch_proto::wattch::v1::Sample;

#[derive(Clone, Debug, PartialEq)]
pub struct SourceSummary {
    pub source_id: u32,
    pub source_name: String,
    pub sample_count: u64,
    pub total_delta_j: f64,
    pub avg_power_w: Option<f64>,
    pub max_power_w: Option<f64>,
    pub min_power_w: Option<f64>,
    pub first_monotonic_ns: Option<u64>,
    pub last_monotonic_ns: Option<u64>,
    pub counter_wrap_count: u64,
}

impl SourceSummary {
    pub fn new(source_id: u32, source_name: impl Into<String>) -> Self {
        Self {
            source_id,
            source_name: source_name.into(),
            sample_count: 0,
            total_delta_j: 0.0,
            avg_power_w: None,
            max_power_w: None,
            min_power_w: None,
            first_monotonic_ns: None,
            last_monotonic_ns: None,
            counter_wrap_count: 0,
        }
    }

    pub fn observe(&mut self, sample: &Sample) {
        self.sample_count += 1;
        self.total_delta_j += sample.delta_j;
        self.first_monotonic_ns.get_or_insert(sample.monotonic_ns);
        self.last_monotonic_ns = Some(sample.monotonic_ns);

        self.max_power_w = Some(
            self.max_power_w
                .map_or(sample.power_w, |current| current.max(sample.power_w)),
        );
        self.min_power_w = Some(
            self.min_power_w
                .map_or(sample.power_w, |current| current.min(sample.power_w)),
        );

        if sample.counter_wrap {
            self.counter_wrap_count += 1;
        }

        self.avg_power_w = self.duration_s().and_then(|duration_s| {
            if duration_s > 0.0 {
                Some(self.total_delta_j / duration_s)
            } else {
                None
            }
        });
    }

    pub fn duration_s(&self) -> Option<f64> {
        let first = self.first_monotonic_ns?;
        let last = self.last_monotonic_ns?;
        if self.sample_count < 2 || last <= first {
            return None;
        }

        Some((last - first) as f64 / 1_000_000_000.0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct SummaryAggregator {
    summaries: BTreeMap<u32, SourceSummary>,
}

impl SummaryAggregator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, source_id: u32, source_name: impl Into<String>, sample: &Sample) {
        self.summaries
            .entry(source_id)
            .or_insert_with(|| SourceSummary::new(source_id, source_name))
            .observe(sample);
    }

    pub fn summaries(&self) -> Vec<SourceSummary> {
        self.summaries.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(monotonic_ns: u64, delta_j: f64, power_w: f64, counter_wrap: bool) -> Sample {
        Sample {
            source_id: 1,
            monotonic_ns,
            energy_j: 0.0,
            delta_j,
            power_w,
            interval_ns: 100_000_000,
            counter_wrap,
        }
    }

    #[test]
    fn summary_tracks_total_energy() {
        let mut summary = SourceSummary::new(1, "rapl:package-0");
        summary.observe(&sample(100, 1.25, 4.0, false));
        summary.observe(&sample(200, 2.75, 5.0, false));

        assert_eq!(summary.total_delta_j, 4.0);
    }

    #[test]
    fn summary_tracks_avg_power() {
        let mut summary = SourceSummary::new(1, "rapl:package-0");
        summary.observe(&sample(0, 1.0, 4.0, false));
        summary.observe(&sample(1_000_000_000, 3.0, 5.0, false));

        assert_eq!(summary.avg_power_w, Some(4.0));
    }

    #[test]
    fn summary_tracks_max_power() {
        let mut summary = SourceSummary::new(1, "rapl:package-0");
        summary.observe(&sample(0, 1.0, 4.0, false));
        summary.observe(&sample(1, 1.0, 7.0, false));

        assert_eq!(summary.max_power_w, Some(7.0));
    }

    #[test]
    fn summary_tracks_min_power() {
        let mut summary = SourceSummary::new(1, "rapl:package-0");
        summary.observe(&sample(0, 1.0, 4.0, false));
        summary.observe(&sample(1, 1.0, 2.0, false));

        assert_eq!(summary.min_power_w, Some(2.0));
    }

    #[test]
    fn summary_handles_single_sample() {
        let mut summary = SourceSummary::new(1, "rapl:package-0");
        summary.observe(&sample(0, 1.0, 4.0, false));

        assert_eq!(summary.avg_power_w, None);
        assert_eq!(summary.duration_s(), None);
    }

    #[test]
    fn summary_counts_counter_wraps() {
        let mut summary = SourceSummary::new(1, "rapl:package-0");
        summary.observe(&sample(0, 1.0, 4.0, true));
        summary.observe(&sample(1, 1.0, 4.0, false));

        assert_eq!(summary.counter_wrap_count, 1);
    }
}
