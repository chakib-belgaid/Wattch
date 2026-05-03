use wattch_core::SourceSummary;

use crate::output::json_escape;
use crate::output::table::render_rows;

pub fn format_summary_table(summaries: &[SourceSummary]) -> String {
    let mut rows = vec![vec![
        "SOURCE".to_string(),
        "ENERGY_J".to_string(),
        "AVG_POWER_W".to_string(),
        "MAX_POWER_W".to_string(),
        "SAMPLES".to_string(),
    ]];

    for summary in summaries {
        rows.push(vec![
            summary.source_name.clone(),
            format!("{:.3}", summary.total_delta_j),
            format_optional(summary.avg_power_w),
            format_optional(summary.max_power_w),
            summary.sample_count.to_string(),
        ]);
    }

    render_rows(&rows)
}

pub fn format_summary_json(summaries: &[SourceSummary]) -> String {
    let rows = summaries
        .iter()
        .map(|summary| {
            format!(
                "  {{\"source_id\":{},\"source_name\":\"{}\",\"sample_count\":{},\"total_energy_j\":{},\"avg_power_w\":{},\"max_power_w\":{},\"min_power_w\":{},\"counter_wrap_count\":{}}}",
                summary.source_id,
                json_escape(&summary.source_name),
                summary.sample_count,
                summary.total_delta_j,
                json_optional(summary.avg_power_w),
                json_optional(summary.max_power_w),
                json_optional(summary.min_power_w),
                summary.counter_wrap_count
            )
        })
        .collect::<Vec<_>>();
    format!("[\n{}\n]", rows.join(",\n"))
}

pub fn format_run_summary_csv(
    command: &str,
    exit_code: i32,
    duration_s: f64,
    summaries: &[SourceSummary],
) -> String {
    let mut rows = vec![
        "command,exit_code,duration_s,source_id,source_name,total_energy_j,avg_power_w,max_power_w,min_power_w,samples,counter_wrap_count"
            .to_string(),
    ];

    rows.extend(summaries.iter().map(|summary| {
        format!(
            "{},{},{},{},{},{},{},{},{},{},{}",
            csv_escape(command),
            exit_code,
            duration_s,
            summary.source_id,
            csv_escape(&summary.source_name),
            summary.total_delta_j,
            csv_optional(summary.avg_power_w),
            csv_optional(summary.max_power_w),
            csv_optional(summary.min_power_w),
            summary.sample_count,
            summary.counter_wrap_count
        )
    }));

    rows.join("\n")
}

fn format_optional(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "n/a".to_string())
}

fn json_optional(value: Option<f64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn csv_optional(value: Option<f64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
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

    fn summary() -> SourceSummary {
        SourceSummary {
            source_id: 1,
            source_name: "rapl:package-0".to_string(),
            sample_count: 48,
            total_delta_j: 21.421,
            avg_power_w: Some(4.444),
            max_power_w: Some(7.812),
            min_power_w: Some(2.0),
            first_monotonic_ns: Some(0),
            last_monotonic_ns: Some(1),
            counter_wrap_count: 0,
        }
    }

    #[test]
    fn format_summary_table() {
        let output = super::format_summary_table(&[summary()]);

        assert!(output.contains("SOURCE"));
        assert!(output.contains("rapl:package-0"));
        assert!(output.contains("21.421"));
    }

    #[test]
    fn format_summary_json() {
        let output = super::format_summary_json(&[summary()]);

        assert!(output.contains("\"source_id\":1"));
        assert!(output.contains("\"avg_power_w\":4.444"));
    }

    #[test]
    fn format_run_summary_csv() {
        let output = super::format_run_summary_csv("cargo test", 0, 4.82, &[summary()]);

        assert!(output.starts_with("command,exit_code,duration_s"));
        assert!(output.contains("cargo test,0,4.82,1,rapl:package-0,21.421"));
    }
}
