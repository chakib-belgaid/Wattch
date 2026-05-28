pub mod energy;
pub mod powercap;

pub use energy::{BoxedEnergySource, EnergySource, SourceMetadata};

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{EnergySource, SourceMetadata};
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
}
