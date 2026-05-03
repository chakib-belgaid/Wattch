use wattch_proto::wattch::v1::Source;

use crate::output::{json_escape, SampleRow};

pub fn format_sources_json(sources: &[Source]) -> String {
    let rows = sources
        .iter()
        .map(|source| {
            format!(
                "  {{\n    \"id\": {},\n    \"name\": \"{}\",\n    \"kind\": \"{}\",\n    \"unit\": \"{}\",\n    \"available\": {}\n  }}",
                source.source_id,
                json_escape(&source.name),
                json_escape(&source.kind),
                json_escape(&source.unit),
                source.available
            )
        })
        .collect::<Vec<_>>();
    format!("[\n{}\n]", rows.join(",\n"))
}

pub fn format_sample_jsonl_row(sample: &SampleRow) -> String {
    format!(
        "{{\"elapsed_s\":{},\"source_id\":{},\"source_name\":\"{}\",\"energy_j\":{},\"delta_j\":{},\"power_w\":{},\"interval_ns\":{},\"counter_wrap\":{}}}",
        sample.elapsed_s,
        sample.source_id,
        json_escape(&sample.source_name),
        sample.energy_j,
        sample.delta_j,
        sample.power_w,
        sample.interval_ns,
        sample.counter_wrap
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_sources_json() {
        let output = super::format_sources_json(&[Source {
            source_id: 1,
            name: "rapl:package-0".to_string(),
            kind: "rapl".to_string(),
            unit: "joule".to_string(),
            available: true,
        }]);

        assert!(output.contains("\"id\": 1"));
        assert!(output.contains("\"name\": \"rapl:package-0\""));
    }

    #[test]
    fn format_sample_jsonl_row() {
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
            super::format_sample_jsonl_row(&row),
            "{\"elapsed_s\":0.1,\"source_id\":1,\"source_name\":\"rapl:package-0\",\"energy_j\":12403.551,\"delta_j\":0.423,\"power_w\":4.23,\"interval_ns\":100000000,\"counter_wrap\":false}"
        );
    }
}
