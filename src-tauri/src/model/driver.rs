use std::collections::HashMap;
use serde::Serialize;
use super::instrument::{DacInstrument, FmInstrument, PsgInstrument};

pub trait DriverProfile: Send + Sync {
    fn name(&self) -> &str;
    fn id(&self) -> &str;
    fn channel_layout(&self) -> ChannelLayout;
    fn supports_feature(&self, feature: DriverFeature) -> bool;
    fn validate_fm(&self, inst: &FmInstrument) -> Result<(), Vec<String>>;
    fn validate_psg(&self, inst: &PsgInstrument) -> Result<(), Vec<String>>;
    fn validate_dac(&self, inst: &DacInstrument) -> Result<(), Vec<String>>;
    fn fm_to_bytes(&self, inst: &FmInstrument) -> Vec<u8>;
    fn fm_from_bytes(&self, bytes: &[u8]) -> Result<FmInstrument, String>;
    fn import_formats(&self) -> Vec<&str>;
    fn export_formats(&self) -> Vec<&str>;
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelLayout {
    pub fm_channels: Vec<FmChannelInfo>,
    pub psg_channels: Vec<PsgChannelInfo>,
    pub dac_channels: Vec<DacChannelInfo>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FmChannelInfo {
    pub index: u8,
    pub name: String,
    pub supports_special_mode: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PsgChannelInfo {
    pub index: u8,
    pub name: String,
    pub is_noise: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DacChannelInfo {
    pub index: u8,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriverFeature {
    SsgEg,
    Fm3SpecialMode,
    MultiDac,
    Dpcm,
    PseudoStereo,
}

use serde::Deserialize;

pub struct DriverRegistry {
    drivers: HashMap<String, Box<dyn DriverProfile>>,
}

impl DriverRegistry {
    pub fn new() -> Self {
        Self {
            drivers: HashMap::new(),
        }
    }

    pub fn register(&mut self, driver: Box<dyn DriverProfile>) {
        self.drivers.insert(driver.id().to_string(), driver);
    }

    pub fn get(&self, id: &str) -> Option<&dyn DriverProfile> {
        self.drivers.get(id).map(|d| d.as_ref())
    }

    pub fn list(&self) -> Vec<(&str, &str)> {
        self.drivers.values().map(|d| (d.id(), d.name())).collect()
    }
}
