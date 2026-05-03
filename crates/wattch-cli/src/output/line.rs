use crate::output::SampleRow;

pub fn format_sample_line(sample: &SampleRow) -> String {
    format!(
        "{:.3}s source={} name={} energy_j={:.3} delta_j={:.3} power_w={:.3}",
        sample.elapsed_s,
        sample.source_id,
        sample.source_name,
        sample.energy_j,
        sample.delta_j,
        sample.power_w
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_sample_line() {
        let row = SampleRow {
            elapsed_s: 0.1,
            source_id: 1,
            source_name: "rapl:package-0".to_string(),
            energy_j: 12403.551,
            delta_j: 0.423,
            power_w: 4.23,
            interval_ns: 100_000_000,
            counter_wrap: false,
        };

        assert_eq!(
            super::format_sample_line(&row),
            "0.100s source=1 name=rapl:package-0 energy_j=12403.551 delta_j=0.423 power_w=4.230"
        );
    }
}
