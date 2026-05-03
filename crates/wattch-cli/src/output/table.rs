use wattch_proto::wattch::v1::Source;

use crate::output::SampleRow;

pub fn format_sources_table(sources: &[Source]) -> String {
    let mut rows = vec![vec![
        "ID".to_string(),
        "NAME".to_string(),
        "KIND".to_string(),
        "UNIT".to_string(),
        "AVAILABLE".to_string(),
    ]];

    for source in sources {
        rows.push(vec![
            source.source_id.to_string(),
            source.name.clone(),
            source.kind.clone(),
            source.unit.clone(),
            if source.available { "yes" } else { "no" }.to_string(),
        ]);
    }

    render_rows(&rows)
}

pub fn format_sample_table_header() -> String {
    render_rows(&[vec![
        "TIME".to_string(),
        "SOURCE".to_string(),
        "ENERGY_J".to_string(),
        "DELTA_J".to_string(),
        "POWER_W".to_string(),
    ]])
}

pub fn format_sample_table_row(sample: &SampleRow) -> String {
    render_rows(&[vec![
        format!("{:.3}s", sample.elapsed_s),
        sample.source_id.to_string(),
        format!("{:.3}", sample.energy_j),
        format!("{:.3}", sample.delta_j),
        format!("{:.3}", sample.power_w),
    ]])
}

pub fn render_rows(rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        return String::new();
    }

    let column_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths = vec![0; column_count];
    for row in rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }

    rows.iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(index, value)| {
                    if index + 1 == row.len() {
                        value.clone()
                    } else {
                        format!("{value:<width$}", width = widths[index])
                    }
                })
                .collect::<Vec<_>>()
                .join("  ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_sources_table() {
        let output = super::format_sources_table(&[Source {
            source_id: 1,
            name: "rapl:package-0".to_string(),
            kind: "rapl".to_string(),
            unit: "joule".to_string(),
            available: true,
        }]);

        assert!(output.contains("ID"));
        assert!(output.contains("rapl:package-0"));
        assert!(output.contains("yes"));
    }
}
