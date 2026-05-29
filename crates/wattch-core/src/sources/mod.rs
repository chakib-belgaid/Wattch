pub mod energy;
pub mod fake;
pub mod powercap;

pub use energy::{BoxedEnergySource, EnergySource, SourceMetadata};
pub use fake::{fake_energy_sources, FakeEnergySource};

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{EnergySource, SourceMetadata};
    use crate::sources::fake::{fake_energy_sources, FakeEnergySource};
    use crate::sources::powercap::PowercapSource;

    fn powercap_source() -> PowercapSource {
        PowercapSource {
            source_id: 7,
            name: "rapl:package-0".to_string(),
            kind: "rapl".to_string(),
            unit: "joule".to_string(),
            available: true,
            path: PathBuf::from("/fake"),
            max_energy_j: 262_143.0,
        }
    }

    #[test]
    fn powercap_source_exposes_energy_source_interface() {
        let source = powercap_source();

        assert_eq!(SourceMetadata::source_id(&source), 7);
        assert_eq!(SourceMetadata::source_name(&source), "rapl:package-0");
        assert_eq!(SourceMetadata::kind(&source), "rapl");
        assert_eq!(SourceMetadata::unit(&source), "joule");
        assert!(SourceMetadata::available(&source));
        assert_eq!(EnergySource::max_energy_j(&source), 262_143.0);

        let proto = SourceMetadata::to_proto(&source);
        assert_eq!(proto.source_id, 7);
        assert_eq!(proto.name, "rapl:package-0");
        assert_eq!(proto.kind, "rapl");
        assert_eq!(proto.unit, "joule");
        assert!(proto.available);
    }

    #[test]
    fn fake_energy_source_returns_deterministic_energy_steps() {
        let source = FakeEnergySource::new(1, "fake:deterministic", 100.0, 5.0, 1_000_000.0);

        assert_eq!(source.read_energy_j().expect("first reading"), 100.0);
        assert_eq!(source.read_energy_j().expect("second reading"), 105.0);
        assert_eq!(source.read_energy_j().expect("third reading"), 110.0);
        assert_eq!(EnergySource::max_energy_j(&source), 1_000_000.0);
    }

    #[test]
    fn fake_energy_sources_expose_one_available_source() {
        let sources = fake_energy_sources();

        assert_eq!(sources.len(), 1);
        assert_eq!(SourceMetadata::source_id(&sources[0]), 1);
        assert_eq!(
            SourceMetadata::source_name(&sources[0]),
            "fake:deterministic"
        );
        assert_eq!(SourceMetadata::kind(&sources[0]), "fake");
        assert_eq!(SourceMetadata::unit(&sources[0]), "joule");
        assert!(SourceMetadata::available(&sources[0]));
    }
}
