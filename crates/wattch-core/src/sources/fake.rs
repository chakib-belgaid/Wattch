use std::sync::atomic::{AtomicU64, Ordering};

use crate::errors::Result;
use crate::sources::energy::{BoxedEnergySource, EnergySource, SourceMetadata};

#[derive(Debug)]
pub struct FakeEnergySource {
    source_id: u32,
    name: String,
    initial_energy_j: f64,
    step_j: f64,
    max_energy_j: f64,
    reading_index: AtomicU64,
}

impl FakeEnergySource {
    pub fn new(
        source_id: u32,
        name: impl Into<String>,
        initial_energy_j: f64,
        step_j: f64,
        max_energy_j: f64,
    ) -> Self {
        Self {
            source_id,
            name: name.into(),
            initial_energy_j,
            step_j,
            max_energy_j,
            reading_index: AtomicU64::new(0),
        }
    }
}

impl SourceMetadata for FakeEnergySource {
    fn source_id(&self) -> u32 {
        self.source_id
    }

    fn source_name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> &str {
        "fake"
    }

    fn unit(&self) -> &str {
        "joule"
    }

    fn available(&self) -> bool {
        true
    }
}

impl EnergySource for FakeEnergySource {
    fn read_energy_j(&self) -> Result<f64> {
        let index = self.reading_index.fetch_add(1, Ordering::Relaxed);
        Ok(self.initial_energy_j + self.step_j * index as f64)
    }

    fn max_energy_j(&self) -> f64 {
        self.max_energy_j
    }
}

pub fn fake_energy_sources() -> Vec<BoxedEnergySource> {
    vec![Box::new(FakeEnergySource::new(
        1,
        "fake:deterministic",
        100.0,
        5.0,
        1_000_000.0,
    ))]
}
