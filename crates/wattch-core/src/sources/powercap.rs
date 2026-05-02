use std::fs;
use std::path::{Path, PathBuf};

use wattch_proto::wattch::v1::Source;

use crate::errors::{Result, WattchError};

pub const PRODUCTION_POWER_CAP_ROOT: &str = "/sys/devices/virtual/powercap";

#[derive(Clone, Debug, PartialEq)]
pub struct PowercapSource {
    pub source_id: u32,
    pub name: String,
    pub kind: String,
    pub unit: String,
    pub available: bool,
    pub path: PathBuf,
    pub max_energy_j: f64,
}

impl PowercapSource {
    pub fn to_proto(&self) -> Source {
        Source {
            source_id: self.source_id,
            name: self.name.clone(),
            kind: self.kind.clone(),
            unit: self.unit.clone(),
            available: self.available,
        }
    }

    pub fn read_energy_j(&self) -> Result<f64> {
        read_energy_j(&self.path)
    }
}

pub fn discover_powercap_sources(root: &Path) -> Result<Vec<PowercapSource>> {
    let rapl_root = root.join("intel-rapl");
    if !rapl_root.exists() {
        return Ok(Vec::new());
    }

    let mut dirs = Vec::new();
    collect_dirs(&rapl_root, &mut dirs)?;

    let mut sources = Vec::new();
    for dir in dirs {
        if !is_complete_source_dir(&dir) {
            continue;
        }

        let zone_name = read_trimmed_file(&dir.join("name"))?;
        let max_energy_uj = read_u64_file(&dir.join("max_energy_range_uj"))?;
        sources.push(PowercapSource {
            source_id: (sources.len() + 1) as u32,
            name: format!("rapl:{zone_name}"),
            kind: "rapl".to_string(),
            unit: "joule".to_string(),
            available: true,
            path: dir,
            max_energy_j: microjoules_to_joules(max_energy_uj),
        });
    }

    Ok(sources)
}

pub fn compute_delta_j(previous: f64, current: f64, max_range: f64) -> (f64, bool) {
    if current >= previous {
        (current - previous, false)
    } else {
        ((max_range - previous) + current, true)
    }
}

pub fn microjoules_to_joules(value: u64) -> f64 {
    value as f64 / 1_000_000.0
}

fn read_energy_j(path: &Path) -> Result<f64> {
    let energy_uj = read_u64_file(&path.join("energy_uj"))?;
    Ok(microjoules_to_joules(energy_uj))
}

fn is_complete_source_dir(path: &Path) -> bool {
    path.join("energy_uj").is_file()
        && path.join("max_energy_range_uj").is_file()
        && path.join("name").is_file()
}

fn collect_dirs(path: &Path, dirs: &mut Vec<PathBuf>) -> Result<()> {
    dirs.push(path.to_path_buf());

    let mut entries = fs::read_dir(path)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        if entry.file_type()?.is_dir() {
            collect_dirs(&entry.path(), dirs)?;
        }
    }

    Ok(())
}

fn read_trimmed_file(path: &Path) -> Result<String> {
    Ok(fs::read_to_string(path)?.trim().to_string())
}

fn read_u64_file(path: &Path) -> Result<u64> {
    let value = read_trimmed_file(path)?;
    value.parse::<u64>().map_err(|error| {
        WattchError::Internal(format!(
            "failed to parse {} as u64: {error}",
            path.display()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_source(root: &Path, relative: &str, name: &str, energy_uj: u64, max_uj: u64) {
        let dir = root.join(relative);
        fs::create_dir_all(&dir).expect("create fake source dir");
        fs::write(dir.join("name"), name).expect("write name");
        fs::write(dir.join("energy_uj"), energy_uj.to_string()).expect("write energy");
        fs::write(dir.join("max_energy_range_uj"), max_uj.to_string()).expect("write max");
    }

    #[test]
    fn powercap_delta_without_wrap() {
        let (delta_j, counter_wrap) = compute_delta_j(1.0, 1.5, 10.0);

        assert_eq!(delta_j, 0.5);
        assert!(!counter_wrap);
    }

    #[test]
    fn powercap_delta_with_wrap() {
        let (delta_j, counter_wrap) = compute_delta_j(9.0, 1.0, 10.0);

        assert_eq!(delta_j, 2.0);
        assert!(counter_wrap);
    }

    #[test]
    fn powercap_microjoules_to_joules() {
        assert_eq!(microjoules_to_joules(1_500_000), 1.5);
    }

    #[test]
    fn powercap_discovers_fake_sources() {
        let temp = TempDir::new().expect("tempdir");
        write_source(
            temp.path(),
            "intel-rapl/intel-rapl:0",
            "package-0",
            1_000_000,
            262_143_000_000,
        );
        write_source(
            temp.path(),
            "intel-rapl/intel-rapl:0/intel-rapl:0:0",
            "core",
            500_000,
            262_143_000_000,
        );

        let sources = discover_powercap_sources(temp.path()).expect("discover sources");

        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].source_id, 1);
        assert_eq!(sources[0].name, "rapl:package-0");
        assert_eq!(sources[1].source_id, 2);
        assert_eq!(sources[1].name, "rapl:core");
        assert_eq!(sources[1].read_energy_j().expect("read energy"), 0.5);
    }

    #[test]
    fn powercap_ignores_incomplete_source_dirs() {
        let temp = TempDir::new().expect("tempdir");
        let incomplete = temp.path().join("intel-rapl/intel-rapl:0");
        fs::create_dir_all(&incomplete).expect("create incomplete dir");
        fs::write(incomplete.join("name"), "package-0").expect("write name");
        fs::write(incomplete.join("energy_uj"), "1000000").expect("write energy");

        let sources = discover_powercap_sources(temp.path()).expect("discover sources");

        assert!(sources.is_empty());
    }
}
