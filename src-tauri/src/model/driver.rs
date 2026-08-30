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

impl ChannelLayout {
    /// The layout's own name for `channel`, or `None` when this driver has no
    /// such channel.
    ///
    /// THE authority on which channels a driver actually has. Deliberately
    /// derived from the layout and never from a hardcoded index: Flamedriver's
    /// sixth FM slot is the DAC (audit F31), but that is a fact about *that*
    /// profile, and a check written against the literal 5 would silently stop
    /// being true for the next profile registered in
    /// `driver::default_registry()`.
    ///
    /// The `Psg(n)` / `PsgNoise` split matches the convention the rest of the
    /// app already uses (`ProjectManager::default_lane_name`): a numbered PSG
    /// lane binds to a non-noise entry, and `PsgNoise` binds to whichever
    /// entry is flagged `is_noise`.
    pub fn channel_name(&self, channel: &super::song::ChannelAssignment) -> Option<&str> {
        use super::song::ChannelAssignment;
        match channel {
            ChannelAssignment::Fm(n) => self
                .fm_channels.iter().find(|c| c.index == *n).map(|c| c.name.as_str()),
            ChannelAssignment::Psg(n) => self
                .psg_channels.iter().find(|c| !c.is_noise && c.index == *n).map(|c| c.name.as_str()),
            ChannelAssignment::PsgNoise => self
                .psg_channels.iter().find(|c| c.is_noise).map(|c| c.name.as_str()),
            ChannelAssignment::Dac(n) => self
                .dac_channels.iter().find(|c| c.index == *n).map(|c| c.name.as_str()),
        }
    }

    /// Every channel this driver offers, named as the layout names them, in
    /// SMPS header order (DAC, then FM, then PSG). For telling an author what
    /// they *can* use when they have used something the driver lacks.
    pub fn channel_names(&self) -> Vec<&str> {
        self.dac_channels.iter().map(|c| c.name.as_str())
            .chain(self.fm_channels.iter().map(|c| c.name.as_str()))
            .chain(self.psg_channels.iter().map(|c| c.name.as_str()))
            .collect()
    }
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

#[cfg(test)]
mod tests {
    use crate::model::song::ChannelAssignment;

    /// `channel_name`'s `PsgNoise` arm must resolve to the entry the layout
    /// FLAGS as noise -- not merely to some PSG entry that happens to sort
    /// first.
    ///
    /// Audit F40, a vacuous-coverage finding: inverting that one arm
    /// (`find(|c| c.is_noise)` -> `find(|c| !c.is_noise)`), so noise resolved
    /// to a TONE channel, left the ENTIRE suite green. The FM side was covered
    /// by `channel_validity_is_read_from_each_registered_driver_not_a_fixed_index`
    /// (`export/smps.rs`); the noise side was not. Since `channel_name` became
    /// the single authority, that one gap covered both of its call sites --
    /// `validate_for_export`'s channel-existence check and
    /// `ProjectManager::default_lane_name`.
    ///
    /// Every expectation here is READ OUT OF the profile's own layout, never
    /// typed from memory as "PSG Noise": a profile that names its noise
    /// channel something else is still checked, and a profile registered later
    /// is covered without this test being edited (the F34 rule).
    #[test]
    fn psg_noise_resolves_to_the_layout_entry_flagged_noise() {
        let registry = crate::driver::default_registry();
        let mut profiles_checked = 0usize;

        for driver in registry.profiles() {
            let layout = driver.channel_layout();
            let Some(noise) = layout.psg_channels.iter().find(|c| c.is_noise) else {
                continue;
            };
            profiles_checked += 1;

            let resolved = layout
                .channel_name(&ChannelAssignment::PsgNoise)
                .unwrap_or_else(|| {
                    panic!(
                        "driver `{}` advertises a noise channel (`{}`) but PsgNoise resolved \
                         to nothing",
                        driver.id(),
                        noise.name,
                    )
                });

            // Stated as the RELATIONSHIP rather than as a name, so nothing
            // here can be satisfied by a tone entry: this is the assertion
            // that goes red when the arm is inverted.
            assert!(
                !layout.psg_channels.iter().any(|c| !c.is_noise && c.name == resolved),
                "driver `{}` resolved PsgNoise to `{resolved}`, which its own layout flags as \
                 a TONE channel",
                driver.id(),
            );
            assert!(
                layout.psg_channels.iter().any(|c| c.is_noise && c.name == resolved),
                "driver `{}` resolved PsgNoise to `{resolved}`, which is not any entry its own \
                 layout flags as noise",
                driver.id(),
            );
            assert_eq!(
                resolved,
                noise.name.as_str(),
                "driver `{}` flags `{}` as its noise channel, so PsgNoise must bind to it",
                driver.id(),
                noise.name,
            );

            // CONTROL for the numbered arm, and the thing that shows the two
            // arms are separable rather than interchangeable. `Psg(n)` filters
            // to NON-noise entries, so asking it for the noise entry's own
            // index must not hand back the noise channel -- for Flamedriver
            // that index is 3 and the answer is `None`, but the test never
            // says 3.
            assert_ne!(
                layout.channel_name(&ChannelAssignment::Psg(noise.index)),
                Some(noise.name.as_str()),
                "driver `{}`: the numbered PSG arm must not reach the noise entry, but Psg({}) \
                 resolved to `{}`",
                driver.id(),
                noise.index,
                noise.name,
            );

            // CONTROL: the tone side works and names something else. Without
            // this a reader cannot tell whether the noise assertion above is
            // load-bearing or whether both arms simply return the same thing.
            let mut tones_checked = 0usize;
            for tone in layout.psg_channels.iter().filter(|c| !c.is_noise) {
                assert_eq!(
                    layout.channel_name(&ChannelAssignment::Psg(tone.index)),
                    Some(tone.name.as_str()),
                    "driver `{}`: Psg({}) must bind to the non-noise entry of that index",
                    driver.id(),
                    tone.index,
                );
                assert_ne!(
                    tone.name, noise.name,
                    "driver `{}` gives a tone entry and its noise entry the same name, so this \
                     test cannot tell the two arms apart",
                    driver.id(),
                );
                tones_checked += 1;
            }
            assert!(
                tones_checked > 0,
                "driver `{}` advertises no numbered PSG channel, so the control checked nothing",
                driver.id(),
            );
        }

        assert!(
            profiles_checked > 0,
            "no driver `default_registry()` registers advertises a noise channel, so this guard \
             checked nothing -- it must fail rather than report green (F32)",
        );
    }
}
