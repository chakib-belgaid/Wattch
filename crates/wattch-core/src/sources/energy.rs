use std::fmt::Debug;

use wattch_proto::wattch::v1::Source;

use crate::errors::Result;

pub type BoxedEnergySource = Box<dyn EnergySource>;

pub trait SourceMetadata {
    fn source_id(&self) -> u32;

    fn source_name(&self) -> &str;

    fn kind(&self) -> &str;

    fn unit(&self) -> &str;

    fn available(&self) -> bool;

    fn to_proto(&self) -> Source {
        Source {
            source_id: self.source_id(),
            name: self.source_name().to_string(),
            kind: self.kind().to_string(),
            unit: self.unit().to_string(),
            available: self.available(),
        }
    }
}

impl<T> SourceMetadata for Box<T>
where
    T: SourceMetadata + ?Sized,
{
    fn source_id(&self) -> u32 {
        (**self).source_id()
    }

    fn source_name(&self) -> &str {
        (**self).source_name()
    }

    fn kind(&self) -> &str {
        (**self).kind()
    }

    fn unit(&self) -> &str {
        (**self).unit()
    }

    fn available(&self) -> bool {
        (**self).available()
    }
}

pub trait EnergySource: SourceMetadata + Debug + Send + Sync {
    fn read_energy_j(&self) -> Result<f64>;

    fn max_energy_j(&self) -> f64;
}
