pub mod csv;
pub mod format;
pub mod jsonl;
pub mod line;
pub mod summary;
pub mod table;

use wattch_proto::wattch::v1::Sample;

#[derive(Clone, Debug)]
pub struct SampleRow {
    pub elapsed_s: f64,
    pub source_id: u32,
    pub source_name: String,
    pub energy_j: f64,
    pub delta_j: f64,
    pub power_w: f64,
    pub interval_ns: u64,
    pub counter_wrap: bool,
}

impl SampleRow {
    pub fn from_sample(elapsed_s: f64, source_name: impl Into<String>, sample: &Sample) -> Self {
        Self {
            elapsed_s,
            source_id: sample.source_id,
            source_name: source_name.into(),
            energy_j: sample.energy_j,
            delta_j: sample.delta_j,
            power_w: sample.power_w,
            interval_ns: sample.interval_ns,
            counter_wrap: sample.counter_wrap,
        }
    }
}

pub fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped
}
