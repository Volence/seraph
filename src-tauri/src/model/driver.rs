use std::collections::HashMap;
use std::path::Path;
use serde::Serialize;
use super::instrument::{DacInstrument, FmInstrument, InstrumentBank, PsgInstrument};
use super::song::Song;
use crate::export::{ExportResult, ExportError};

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
    /// `project_dir` is the open project's directory, needed to resolve DAC
    /// sample files: `DacInstrument::pcm_file` is a BARE FILENAME that lives
    /// at `<project_dir>/instruments/dac/<pcm_file>`. `None` when no project
    /// is saved on disk, which a driver that needs samples must report rather
    /// than skip (audit F33).
    fn export_song(
        &self,
        song: &Song,
        instruments: &InstrumentBank,
        output_dir: &Path,
        project_dir: Option<&Path>,
    ) -> Result<ExportResult, Vec<ExportError>>;
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ChannelLayout {
    pub fm_channels: Vec<FmChannelInfo>,
    pub psg_channels: Vec<PsgChannelInfo>,
    pub dac_channels: Vec<DacChannelInfo>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FmChannelInfo {
    pub index: u8,
    pub name: String,
    pub supports_special_mode: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PsgChannelInfo {
    pub index: u8,
    pub name: String,
    pub is_noise: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct DacChannelInfo {
    pub index: u8,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
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

    /// Every registered profile. Exists so an invariant can be asserted over
    /// the drivers the app ACTUALLY registers rather than over a list a test
    /// re-types by hand, which cannot notice a driver added later (audit F34).
    pub fn profiles(&self) -> impl Iterator<Item = &dyn DriverProfile> {
        self.drivers.values().map(|d| d.as_ref())
    }
}
