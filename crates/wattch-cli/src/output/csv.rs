use wattch_proto::wattch::v1::Source;

use crate::output::SampleRow;

pub fn format_sources_csv(sources: &[Source]) -> String {
    let mut rows = vec!["id,name,kind,unit,available".to_string()];
    rows.extend(sources.iter().map(|source| {
        format!(
            "{},{},{},{},{}",
            source.source_id,
            csv_escape(&source.name),
            csv_escape(&source.kind),
            csv_escape(&source.unit),
            source.available
        )
    }));
    rows.join("\n")
}

pub fn format_sample_csv_header() -> String {
    "elapsed_s,source_id,source_name,energy_j,delta_j,power_w,interval_ns,counter_wrap".to_string()
}

pub fn format_sample_csv_row(sample: &SampleRow) -> String {
    format!(
        "{},{},{},{},{},{},{},{}",
        sample.elapsed_s,
        sample.source_id,
        csv_escape(&sample.source_name),
        sample.energy_j,
        sample.delta_j,
        sample.power_w,
        sample.interval_ns,
        sample.counter_wrap
    )
}

fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_sources_csv() {
        let output = super::format_sources_csv(&[Source {
            source_id: 1,
            name: "rapl:package-0".to_string(),
            kind: "rapl".to_string(),
            unit: "joule".to_string(),
            available: true,
        }]);

        assert_eq!(
            output,
            "id,name,kind,unit,available\n1,rapl:package-0,rapl,joule,true"
        );
    }

    #[test]
    fn format_sample_csv_header() {
        assert_eq!(
            super::format_sample_csv_header(),
            "elapsed_s,source_id,source_name,energy_j,delta_j,power_w,interval_ns,counter_wrap"
        );
    }

    #[test]
    fn format_sample_csv_row() {
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
            super::format_sample_csv_row(&row),
            "0.1,1,rapl:package-0,12403.551,0.423,4.23,100000000,false"
        );
    }
}
