use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

use crate::model::driver::DriverRegistry;
use crate::model::instrument::*;
use crate::model::song::*;
use crate::sequencer::{
    SequencerSnapshot, ChannelSequence, ChannelType, SequencerEvent, InstrumentData, OverlapWarning,
    ModulationParams,
};

/// Maximum song-edit undo steps retained; the oldest snapshot is dropped
/// once the stack exceeds this.
pub const MAX_UNDO_DEPTH: usize = 100;

pub struct ProjectManager {
    project_path: Option<PathBuf>,
    metadata: Option<SongMetadata>,
    tracks: Vec<Track>,
    instruments: InstrumentBank,
    dirty_instruments: HashSet<Uuid>,
    dac_pcm_cache: HashMap<Uuid, Arc<Vec<u8>>>,
    driver_registry: DriverRegistry,
    /// Snapshots of `tracks` taken BEFORE each in-scope song edit
    /// (notes, regions, tracks). Instrument-parameter edits, library
    /// operations, and project create/open are out of undo scope.
    undo_stack: Vec<Vec<Track>>,
    redo_stack: Vec<Vec<Track>>,
    /// True between `begin_undo_group` / `end_undo_group`: only the first
    /// mutation of the group pushes a snapshot (drag-gesture coalescing).
    in_undo_group: bool,
    group_pushed: bool,
    /// Unsaved-changes flag: set by EVERY mutation (in-scope and
    /// out-of-scope-for-undo alike — dirty is about saving, not undo),
    /// cleared by save/create/open/close. Undo/redo do NOT clear it.
    dirty: bool,
}

impl ProjectManager {
    pub fn new(driver_registry: DriverRegistry) -> Self {
        Self {
            project_path: None,
            metadata: None,
            tracks: Vec::new(),
            instruments: InstrumentBank::default(),
            dirty_instruments: HashSet::new(),
            dac_pcm_cache: HashMap::new(),
            driver_registry,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            in_undo_group: false,
            group_pushed: false,
            dirty: false,
        }
    }

    // --- Undo / Redo (song edits only: notes, regions, tracks) ---

    /// Record an in-scope song edit: mark dirty and push an undo snapshot of
    /// the CURRENT tracks state (call after validation, before mutating).
    /// Inside an undo group only the first mutation pushes (gesture
    /// coalescing); every new snapshot invalidates the redo branch.
    fn record_song_edit(&mut self) {
        self.dirty = true;
        if self.in_undo_group {
            if self.group_pushed {
                return;
            }
            self.group_pushed = true;
        }
        self.undo_stack.push(self.tracks.clone());
        if self.undo_stack.len() > MAX_UNDO_DEPTH {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// Open a coalescing group (drag gesture / batch loop): until
    /// `end_undo_group`, only the first mutation pushes a snapshot.
    /// A nested begin is ignored with a warning.
    pub fn begin_undo_group(&mut self) {
        if self.in_undo_group {
            eprintln!("[undo] warning: begin_undo_group while a group is already open; ignoring nested begin");
            return;
        }
        self.in_undo_group = true;
        self.group_pushed = false;
    }

    /// Close the current coalescing group. An end without an open group is
    /// a no-op.
    pub fn end_undo_group(&mut self) {
        self.in_undo_group = false;
        self.group_pushed = false;
    }

    /// Restore the previous song-edit state. Returns the (possibly
    /// unchanged, when there is nothing to undo) current tracks.
    /// Undo marks dirty — the in-memory state now differs from disk.
    pub fn undo(&mut self) -> Vec<Track> {
        if let Some(prev) = self.undo_stack.pop() {
            let current = std::mem::replace(&mut self.tracks, prev);
            self.redo_stack.push(current);
            self.dirty = true;
        }
        self.tracks.clone()
    }

    pub fn redo(&mut self) -> Vec<Track> {
        if let Some(next) = self.redo_stack.pop() {
            let current = std::mem::replace(&mut self.tracks, next);
            self.undo_stack.push(current);
            self.dirty = true;
        }
        self.tracks.clone()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Reset undo history and the dirty flag — project boundary
    /// (create/open/close).
    fn reset_edit_state(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.in_undo_group = false;
        self.group_pushed = false;
        self.dirty = false;
    }

    pub fn create(
        &mut self,
        path: &Path,
        name: &str,
        driver_id: &str,
        tempo: f64,
        time_sig: (u8, u8),
    ) -> Result<(), String> {
        let layout = self
            .driver_registry
            .get(driver_id)
            .ok_or_else(|| format!("unknown driver: {driver_id}"))?
            .channel_layout();

        fs::create_dir_all(path).map_err(|e| e.to_string())?;
        fs::create_dir_all(path.join("instruments/fm")).map_err(|e| e.to_string())?;
        fs::create_dir_all(path.join("instruments/psg")).map_err(|e| e.to_string())?;
        fs::create_dir_all(path.join("instruments/dac")).map_err(|e| e.to_string())?;
        fs::create_dir_all(path.join("exports")).map_err(|e| e.to_string())?;

        let version = serde_json::json!({ "version": "0.1.0" });
        fs::write(
            path.join(".seraph"),
            serde_json::to_string_pretty(&version).unwrap(),
        )
        .map_err(|e| e.to_string())?;

        let metadata = SongMetadata {
            name: name.to_string(),
            tempo,
            time_signature: time_sig,
            ticks_per_beat: 480,
            driver_id: driver_id.to_string(),
        };

        // Seed one instrument-less track per driver channel so a fresh
        // project opens with the full lane roster ready to compose into.
        let tracks = Self::default_tracks_for_layout(&layout);

        let project_file = ProjectFile {
            metadata: metadata.clone(),
            tracks: tracks.clone(),
        };
        let json = serde_json::to_string_pretty(&project_file).map_err(|e| e.to_string())?;
        fs::write(path.join("project.json"), json).map_err(|e| e.to_string())?;

        self.project_path = Some(path.to_path_buf());
        self.metadata = Some(metadata);
        self.tracks = tracks;
        self.instruments = InstrumentBank::default();
        self.dirty_instruments.clear();
        self.dac_pcm_cache.clear();
        self.reset_edit_state();

        Ok(())
    }

    /// One instrument-less track per driver channel, named after the channel,
    /// in FM → PSG → DAC order. Derived from the driver's `ChannelLayout` so
    /// the roster follows whatever driver the project was created with.
    fn default_tracks_for_layout(layout: &crate::model::driver::ChannelLayout) -> Vec<Track> {
        fn lane(name: &str, channel: ChannelAssignment) -> Track {
            Track {
                id: Uuid::new_v4(),
                name: name.to_string(),
                channel,
                instrument_id: None,
                regions: Vec::new(),
                muted: false,
                solo: false,
                volume: 100,
                pan: Pan::Center,
                pitch_offset: 0,
                modulation: None,
            }
        }

        let mut tracks = Vec::new();
        for ch in &layout.fm_channels {
            tracks.push(lane(&ch.name, ChannelAssignment::Fm(ch.index)));
        }
        for ch in &layout.psg_channels {
            let assign = if ch.is_noise {
                ChannelAssignment::PsgNoise
            } else {
                ChannelAssignment::Psg(ch.index)
            };
            tracks.push(lane(&ch.name, assign));
        }
        for ch in &layout.dac_channels {
            tracks.push(lane(&ch.name, ChannelAssignment::Dac(ch.index)));
        }
        tracks
    }

    pub fn open(&mut self, path: &Path) -> Result<Song, String> {
        // `.seraph` is the current marker; `.megadaw` is accepted for legacy projects.
        if !path.join(".seraph").exists() && !path.join(".megadaw").exists() {
            return Err("not a Seraph project (no .seraph file)".into());
        }

        let json = fs::read_to_string(path.join("project.json")).map_err(|e| e.to_string())?;
        let project_file: ProjectFile =
            serde_json::from_str(&json).map_err(|e| e.to_string())?;

        let mut instruments = InstrumentBank::default();

        let fm_dir = path.join("instruments/fm");
        if fm_dir.exists() {
            for entry in fs::read_dir(&fm_dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.path().extension().map_or(false, |ext| ext == "json") {
                    let data = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
                    let inst: FmInstrument =
                        serde_json::from_str(&data).map_err(|e| e.to_string())?;
                    instruments.fm.push(inst);
                }
            }
        }

        let psg_dir = path.join("instruments/psg");
        if psg_dir.exists() {
            for entry in fs::read_dir(&psg_dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.path().extension().map_or(false, |ext| ext == "json") {
                    let data = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
                    let inst: PsgInstrument =
                        serde_json::from_str(&data).map_err(|e| e.to_string())?;
                    instruments.psg.push(inst);
                }
            }
        }

        let dac_dir = path.join("instruments/dac");
        if dac_dir.exists() {
            for entry in fs::read_dir(&dac_dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.path().extension().map_or(false, |ext| ext == "json") {
                    let data = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
                    let inst: DacInstrument =
                        serde_json::from_str(&data).map_err(|e| e.to_string())?;
                    let pcm_path = path.join("instruments/dac").join(&inst.pcm_file);
                    if !inst.pcm_file.is_empty() && pcm_path.exists() {
                        let pcm_data = fs::read(&pcm_path).map_err(|e| e.to_string())?;
                        self.dac_pcm_cache.insert(inst.id, Arc::new(pcm_data));
                    }
                    instruments.dac.push(inst);
                }
            }
        }

        self.project_path = Some(path.to_path_buf());
        self.metadata = Some(project_file.metadata.clone());
        self.tracks = project_file.tracks.clone();
        self.instruments = instruments.clone();
        self.dirty_instruments.clear();
        self.reset_edit_state();

        Ok(Song {
            metadata: project_file.metadata,
            tracks: project_file.tracks,
            instruments,
        })
    }

    pub fn save(&mut self) -> Result<(), String> {
        let path = self.project_path.as_ref().ok_or("no project open")?;
        let metadata = self.metadata.as_ref().ok_or("no project open")?.clone();

        let project_file = ProjectFile {
            metadata,
            tracks: self.tracks.clone(),
        };
        let json = serde_json::to_string_pretty(&project_file).map_err(|e| e.to_string())?;
        fs::write(path.join("project.json"), json).map_err(|e| e.to_string())?;

        for inst in &self.instruments.fm {
            if self.dirty_instruments.contains(&inst.id) {
                let json = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
                fs::write(path.join(format!("instruments/fm/{}.json", inst.id)), json)
                    .map_err(|e| e.to_string())?;
            }
        }
        for inst in &self.instruments.psg {
            if self.dirty_instruments.contains(&inst.id) {
                let json = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
                fs::write(path.join(format!("instruments/psg/{}.json", inst.id)), json)
                    .map_err(|e| e.to_string())?;
            }
        }
        for inst in &self.instruments.dac {
            if self.dirty_instruments.contains(&inst.id) {
                let json = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
                fs::write(path.join(format!("instruments/dac/{}.json", inst.id)), json)
                    .map_err(|e| e.to_string())?;
                if let Some(pcm) = self.dac_pcm_cache.get(&inst.id) {
                    fs::write(path.join("instruments/dac").join(&inst.pcm_file), pcm.as_ref())
                        .map_err(|e| e.to_string())?;
                }
            }
        }

        self.dirty_instruments.clear();
        self.dirty = false;
        Ok(())
    }

    pub fn close(&mut self) {
        self.project_path = None;
        self.metadata = None;
        self.tracks.clear();
        self.instruments = InstrumentBank::default();
        self.dirty_instruments.clear();
        self.dac_pcm_cache.clear();
        self.reset_edit_state();
    }

    pub fn is_open(&self) -> bool {
        self.project_path.is_some()
    }

    pub fn metadata(&self) -> Option<&SongMetadata> {
        self.metadata.as_ref()
    }

    pub fn project_path(&self) -> Option<&Path> {
        self.project_path.as_deref()
    }

    pub fn driver_registry(&self) -> &DriverRegistry {
        &self.driver_registry
    }

    pub fn song(&self) -> Option<Song> {
        self.metadata.as_ref().map(|meta| Song {
            metadata: meta.clone(),
            tracks: self.tracks.clone(),
            instruments: self.instruments.clone(),
        })
    }

    // --- FM CRUD ---

    /// Bind instrument `id` to the first empty lane whose channel `rank`s
    /// (lowest rank wins), renaming the lane to the instrument — the
    /// convention everywhere is that a track's name follows its bound
    /// instrument. Returns false when no empty lane of that kind exists.
    fn bind_to_empty_lane(
        &mut self,
        id: Uuid,
        name: &str,
        rank: impl Fn(&ChannelAssignment) -> Option<u8>,
    ) -> bool {
        let target = self
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.instrument_id.is_none())
            .filter_map(|(i, t)| rank(&t.channel).map(|r| (r, i)))
            .min()
            .map(|(_, i)| i);
        match target {
            Some(i) => {
                self.tracks[i].instrument_id = Some(id);
                self.tracks[i].name = name.to_string();
                true
            }
            None => false,
        }
    }

    /// Clear `instrument_id` on any track bound to `id`. Lanes survive
    /// instrument deletion (the seeded roster is the channel layout).
    fn unbind_instrument_from_tracks(&mut self, id: Uuid) {
        for track in self.tracks.iter_mut().filter(|t| t.instrument_id == Some(id)) {
            track.instrument_id = None;
        }
    }

    pub fn add_fm_instrument(&mut self, mut inst: FmInstrument) -> Uuid {
        let id = Uuid::new_v4();
        inst.id = id;
        let track_name = inst.name.clone();
        self.instruments.fm.push(inst);
        self.dirty_instruments.insert(id);
        self.dirty = true;
        let bound = self.bind_to_empty_lane(id, &track_name, |c| match c {
            ChannelAssignment::Fm(n) => Some(*n),
            _ => None,
        });
        if !bound {
            let channel = self.next_available_fm_channel();
            // raw push: instrument operations are out of undo scope
            self.push_track_raw(track_name, channel, Some(id));
        }
        id
    }

    fn next_available_fm_channel(&self) -> ChannelAssignment {
        let used: HashSet<u8> = self.tracks.iter().filter_map(|t| {
            if let ChannelAssignment::Fm(n) = t.channel { Some(n) } else { None }
        }).collect();
        for i in 0..6u8 {
            if !used.contains(&i) { return ChannelAssignment::Fm(i); }
        }
        ChannelAssignment::Fm(0)
    }

    pub fn update_fm_instrument(&mut self, id: Uuid, mut inst: FmInstrument) -> Result<(), String> {
        let existing = self.instruments.fm.iter_mut().find(|i| i.id == id)
            .ok_or("FM instrument not found")?;
        inst.id = id;
        let new_name = inst.name.clone();
        *existing = inst;
        self.dirty_instruments.insert(id);
        self.dirty = true;
        if let Some(track) = self.tracks.iter_mut().find(|t| t.instrument_id == Some(id)) {
            track.name = new_name;
        }
        Ok(())
    }

    pub fn delete_fm_instrument(&mut self, id: Uuid) -> Result<(), String> {
        let pos = self.instruments.fm.iter().position(|i| i.id == id)
            .ok_or("FM instrument not found")?;
        self.instruments.fm.remove(pos);
        self.dirty_instruments.remove(&id);
        self.dirty = true;
        self.unbind_instrument_from_tracks(id);
        if let Some(path) = &self.project_path {
            let file = path.join(format!("instruments/fm/{id}.json"));
            if file.exists() { let _ = fs::remove_file(file); }
        }
        Ok(())
    }

    pub fn list_fm_instruments(&self) -> &[FmInstrument] {
        &self.instruments.fm
    }

    pub fn get_fm_instrument(&self, id: &Uuid) -> Option<&FmInstrument> {
        self.instruments.fm.iter().find(|i| &i.id == id)
    }

    // --- PSG CRUD ---

    pub fn add_psg_instrument(&mut self, mut inst: PsgInstrument) -> Uuid {
        let id = Uuid::new_v4();
        inst.id = id;
        let is_noise = inst.noise_mode.is_some();
        let track_name = inst.name.clone();
        let fallback_channel = self.next_available_psg_channel(&inst);
        self.instruments.psg.push(inst);
        self.dirty_instruments.insert(id);
        self.dirty = true;
        let bound = self.bind_to_empty_lane(id, &track_name, |c| match c {
            ChannelAssignment::PsgNoise if is_noise => Some(0),
            ChannelAssignment::Psg(n) if !is_noise => Some(*n),
            _ => None,
        });
        if !bound {
            self.push_track_raw(track_name, fallback_channel, Some(id));
        }
        id
    }

    fn next_available_psg_channel(&self, inst: &PsgInstrument) -> ChannelAssignment {
        if inst.noise_mode.is_some() {
            return ChannelAssignment::PsgNoise;
        }
        let used: HashSet<u8> = self.tracks.iter().filter_map(|t| {
            if let ChannelAssignment::Psg(n) = t.channel { Some(n) } else { None }
        }).collect();
        for i in 0..3u8 {
            if !used.contains(&i) { return ChannelAssignment::Psg(i); }
        }
        ChannelAssignment::Psg(0)
    }

    pub fn update_psg_instrument(&mut self, id: Uuid, mut inst: PsgInstrument) -> Result<(), String> {
        let existing = self.instruments.psg.iter_mut().find(|i| i.id == id)
            .ok_or("PSG instrument not found")?;
        inst.id = id;
        let new_name = inst.name.clone();
        *existing = inst;
        self.dirty_instruments.insert(id);
        self.dirty = true;
        if let Some(track) = self.tracks.iter_mut().find(|t| t.instrument_id == Some(id)) {
            track.name = new_name;
        }
        Ok(())
    }

    pub fn delete_psg_instrument(&mut self, id: Uuid) -> Result<(), String> {
        let pos = self.instruments.psg.iter().position(|i| i.id == id)
            .ok_or("PSG instrument not found")?;
        self.instruments.psg.remove(pos);
        self.dirty_instruments.remove(&id);
        self.dirty = true;
        self.unbind_instrument_from_tracks(id);
        if let Some(path) = &self.project_path {
            let file = path.join(format!("instruments/psg/{id}.json"));
            if file.exists() { let _ = fs::remove_file(file); }
        }
        Ok(())
    }

    pub fn list_psg_instruments(&self) -> &[PsgInstrument] {
        &self.instruments.psg
    }

    pub fn get_psg_instrument(&self, id: &Uuid) -> Option<&PsgInstrument> {
        self.instruments.psg.iter().find(|i| &i.id == id)
    }

    // --- Library assignment ---

    /// Bind a library voice to an existing track (drag-to-track swap).
    ///
    /// If a project instrument of the same kind already has `hash` as its
    /// content hash it is REUSED; otherwise the library instrument is added
    /// to the bank (with a fresh id, WITHOUT creating a new track — unlike
    /// `add_fm_instrument`/`add_psg_instrument`). The track is then pointed
    /// at the instrument and renamed to it, matching the update-path
    /// convention that a track's name follows its bound instrument.
    ///
    /// All per-region and per-note `instrument_id` overrides on the track
    /// are CLEARED: importers stamp an id on every note and `build_snapshot`
    /// gives those precedence over the track binding, so without the clear a
    /// swap would be inaudible. Chosen semantic: "swap this track's
    /// instrument" flattens any mid-track voice changes to the dropped voice.
    ///
    /// Content hashes cover only sound fields (`fm_canonical_bytes` /
    /// `psg_canonical_bytes` exclude id/name/metadata), so project
    /// instruments can be hashed via a straight clone.
    pub fn assign_library_instrument_to_track(
        &mut self,
        track_id: Uuid,
        inst: &crate::library::entry::LibraryInstrument,
        hash: &str,
    ) -> Result<Uuid, String> {
        use crate::library::entry::{content_hash, LibraryInstrument};

        let track = self.tracks.iter().find(|t| t.id == track_id)
            .ok_or("track not found")?;
        match (inst, &track.channel) {
            (LibraryInstrument::Fm(_), ChannelAssignment::Fm(_)) => {}
            (LibraryInstrument::Psg(_), ChannelAssignment::Psg(_) | ChannelAssignment::PsgNoise) => {}
            (LibraryInstrument::Fm(_), _) => {
                return Err("cannot assign an FM voice to a non-FM track".into());
            }
            (LibraryInstrument::Psg(_), _) => {
                return Err("cannot assign a PSG voice to a non-PSG track".into());
            }
        }

        // The TRACK binding change is an undoable song edit (the instrument
        // added to the bank below, if any, stays — instrument operations are
        // out of undo scope; undo only restores the binding).
        self.record_song_edit();

        let existing = match inst {
            LibraryInstrument::Fm(_) => self.instruments.fm.iter()
                .find(|i| content_hash(&LibraryInstrument::Fm((*i).clone())) == hash)
                .map(|i| (i.id, i.name.clone())),
            LibraryInstrument::Psg(_) => self.instruments.psg.iter()
                .find(|i| content_hash(&LibraryInstrument::Psg((*i).clone())) == hash)
                .map(|i| (i.id, i.name.clone())),
        };

        let (id, name) = match existing {
            Some(found) => found,
            None => {
                let id = Uuid::new_v4();
                let name = match inst {
                    LibraryInstrument::Fm(i) => {
                        let mut i = i.clone();
                        i.id = id;
                        let name = i.name.clone();
                        self.instruments.fm.push(i);
                        name
                    }
                    LibraryInstrument::Psg(i) => {
                        let mut i = i.clone();
                        i.id = id;
                        let name = i.name.clone();
                        self.instruments.psg.push(i);
                        name
                    }
                };
                self.dirty_instruments.insert(id);
                (id, name)
            }
        };

        let track = self.tracks.iter_mut().find(|t| t.id == track_id)
            .expect("track existence checked above");
        track.instrument_id = Some(id);
        track.name = name;
        // A track-level swap must win over stale per-note/per-region bindings
        // (importers stamp instrument_id on every note; build_snapshot gives
        // those precedence over the track binding).
        for region in &mut track.regions {
            region.instrument_id = None;
            for note in &mut region.notes {
                note.instrument_id = None;
            }
        }
        Ok(id)
    }

    // --- DAC CRUD ---

    pub fn add_dac_instrument(&mut self, inst: DacInstrument, pcm_data: Vec<u8>) -> Uuid {
        let id = inst.id;
        let track_name = inst.name.clone();
        self.dac_pcm_cache.insert(id, Arc::new(pcm_data));
        self.instruments.dac.push(inst);
        self.dirty_instruments.insert(id);
        self.dirty = true;
        let bound = self.bind_to_empty_lane(id, &track_name, |c| match c {
            ChannelAssignment::Dac(n) => Some(*n),
            _ => None,
        });
        if !bound {
            self.push_track_raw(track_name, ChannelAssignment::Dac(0), Some(id));
        }
        id
    }

    pub fn update_dac_instrument(&mut self, id: Uuid, mut inst: DacInstrument) -> Result<(), String> {
        let existing = self.instruments.dac.iter_mut().find(|i| i.id == id)
            .ok_or("DAC instrument not found")?;
        inst.id = id;
        let new_name = inst.name.clone();
        *existing = inst;
        self.dirty_instruments.insert(id);
        self.dirty = true;
        if let Some(track) = self.tracks.iter_mut().find(|t| t.instrument_id == Some(id)) {
            track.name = new_name;
        }
        Ok(())
    }

    pub fn delete_dac_instrument(&mut self, id: Uuid) -> Result<(), String> {
        let pos = self.instruments.dac.iter().position(|i| i.id == id)
            .ok_or("DAC instrument not found")?;
        let inst = self.instruments.dac.remove(pos);
        self.dirty_instruments.remove(&id);
        self.dirty = true;
        self.dac_pcm_cache.remove(&id);
        self.unbind_instrument_from_tracks(id);
        if let Some(path) = &self.project_path {
            for name in [
                format!("instruments/dac/{id}.json"),
                format!("instruments/dac/{}", inst.pcm_file),
                format!("instruments/dac/{}", inst.original_file),
            ] {
                let p = path.join(&name);
                if p.exists() { let _ = fs::remove_file(p); }
            }
        }
        Ok(())
    }

    pub fn list_dac_instruments(&self) -> &[DacInstrument] {
        &self.instruments.dac
    }

    pub fn get_dac_instrument(&self, id: &Uuid) -> Option<&DacInstrument> {
        self.instruments.dac.iter().find(|i| &i.id == id)
    }

    pub fn get_dac_pcm(&self, id: &Uuid) -> Option<Arc<Vec<u8>>> {
        self.dac_pcm_cache.get(id).cloned()
    }

    pub fn update_dac_pcm(&mut self, id: Uuid, pcm_data: Vec<u8>) {
        self.dac_pcm_cache.insert(id, Arc::new(pcm_data));
        self.dirty_instruments.insert(id);
        self.dirty = true;
    }

    // --- Snapshot Builder ---

    pub fn build_snapshot(&self) -> SequencerSnapshot {
        let metadata = match &self.metadata {
            Some(m) => m,
            None => return SequencerSnapshot::empty(),
        };

        let any_solo = self.tracks.iter().any(|t| t.solo);

        let mut channel_map: BTreeMap<String, Vec<&Track>> = BTreeMap::new();

        for track in &self.tracks {
            if track.muted {
                continue;
            }
            if any_solo && !track.solo {
                continue;
            }
            let key = match &track.channel {
                ChannelAssignment::Fm(n) => format!("fm_{n}"),
                ChannelAssignment::Psg(n) => format!("psg_{n}"),
                ChannelAssignment::PsgNoise => "psg_noise".to_string(),
                ChannelAssignment::Dac(n) => format!("dac_{n}"),
            };
            channel_map.entry(key).or_default().push(track);
        }

        let driver = self.driver_registry.get(metadata.driver_id.as_str());

        let mut channels = Vec::new();
        for (_key, tracks) in &channel_map {
            let channel_type = match &tracks[0].channel {
                ChannelAssignment::Fm(n) => ChannelType::Fm(*n),
                ChannelAssignment::Psg(n) => ChannelType::Psg(*n),
                ChannelAssignment::PsgNoise => ChannelType::PsgNoise,
                ChannelAssignment::Dac(n) => ChannelType::Dac(*n),
            };

            let mut events: Vec<SequencerEvent> = Vec::new();
            let mut overlap_sources: Vec<(u64, u64, String)> = Vec::new();

            for track in tracks {
                let track_inst_data = self.resolve_instrument_data(track, driver);
                let pitch_off = track.pitch_offset as i16;
                for region in &track.regions {
                    let inst_data = if region.instrument_id.is_some() {
                        self.resolve_instrument_data_for_region(region, &track.channel, driver)
                            .or_else(|| track_inst_data.clone())
                    } else {
                        track_inst_data.clone()
                    };
                    for note in &region.notes {
                        let abs_tick = region.start_tick + note.tick;
                        let end_tick = abs_tick + note.duration_ticks;
                        let pitched = if matches!(track.channel, ChannelAssignment::Dac(_)) {
                            note.pitch
                        } else {
                            (note.pitch as i16 + pitch_off).clamp(0, 127) as u8
                        };
                        let note_inst = if let Some(nid) = note.instrument_id {
                            self.resolve_instrument_data_by_id(nid, &track.channel, driver)
                                .or_else(|| inst_data.clone())
                        } else {
                            inst_data.clone()
                        };
                        if let Some(ref data) = note_inst {
                            let note_mod = if let Some(ref m) = note.modulation {
                                Some(ModulationParams { wait: m.wait, speed: m.speed, delta: m.delta, steps: m.steps })
                            } else if let Some(ref m) = track.modulation {
                                Some(ModulationParams { wait: m.wait, speed: m.speed, delta: m.delta, steps: m.steps })
                            } else {
                                None
                            };
                            events.push(SequencerEvent::NoteOn {
                                tick: abs_tick,
                                pitch: pitched,
                                velocity: note.velocity,
                                detune: note.detune,
                                duration_ticks: note.duration_ticks,
                                instrument: data.clone(),
                                modulation: note_mod,
                                pan_override: note.pan_override,
                            });
                        }
                        events.push(SequencerEvent::NoteOff {
                            tick: end_tick,
                            pitch: pitched,
                        });
                        overlap_sources.push((abs_tick, end_tick, track.id.to_string()));
                    }
                }
            }

            // NoteOff before NoteOn at same tick
            events.sort_by(|a, b| {
                let ta = a.tick();
                let tb = b.tick();
                if ta != tb {
                    return ta.cmp(&tb);
                }
                let priority = |e: &SequencerEvent| -> u8 {
                    match e {
                        SequencerEvent::NoteOff { .. } => 0,
                        SequencerEvent::NoteOn { .. } => 1,
                    }
                };
                priority(a).cmp(&priority(b))
            });

            let mut overlaps = Vec::new();
            overlap_sources.sort_by_key(|s| s.0);
            for i in 0..overlap_sources.len() {
                for j in (i + 1)..overlap_sources.len() {
                    if overlap_sources[j].0 >= overlap_sources[i].1 {
                        break;
                    }
                    let ch_name = match &channel_type {
                        ChannelType::Fm(n) => format!("FM{}", n + 1),
                        ChannelType::Psg(n) => format!("PSG{}", n + 1),
                        ChannelType::PsgNoise => "PSG Noise".to_string(),
                        ChannelType::Dac(n) => format!("DAC{}", n + 1),
                    };
                    overlaps.push(OverlapWarning {
                        channel_name: ch_name,
                        tick_start: overlap_sources[j].0,
                        tick_end: overlap_sources[i].1.min(overlap_sources[j].1),
                        track_ids: vec![
                            overlap_sources[i].2.clone(),
                            overlap_sources[j].2.clone(),
                        ],
                    });
                }
            }

            let volume = tracks[0].volume;
            let pan_byte = match tracks[0].pan {
                crate::model::song::Pan::Left => 0x80u8,
                crate::model::song::Pan::Right => 0x40,
                crate::model::song::Pan::Center => 0xC0,
            };
            let modulation = tracks[0].modulation.as_ref().map(|m| {
                crate::sequencer::snapshot::ModulationParams {
                    wait: m.wait, speed: m.speed, delta: m.delta, steps: m.steps,
                }
            });
            let noise_reg = if matches!(channel_type, ChannelType::PsgNoise) {
                if let Some(ref inst_id) = tracks[0].instrument_id {
                    self.instruments.psg.iter()
                        .find(|p| &p.id == inst_id)
                        .and_then(|p| p.noise_mode.as_ref())
                        .map(|nm| match nm {
                            crate::model::instrument::NoiseMode::Periodic(f) => 0xE0 | ((*f as u8) & 0x03),
                            crate::model::instrument::NoiseMode::White(f) => 0xE0 | 0x04 | ((*f as u8) & 0x03),
                        })
                        .unwrap_or(0xE4)
                } else { 0xE4 }
            } else { 0xE4 };
            channels.push(ChannelSequence {
                channel_type,
                volume,
                pan: pan_byte,
                modulation,
                noise_reg,
                events,
                overlaps,
            });
        }

        SequencerSnapshot {
            tempo_bpm: metadata.tempo,
            ticks_per_beat: metadata.ticks_per_beat,
            loop_start: None,
            loop_end: None,
            channels,
        }
    }

    fn resolve_instrument_data_for_region(
        &self,
        region: &Region,
        channel: &ChannelAssignment,
        driver: Option<&dyn crate::model::driver::DriverProfile>,
    ) -> Option<InstrumentData> {
        let inst_id = region.instrument_id.as_ref()?;
        match channel {
            ChannelAssignment::Fm(_) => {
                let inst = self.instruments.fm.iter().find(|i| &i.id == inst_id)?;
                let bytes: [u8; 25] = if let Some(drv) = driver {
                    let vec = drv.fm_to_bytes(inst);
                    let mut arr = [0u8; 25];
                    let len = vec.len().min(25);
                    arr[..len].copy_from_slice(&vec[..len]);
                    arr
                } else {
                    inst.pack_patch()
                };
                let ssg_eg = [
                    inst.operators[0].ssg_eg,
                    inst.operators[1].ssg_eg,
                    inst.operators[2].ssg_eg,
                    inst.operators[3].ssg_eg,
                ];
                Some(InstrumentData::FmPatch { bytes, ssg_eg })
            }
            ChannelAssignment::Psg(_) | ChannelAssignment::PsgNoise => {
                let inst = self.instruments.psg.iter().find(|i| &i.id == inst_id)?;
                Some(InstrumentData::PsgEnvelope {
                    period: 0,
                    envelope: Arc::new(inst.volume_sequence.clone()),
                    loop_point: inst.loop_point,
                    silence_on_end: inst.silence_on_end,
                })
            }
            ChannelAssignment::Dac(_) => {
                let inst = self.instruments.dac.iter().find(|i| &i.id == inst_id)?;
                let pcm = self.dac_pcm_cache.get(inst_id)?;
                Some(InstrumentData::DacSample {
                    samples: pcm.clone(),
                    sample_rate: inst.target_sample_rate,
                })
            }
        }
    }

    fn resolve_instrument_data_by_id(
        &self,
        inst_id: Uuid,
        channel: &ChannelAssignment,
        driver: Option<&dyn crate::model::driver::DriverProfile>,
    ) -> Option<InstrumentData> {
        match channel {
            ChannelAssignment::Fm(_) => {
                let inst = self.instruments.fm.iter().find(|i| i.id == inst_id)?;
                let bytes: [u8; 25] = if let Some(drv) = driver {
                    let vec = drv.fm_to_bytes(inst);
                    let mut arr = [0u8; 25];
                    let len = vec.len().min(25);
                    arr[..len].copy_from_slice(&vec[..len]);
                    arr
                } else {
                    inst.pack_patch()
                };
                let ssg_eg = [
                    inst.operators[0].ssg_eg,
                    inst.operators[1].ssg_eg,
                    inst.operators[2].ssg_eg,
                    inst.operators[3].ssg_eg,
                ];
                Some(InstrumentData::FmPatch { bytes, ssg_eg })
            }
            ChannelAssignment::Psg(_) | ChannelAssignment::PsgNoise => {
                let inst = self.instruments.psg.iter().find(|i| i.id == inst_id)?;
                Some(InstrumentData::PsgEnvelope {
                    period: 0,
                    envelope: Arc::new(inst.volume_sequence.clone()),
                    loop_point: inst.loop_point,
                    silence_on_end: inst.silence_on_end,
                })
            }
            ChannelAssignment::Dac(_) => {
                let inst = self.instruments.dac.iter().find(|i| i.id == inst_id)?;
                let pcm = self.dac_pcm_cache.get(&inst_id)?;
                Some(InstrumentData::DacSample {
                    samples: pcm.clone(),
                    sample_rate: inst.target_sample_rate,
                })
            }
        }
    }

    fn resolve_instrument_data(
        &self,
        track: &Track,
        driver: Option<&dyn crate::model::driver::DriverProfile>,
    ) -> Option<InstrumentData> {
        let inst_id = track.instrument_id.as_ref()?;
        match &track.channel {
            ChannelAssignment::Fm(_) => {
                let inst = self.instruments.fm.iter().find(|i| &i.id == inst_id)?;
                let bytes: [u8; 25] = if let Some(drv) = driver {
                    let vec = drv.fm_to_bytes(inst);
                    let mut arr = [0u8; 25];
                    let len = vec.len().min(25);
                    arr[..len].copy_from_slice(&vec[..len]);
                    arr
                } else {
                    inst.pack_patch()
                };
                let ssg_eg = [
                    inst.operators[0].ssg_eg,
                    inst.operators[1].ssg_eg,
                    inst.operators[2].ssg_eg,
                    inst.operators[3].ssg_eg,
                ];
                Some(InstrumentData::FmPatch { bytes, ssg_eg })
            }
            ChannelAssignment::Psg(_) | ChannelAssignment::PsgNoise => {
                let inst = self.instruments.psg.iter().find(|i| &i.id == inst_id)?;
                Some(InstrumentData::PsgEnvelope {
                    period: 0,
                    envelope: Arc::new(inst.volume_sequence.clone()),
                    loop_point: inst.loop_point,
                    silence_on_end: inst.silence_on_end,
                })
            }
            ChannelAssignment::Dac(_) => {
                let inst = self.instruments.dac.iter().find(|i| &i.id == inst_id)?;
                let pcm = self.dac_pcm_cache.get(inst_id)?;
                Some(InstrumentData::DacSample {
                    samples: pcm.clone(),
                    sample_rate: inst.target_sample_rate,
                })
            }
        }
    }

    // --- Track CRUD ---

    /// Song-edit entry point (undoable). Instrument add paths use
    /// `push_track_raw` instead — instrument operations are out of undo scope.
    pub fn add_track(&mut self, name: String, channel: ChannelAssignment, instrument_id: Option<Uuid>) -> Uuid {
        self.record_song_edit();
        self.push_track_raw(name, channel, instrument_id)
    }

    fn push_track_raw(&mut self, name: String, channel: ChannelAssignment, instrument_id: Option<Uuid>) -> Uuid {
        let id = Uuid::new_v4();
        self.tracks.push(Track {
            id,
            name,
            channel,
            instrument_id,
            regions: Vec::new(),
            muted: false,
            solo: false,
            volume: 100,
            pan: Pan::Center,
            pitch_offset: 0,
            modulation: None,
        });
        id
    }

    pub fn update_track(
        &mut self,
        id: Uuid,
        name: String,
        channel: ChannelAssignment,
        instrument_id: Option<Uuid>,
        muted: bool,
        solo: bool,
        volume: u8,
        pan: Pan,
        pitch_offset: i8,
    ) -> Result<(), String> {
        let idx = self.tracks.iter().position(|t| t.id == id)
            .ok_or("track not found")?;
        self.record_song_edit();
        let track = &mut self.tracks[idx];
        track.name = name;
        track.channel = channel;
        track.instrument_id = instrument_id;
        track.muted = muted;
        track.solo = solo;
        track.volume = volume;
        track.pan = pan;
        track.pitch_offset = pitch_offset;
        Ok(())
    }

    pub fn delete_track(&mut self, id: Uuid) -> Result<(), String> {
        let pos = self.tracks.iter().position(|t| t.id == id)
            .ok_or("track not found")?;
        self.record_song_edit();
        self.tracks.remove(pos);
        Ok(())
    }

    pub fn list_tracks(&self) -> &[Track] {
        &self.tracks
    }

    // --- Region CRUD ---

    pub fn add_region(&mut self, track_id: Uuid, start_tick: u64, duration_ticks: u64) -> Result<Uuid, String> {
        let idx = self.tracks.iter().position(|t| t.id == track_id)
            .ok_or("track not found")?;
        self.record_song_edit();
        let track = &mut self.tracks[idx];
        let id = Uuid::new_v4();
        track.regions.push(Region {
            id,
            start_tick,
            duration_ticks,
            notes: Vec::new(),
            instrument_id: None,
        });
        Ok(id)
    }

    pub fn update_region(&mut self, track_id: Uuid, region_id: Uuid, start_tick: u64, duration_ticks: u64) -> Result<(), String> {
        let t_idx = self.tracks.iter().position(|t| t.id == track_id)
            .ok_or("track not found")?;
        let r_idx = self.tracks[t_idx].regions.iter().position(|r| r.id == region_id)
            .ok_or("region not found")?;
        self.record_song_edit();
        let region = &mut self.tracks[t_idx].regions[r_idx];
        region.start_tick = start_tick;
        region.duration_ticks = duration_ticks;
        Ok(())
    }

    pub fn move_region(&mut self, src_track_id: Uuid, region_id: Uuid, dst_track_id: Uuid, start_tick: u64) -> Result<(), String> {
        // Validate every lookup BEFORE recording/mutating: a missing
        // destination must not lose the region (or push a snapshot).
        let src_idx = self.tracks.iter().position(|t| t.id == src_track_id)
            .ok_or("source track not found")?;
        let pos = self.tracks[src_idx].regions.iter().position(|r| r.id == region_id)
            .ok_or("region not found")?;
        if src_track_id == dst_track_id {
            self.record_song_edit();
            self.tracks[src_idx].regions[pos].start_tick = start_tick;
            return Ok(());
        }
        let dst_idx = self.tracks.iter().position(|t| t.id == dst_track_id)
            .ok_or("destination track not found")?;
        self.record_song_edit();
        let mut region = self.tracks[src_idx].regions.remove(pos);
        region.start_tick = start_tick;
        self.tracks[dst_idx].regions.push(region);
        Ok(())
    }

    /// Deep-clone a region (fresh id, notes copied) onto the SAME track at
    /// `at_start_tick`. Backs both region duplicate (Ctrl+D) and region paste.
    pub fn duplicate_region(&mut self, track_id: Uuid, region_id: Uuid, at_start_tick: u64) -> Result<Uuid, String> {
        // Validate every lookup BEFORE recording/mutating (update_note's
        // pattern): a bad id must not push an undo snapshot.
        let t_idx = self.tracks.iter().position(|t| t.id == track_id)
            .ok_or("track not found")?;
        let r_idx = self.tracks[t_idx].regions.iter().position(|r| r.id == region_id)
            .ok_or("region not found")?;
        self.record_song_edit();
        let mut clone = self.tracks[t_idx].regions[r_idx].clone();
        clone.id = Uuid::new_v4();
        clone.start_tick = at_start_tick;
        let id = clone.id;
        self.tracks[t_idx].regions.push(clone);
        Ok(id)
    }

    pub fn delete_region(&mut self, track_id: Uuid, region_id: Uuid) -> Result<(), String> {
        let t_idx = self.tracks.iter().position(|t| t.id == track_id)
            .ok_or("track not found")?;
        let pos = self.tracks[t_idx].regions.iter().position(|r| r.id == region_id)
            .ok_or("region not found")?;
        self.record_song_edit();
        self.tracks[t_idx].regions.remove(pos);
        Ok(())
    }

    // --- Note CRUD ---

    pub fn add_note(
        &mut self,
        track_id: Uuid,
        region_id: Uuid,
        tick: u64,
        pitch: u8,
        velocity: u8,
        duration_ticks: u64,
    ) -> Result<usize, String> {
        let t_idx = self.tracks.iter().position(|t| t.id == track_id)
            .ok_or("track not found")?;
        let r_idx = self.tracks[t_idx].regions.iter().position(|r| r.id == region_id)
            .ok_or("region not found")?;
        self.record_song_edit();
        let region = &mut self.tracks[t_idx].regions[r_idx];
        let idx = region.notes.len();
        region.notes.push(Note { tick, pitch, velocity, duration_ticks, instrument_id: None, detune: 0, pan_override: None, modulation: None });
        Ok(idx)
    }

    pub fn update_note(
        &mut self,
        track_id: Uuid,
        region_id: Uuid,
        note_index: usize,
        tick: u64,
        pitch: u8,
        velocity: u8,
        duration_ticks: u64,
    ) -> Result<(), String> {
        let t_idx = self.tracks.iter().position(|t| t.id == track_id)
            .ok_or("track not found")?;
        let r_idx = self.tracks[t_idx].regions.iter().position(|r| r.id == region_id)
            .ok_or("region not found")?;
        if note_index >= self.tracks[t_idx].regions[r_idx].notes.len() {
            return Err("note index out of range".into());
        }
        self.record_song_edit();
        let note = &mut self.tracks[t_idx].regions[r_idx].notes[note_index];
        note.tick = tick;
        note.pitch = pitch;
        note.velocity = velocity;
        note.duration_ticks = duration_ticks;
        Ok(())
    }

    pub fn delete_note(
        &mut self,
        track_id: Uuid,
        region_id: Uuid,
        note_index: usize,
    ) -> Result<(), String> {
        let t_idx = self.tracks.iter().position(|t| t.id == track_id)
            .ok_or("track not found")?;
        let r_idx = self.tracks[t_idx].regions.iter().position(|r| r.id == region_id)
            .ok_or("region not found")?;
        if note_index >= self.tracks[t_idx].regions[r_idx].notes.len() {
            return Err("note index out of range".into());
        }
        self.record_song_edit();
        self.tracks[t_idx].regions[r_idx].notes.remove(note_index);
        Ok(())
    }

    pub fn get_all_overlaps(&self) -> Vec<OverlapWarning> {
        let snapshot = self.build_snapshot();
        snapshot.channels.into_iter().flat_map(|ch| ch.overlaps).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::FlamedriverProfile;
    use std::env;

    fn test_registry() -> DriverRegistry {
        let mut reg = DriverRegistry::new();
        reg.register(Box::new(FlamedriverProfile));
        reg
    }

    fn temp_project_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!("seraph_test_{name}_{}", Uuid::new_v4()))
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn test_create_project_makes_folder_structure() {
        let path = temp_project_path("create");
        let mut mgr = ProjectManager::new(test_registry());

        mgr.create(&path, "Test Song", "flamedriver", 120.0, (4, 4)).unwrap();

        assert!(path.join(".seraph").exists());
        assert!(path.join("project.json").exists());
        assert!(path.join("instruments/fm").is_dir());
        assert!(path.join("instruments/psg").is_dir());
        assert!(path.join("instruments/dac").is_dir());
        assert!(path.join("exports").is_dir());
        assert!(mgr.is_open());

        cleanup(&path);
    }

    /// The channel roster a fresh project should be seeded with, derived from
    /// the driver's own channel layout (never hardcoded lane counts).
    fn expected_roster(layout: &crate::model::driver::ChannelLayout) -> Vec<(String, ChannelAssignment)> {
        let mut expected: Vec<(String, ChannelAssignment)> = Vec::new();
        for ch in &layout.fm_channels {
            expected.push((ch.name.clone(), ChannelAssignment::Fm(ch.index)));
        }
        for ch in &layout.psg_channels {
            let assign = if ch.is_noise { ChannelAssignment::PsgNoise } else { ChannelAssignment::Psg(ch.index) };
            expected.push((ch.name.clone(), assign));
        }
        for ch in &layout.dac_channels {
            expected.push((ch.name.clone(), ChannelAssignment::Dac(ch.index)));
        }
        expected
    }

    #[test]
    fn test_create_seeds_default_track_roster() {
        use crate::model::driver::DriverProfile as _;

        let path = temp_project_path("seed_roster");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Seeded", "flamedriver", 120.0, (4, 4)).unwrap();

        let expected = expected_roster(&FlamedriverProfile.channel_layout());
        assert!(!expected.is_empty(), "layout must expose channels");

        let tracks = mgr.list_tracks();
        assert_eq!(tracks.len(), expected.len(), "one seeded track per driver channel");
        for (track, (name, channel)) in tracks.iter().zip(&expected) {
            assert_eq!(&track.name, name);
            assert_eq!(
                format!("{:?}", track.channel),
                format!("{:?}", channel),
                "track '{}' channel assignment", name
            );
            assert_eq!(track.instrument_id, None, "seeded tracks carry no instrument");
            assert!(track.regions.is_empty());
        }

        // create() writes project.json itself — the roster must be persisted
        // without an explicit save.
        let mut mgr2 = ProjectManager::new(test_registry());
        let song = mgr2.open(&path).unwrap();
        assert_eq!(song.tracks.len(), expected.len(), "roster persisted to project.json");

        cleanup(&path);
    }

    /// With a seeded roster, adding an instrument must bind it to the first
    /// empty lane of its kind instead of growing a duplicate-channel track,
    /// and deleting the instrument must unbind — the lane survives.
    #[test]
    fn test_add_fm_instrument_binds_to_empty_roster_lane() {
        let path = temp_project_path("bind_lane");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Bind", "flamedriver", 120.0, (4, 4)).unwrap();

        let track_count = mgr.list_tracks().len();
        let id = mgr.add_fm_instrument(assign_test_fm());
        assert_eq!(mgr.list_tracks().len(), track_count, "binds to an empty FM lane, no new track");
        let track = mgr.list_tracks().iter().find(|t| t.instrument_id == Some(id))
            .expect("instrument bound to a track");
        assert!(matches!(track.channel, ChannelAssignment::Fm(0)), "lowest empty FM lane first");
        assert_eq!(track.name, "Library Lead", "lane renamed to the bound instrument");

        mgr.delete_fm_instrument(id).unwrap();
        assert_eq!(mgr.list_tracks().len(), track_count, "delete unbinds; the lane survives");
        assert!(mgr.list_tracks().iter().all(|t| t.instrument_id.is_none()));

        cleanup(&path);
    }

    #[test]
    fn test_create_rejects_unknown_driver() {
        let path = temp_project_path("bad_driver");
        let mut mgr = ProjectManager::new(test_registry());
        let result = mgr.create(&path, "X", "nonexistent", 120.0, (4, 4));
        assert!(result.is_err());
        cleanup(&path);
    }

    #[test]
    fn test_open_rejects_non_project() {
        let path = temp_project_path("not_project");
        fs::create_dir_all(&path).unwrap();
        let mut mgr = ProjectManager::new(test_registry());
        let result = mgr.open(&path);
        assert!(result.is_err());
        cleanup(&path);
    }

    #[test]
    fn test_create_save_open_round_trip() {
        let path = temp_project_path("round_trip");
        let mut mgr = ProjectManager::new(test_registry());

        mgr.create(&path, "Round Trip", "flamedriver", 150.0, (3, 4)).unwrap();

        let fm_inst = FmInstrument {
            id: Uuid::nil(),
            name: "Bass".into(),
            algorithm: 2,
            feedback: 3,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        };
        let fm_id = mgr.add_fm_instrument(fm_inst);

        let psg_inst = PsgInstrument {
            id: Uuid::nil(),
            name: "Pluck".into(),
            volume_sequence: vec![15, 12, 8, 4, 0],
            loop_point: None,
            silence_on_end: true,
            noise_mode: None,
            smps_envelope_index: None,
            metadata: InstrumentMetadata::default(),
        };
        let psg_id = mgr.add_psg_instrument(psg_inst);

        mgr.save().unwrap();
        mgr.close();
        assert!(!mgr.is_open());

        let song = mgr.open(&path).unwrap();
        assert_eq!(song.metadata.name, "Round Trip");
        assert_eq!(song.metadata.tempo, 150.0);
        assert_eq!(song.instruments.fm.len(), 1);
        assert_eq!(song.instruments.fm[0].id, fm_id);
        assert_eq!(song.instruments.fm[0].name, "Bass");
        assert_eq!(song.instruments.psg.len(), 1);
        assert_eq!(song.instruments.psg[0].id, psg_id);

        cleanup(&path);
    }

    #[test]
    fn test_delete_fm_removes_file() {
        let path = temp_project_path("delete_fm");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Del Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let inst = FmInstrument {
            id: Uuid::nil(),
            name: "ToDelete".into(),
            algorithm: 0,
            feedback: 0,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        };
        let id = mgr.add_fm_instrument(inst);
        mgr.save().unwrap();
        assert!(path.join(format!("instruments/fm/{id}.json")).exists());

        mgr.delete_fm_instrument(id).unwrap();
        assert!(!path.join(format!("instruments/fm/{id}.json")).exists());
        assert!(mgr.list_fm_instruments().is_empty());

        cleanup(&path);
    }

    #[test]
    fn test_close_clears_state() {
        let path = temp_project_path("close");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Test", "flamedriver", 120.0, (4, 4)).unwrap();
        mgr.close();
        assert!(!mgr.is_open());
        assert!(mgr.metadata().is_none());
        assert!(mgr.list_fm_instruments().is_empty());
        cleanup(&path);
    }

    #[test]
    fn test_get_dac_pcm_returns_cached_data() {
        let path = temp_project_path("dac_pcm");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "PCM Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let inst = DacInstrument {
            id: Uuid::new_v4(),
            name: "Test".into(),
            target_sample_rate: 16000,
            loop_start: None,
            loop_length: None,
            original_file: "test.raw".into(),
            pcm_file: "test.pcm".into(),
            source_is_raw: true,
            metadata: InstrumentMetadata::default(),
        };
        let pcm_data = vec![128u8, 130, 132, 134];
        let id = mgr.add_dac_instrument(inst, pcm_data.clone());

        let cached = mgr.get_dac_pcm(&id).unwrap();
        assert_eq!(cached.as_ref(), &pcm_data);

        assert!(mgr.get_dac_pcm(&Uuid::new_v4()).is_none());

        cleanup(&path);
    }

    // --- Snapshot builder tests ---

    #[test]
    fn test_track_crud() {
        let path = temp_project_path("track_crud");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Track Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let baseline = mgr.list_tracks().len(); // seeded roster
        let id = mgr.add_track("FM1-Bass".into(), ChannelAssignment::Fm(0), None);
        assert_eq!(mgr.list_tracks().len(), baseline + 1);
        let added = mgr.list_tracks().iter().find(|t| t.id == id).unwrap();
        assert_eq!(added.name, "FM1-Bass");

        mgr.update_track(id, "FM1-Lead".into(), ChannelAssignment::Fm(0), None, true, false, 80, Pan::Left, 0).unwrap();
        let added = mgr.list_tracks().iter().find(|t| t.id == id).unwrap();
        assert_eq!(added.name, "FM1-Lead");
        assert!(added.muted);

        mgr.delete_track(id).unwrap();
        assert_eq!(mgr.list_tracks().len(), baseline);
        assert!(mgr.list_tracks().iter().all(|t| t.id != id));

        cleanup(&path);
    }

    #[test]
    fn test_region_crud() {
        let path = temp_project_path("region_crud");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Region Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let track_id = mgr.add_track("FM1".into(), ChannelAssignment::Fm(0), None);
        let track = |mgr: &ProjectManager| mgr.list_tracks().iter().find(|t| t.id == track_id).unwrap().clone();
        let region_id = mgr.add_region(track_id, 0, 1920).unwrap();
        assert_eq!(track(&mgr).regions.len(), 1);

        mgr.update_region(track_id, region_id, 480, 960).unwrap();
        assert_eq!(track(&mgr).regions[0].start_tick, 480);

        mgr.delete_region(track_id, region_id).unwrap();
        assert!(track(&mgr).regions.is_empty());

        cleanup(&path);
    }

    #[test]
    fn test_duplicate_region_deep_clones_notes() {
        let path = temp_project_path("dup_region");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Dup Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let track_id = mgr.add_track("FM1".into(), ChannelAssignment::Fm(0), None);
        let track = |mgr: &ProjectManager| mgr.list_tracks().iter().find(|t| t.id == track_id).unwrap().clone();
        let region_id = mgr.add_region(track_id, 0, 1920).unwrap();
        mgr.add_note(track_id, region_id, 0, 60, 100, 240).unwrap();
        mgr.add_note(track_id, region_id, 480, 64, 80, 480).unwrap();

        let dup_id = mgr.duplicate_region(track_id, region_id, 1920).unwrap();
        assert_ne!(dup_id, region_id, "clone gets a fresh region id");

        let t = track(&mgr);
        assert_eq!(t.regions.len(), 2);
        let dup = t.regions.iter().find(|r| r.id == dup_id).expect("clone present");
        assert_eq!(dup.start_tick, 1920, "clone placed at the given start tick");
        assert_eq!(dup.duration_ticks, 1920, "duration carried over");
        assert_eq!(dup.notes.len(), 2, "clone carries the notes");
        assert_eq!((dup.notes[0].pitch, dup.notes[0].velocity), (60, 100));
        assert_eq!((dup.notes[1].tick, dup.notes[1].duration_ticks), (480, 480));

        // Deep clone: editing the copy must not touch the original.
        mgr.update_note(track_id, dup_id, 0, 0, 72, 100, 240).unwrap();
        let t = track(&mgr);
        let orig = t.regions.iter().find(|r| r.id == region_id).unwrap();
        assert_eq!(orig.notes[0].pitch, 60, "original untouched by editing the clone");

        // Validate-first (update_note's pattern): bad ids error out without
        // mutating anything.
        assert!(mgr.duplicate_region(track_id, Uuid::new_v4(), 0).is_err());
        assert!(mgr.duplicate_region(Uuid::new_v4(), region_id, 0).is_err());
        assert_eq!(track(&mgr).regions.len(), 2);

        cleanup(&path);
    }

    #[test]
    fn test_undo_removes_duplicated_region() {
        let path = temp_project_path("dup_region_undo");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Dup Undo", "flamedriver", 120.0, (4, 4)).unwrap();

        let track_id = mgr.add_track("FM1".into(), ChannelAssignment::Fm(0), None);
        let track = |mgr: &ProjectManager| mgr.list_tracks().iter().find(|t| t.id == track_id).unwrap().clone();
        let region_id = mgr.add_region(track_id, 0, 1920).unwrap();
        mgr.add_note(track_id, region_id, 0, 60, 100, 240).unwrap();

        let dup_id = mgr.duplicate_region(track_id, region_id, 1920).unwrap();
        assert_eq!(track(&mgr).regions.len(), 2);

        // duplicate_region must record_song_edit() BEFORE mutating: one undo
        // step removes exactly the clone, leaving the original intact.
        mgr.undo();
        let t = track(&mgr);
        assert_eq!(t.regions.len(), 1, "undo removed the clone");
        assert_eq!(t.regions[0].id, region_id, "the surviving region is the original");
        assert_eq!(t.regions[0].notes.len(), 1, "original notes intact");

        mgr.redo();
        let t = track(&mgr);
        assert_eq!(t.regions.len(), 2, "redo restores the clone");
        assert!(t.regions.iter().any(|r| r.id == dup_id));

        cleanup(&path);
    }

    #[test]
    fn test_note_crud() {
        let path = temp_project_path("note_crud");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Note Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let track_id = mgr.add_track("FM1".into(), ChannelAssignment::Fm(0), None);
        let track = |mgr: &ProjectManager| mgr.list_tracks().iter().find(|t| t.id == track_id).unwrap().clone();
        let region_id = mgr.add_region(track_id, 0, 1920).unwrap();

        let idx = mgr.add_note(track_id, region_id, 0, 60, 100, 240).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(track(&mgr).regions[0].notes.len(), 1);

        mgr.update_note(track_id, region_id, 0, 120, 64, 80, 480).unwrap();
        assert_eq!(track(&mgr).regions[0].notes[0].pitch, 64);

        mgr.delete_note(track_id, region_id, 0).unwrap();
        assert!(track(&mgr).regions[0].notes.is_empty());

        cleanup(&path);
    }

    #[test]
    fn test_track_save_load_round_trip() {
        let path = temp_project_path("track_save");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Save Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let track_id = mgr.add_track("FM1".into(), ChannelAssignment::Fm(0), None);
        let region_id = mgr.add_region(track_id, 0, 1920).unwrap();
        mgr.add_note(track_id, region_id, 0, 60, 100, 480).unwrap();
        mgr.add_note(track_id, region_id, 480, 64, 80, 240).unwrap();

        let track_count = mgr.list_tracks().len();
        mgr.save().unwrap();
        mgr.close();

        let song = mgr.open(&path).unwrap();
        assert_eq!(song.tracks.len(), track_count);
        let track = song.tracks.iter().find(|t| t.id == track_id).unwrap();
        assert_eq!(track.regions.len(), 1);
        assert_eq!(track.regions[0].notes.len(), 2);
        assert_eq!(track.regions[0].notes[0].pitch, 60);

        cleanup(&path);
    }

    #[test]
    fn test_build_snapshot_empty_project() {
        let path = temp_project_path("snap_empty");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Empty", "flamedriver", 140.0, (4, 4)).unwrap();

        let snap = mgr.build_snapshot();
        assert_eq!(snap.tempo_bpm, 140.0);
        assert_eq!(snap.ticks_per_beat, 480);
        // The seeded roster surfaces one channel per driver lane, all silent.
        {
            use crate::model::driver::DriverProfile as _;
            let layout = FlamedriverProfile.channel_layout();
            let lane_count = layout.fm_channels.len() + layout.psg_channels.len() + layout.dac_channels.len();
            assert_eq!(snap.channels.len(), lane_count, "one channel per seeded lane");
        }
        assert!(snap.channels.iter().all(|c| c.events.is_empty()), "empty project has no events");

        cleanup(&path);
    }

    #[test]
    fn test_build_snapshot_skips_muted_tracks() {
        let path = temp_project_path("snap_mute");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Mute Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let fm_inst = FmInstrument {
            id: Uuid::nil(),
            name: "Test".into(),
            algorithm: 0,
            feedback: 0,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        };
        let fm_id = mgr.add_fm_instrument(fm_inst);

        // add_fm_instrument auto-creates a track; find it and set it up
        let track = mgr.tracks.iter_mut().find(|t| t.instrument_id == Some(fm_id)).unwrap();
        track.muted = true;
        track.regions.push(Region {
            id: Uuid::new_v4(),
            start_tick: 0,
            duration_ticks: 480,
            notes: vec![Note { tick: 0, pitch: 60, velocity: 100, duration_ticks: 240, instrument_id: None, detune: 0, pan_override: None, modulation: None }],
            instrument_id: None,
        });

        let snap = mgr.build_snapshot();
        // The other seeded lanes still surface (silent) channels; the muted
        // track's notes must not reach any of them.
        assert!(
            snap.channels.iter().all(|c| c.events.is_empty()),
            "muted track's events should be excluded"
        );

        cleanup(&path);
    }

    #[test]
    fn test_build_snapshot_solo_filters() {
        let path = temp_project_path("snap_solo");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Solo Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let fm_inst1 = FmInstrument {
            id: Uuid::nil(),
            name: "Solo".into(),
            algorithm: 0,
            feedback: 0,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        };
        let fm_id1 = mgr.add_fm_instrument(fm_inst1);

        let fm_inst2 = FmInstrument {
            id: Uuid::nil(),
            name: "NotSolo".into(),
            algorithm: 0,
            feedback: 0,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        };
        let fm_id2 = mgr.add_fm_instrument(fm_inst2);

        let note = Note { tick: 0, pitch: 60, velocity: 100, duration_ticks: 240, instrument_id: None, detune: 0, pan_override: None, modulation: None };

        // Set up first auto-created track as solo'd with a region
        let track1 = mgr.tracks.iter_mut().find(|t| t.instrument_id == Some(fm_id1)).unwrap();
        track1.solo = true;
        track1.regions.push(Region {
            id: Uuid::new_v4(),
            start_tick: 0,
            duration_ticks: 480,
            notes: vec![note.clone()],
            instrument_id: None,
        });

        // Set up second auto-created track (not solo'd) with a region
        let track2 = mgr.tracks.iter_mut().find(|t| t.instrument_id == Some(fm_id2)).unwrap();
        track2.regions.push(Region {
            id: Uuid::new_v4(),
            start_tick: 0,
            duration_ticks: 480,
            notes: vec![note],
            instrument_id: None,
        });

        let snap = mgr.build_snapshot();
        assert_eq!(snap.channels.len(), 1, "only solo'd track should appear");

        cleanup(&path);
    }

    #[test]
    fn test_build_snapshot_detects_overlaps() {
        let path = temp_project_path("snap_overlap");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Overlap Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let fm_inst1 = FmInstrument {
            id: Uuid::nil(),
            name: "A".into(),
            algorithm: 0,
            feedback: 0,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        };
        let fm_id1 = mgr.add_fm_instrument(fm_inst1);

        let fm_inst2 = FmInstrument {
            id: Uuid::nil(),
            name: "B".into(),
            algorithm: 0,
            feedback: 0,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        };
        let fm_id2 = mgr.add_fm_instrument(fm_inst2);

        // Force both tracks onto the same channel to create overlap
        let track1 = mgr.tracks.iter_mut().find(|t| t.instrument_id == Some(fm_id1)).unwrap();
        track1.channel = ChannelAssignment::Fm(0);
        track1.regions.push(Region {
            id: Uuid::new_v4(),
            start_tick: 0,
            duration_ticks: 960,
            notes: vec![Note { tick: 0, pitch: 60, velocity: 100, duration_ticks: 480, instrument_id: None, detune: 0, pan_override: None, modulation: None }],
            instrument_id: None,
        });

        let track2 = mgr.tracks.iter_mut().find(|t| t.instrument_id == Some(fm_id2)).unwrap();
        track2.channel = ChannelAssignment::Fm(0);
        track2.regions.push(Region {
            id: Uuid::new_v4(),
            start_tick: 0,
            duration_ticks: 960,
            notes: vec![Note { tick: 240, pitch: 64, velocity: 100, duration_ticks: 480, instrument_id: None, detune: 0, pan_override: None, modulation: None }],
            instrument_id: None,
        });

        let snap = mgr.build_snapshot();
        let fm0 = snap.channels.iter()
            .find(|c| matches!(c.channel_type, ChannelType::Fm(0)))
            .expect("FM1 channel present");
        assert!(!fm0.overlaps.is_empty(), "should detect overlap");

        cleanup(&path);
    }

    #[test]
    fn test_build_snapshot_events_sorted() {
        let path = temp_project_path("snap_sorted");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Sort Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let fm_inst = FmInstrument {
            id: Uuid::nil(),
            name: "Test".into(),
            algorithm: 0,
            feedback: 0,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        };
        let fm_id = mgr.add_fm_instrument(fm_inst);

        // Use the auto-created track
        let track = mgr.tracks.iter_mut().find(|t| t.instrument_id == Some(fm_id)).unwrap();
        track.regions.push(Region {
            id: Uuid::new_v4(),
            start_tick: 0,
            duration_ticks: 1920,
            notes: vec![
                Note { tick: 480, pitch: 60, velocity: 100, duration_ticks: 240, instrument_id: None, detune: 0, pan_override: None, modulation: None },
                Note { tick: 0, pitch: 48, velocity: 100, duration_ticks: 480, instrument_id: None, detune: 0, pan_override: None, modulation: None },
            ],
            instrument_id: None,
        });

        let snap = mgr.build_snapshot();
        let channel = snap.channels.iter()
            .find(|c| !c.events.is_empty())
            .expect("the note-carrying channel is present");
        let ticks: Vec<u64> = channel.events.iter().map(|e| e.tick()).collect();
        assert!(ticks.windows(2).all(|w| w[0] <= w[1]), "events should be sorted by tick");

        cleanup(&path);
    }

    fn assign_test_fm() -> FmInstrument {
        FmInstrument {
            id: Uuid::nil(),
            name: "Library Lead".into(),
            algorithm: 4,
            feedback: 5,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        }
    }

    #[test]
    fn test_assign_library_voice_reuses_matching_hash() {
        use crate::library::entry::{content_hash, LibraryInstrument};

        let path = temp_project_path("assign_reuse");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Assign", "flamedriver", 120.0, (4, 4)).unwrap();

        let inst = assign_test_fm();
        let existing_id = mgr.add_fm_instrument(inst.clone());
        let track_id = mgr.add_track("Empty".into(), ChannelAssignment::Fm(1), None);

        let voice = LibraryInstrument::Fm(inst);
        let hash = content_hash(&voice);
        let bound = mgr.assign_library_instrument_to_track(track_id, &voice, &hash).unwrap();

        assert_eq!(bound, existing_id, "same-hash project instrument must be reused");
        assert_eq!(mgr.list_fm_instruments().len(), 1, "no duplicate instrument added");
        let track = mgr.list_tracks().iter().find(|t| t.id == track_id).unwrap();
        assert_eq!(track.instrument_id, Some(existing_id));
        assert_eq!(track.name, "Library Lead");

        cleanup(&path);
    }

    #[test]
    fn test_assign_library_voice_adds_when_no_hash_match() {
        use crate::library::entry::{content_hash, LibraryInstrument};

        let path = temp_project_path("assign_add");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Assign", "flamedriver", 120.0, (4, 4)).unwrap();

        let track_id = mgr.add_track("Empty".into(), ChannelAssignment::Fm(0), None);
        let track_count = mgr.list_tracks().len();

        let voice = LibraryInstrument::Fm(assign_test_fm());
        let hash = content_hash(&voice);
        let bound = mgr.assign_library_instrument_to_track(track_id, &voice, &hash).unwrap();

        assert_eq!(mgr.list_fm_instruments().len(), 1, "voice added to the bank");
        assert_eq!(mgr.list_fm_instruments()[0].id, bound);
        assert_eq!(mgr.list_tracks().len(), track_count, "no extra track created");
        let track = mgr.list_tracks().iter().find(|t| t.id == track_id).unwrap();
        assert_eq!(track.instrument_id, Some(bound));

        cleanup(&path);
    }

    #[test]
    fn test_assign_library_voice_kind_mismatch_errors() {
        use crate::library::entry::{content_hash, LibraryInstrument};

        let path = temp_project_path("assign_mismatch");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Assign", "flamedriver", 120.0, (4, 4)).unwrap();

        let psg_track = mgr.add_track("PSG".into(), ChannelAssignment::Psg(0), None);
        let voice = LibraryInstrument::Fm(assign_test_fm());
        let hash = content_hash(&voice);

        let err = mgr.assign_library_instrument_to_track(psg_track, &voice, &hash).unwrap_err();
        assert!(err.contains("FM voice"), "error should name the mismatch: {err}");
        assert!(mgr.list_fm_instruments().is_empty(), "nothing added on error");

        cleanup(&path);
    }

    fn first_fm_patch_bytes(snap: &SequencerSnapshot) -> [u8; 25] {
        for ch in &snap.channels {
            for ev in &ch.events {
                if let SequencerEvent::NoteOn {
                    instrument: InstrumentData::FmPatch { bytes, .. }, ..
                } = ev
                {
                    return *bytes;
                }
            }
        }
        panic!("no FM NoteOn in snapshot");
    }

    /// Importers stamp `instrument_id` on every note/region and
    /// `build_snapshot` gives those precedence over the track binding — a
    /// drag-to-track swap must clear them or it is inaudible.
    #[test]
    fn test_assign_library_voice_clears_note_overrides_and_changes_sound() {
        use crate::library::entry::{content_hash, LibraryInstrument};

        let path = temp_project_path("assign_overrides");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Assign", "flamedriver", 120.0, (4, 4)).unwrap();

        let old_id = mgr.add_fm_instrument(assign_test_fm());
        let track_id = mgr.tracks.iter()
            .find(|t| t.instrument_id == Some(old_id))
            .unwrap().id;

        // Imported-style data: region AND note carry the old instrument id.
        let track = mgr.tracks.iter_mut().find(|t| t.id == track_id).unwrap();
        track.regions.push(Region {
            id: Uuid::new_v4(),
            start_tick: 0,
            duration_ticks: 960,
            notes: vec![Note {
                tick: 0,
                pitch: 60,
                velocity: 100,
                duration_ticks: 480,
                instrument_id: Some(old_id),
                detune: 0,
                pan_override: None,
                modulation: None,
            }],
            instrument_id: Some(old_id),
        });

        let before = first_fm_patch_bytes(&mgr.build_snapshot());

        // A library voice with a different sound (and thus different hash).
        let mut new_fm = assign_test_fm();
        new_fm.name = "Swapped Voice".into();
        new_fm.algorithm = 2;
        new_fm.feedback = 1;
        new_fm.operators[0].total_level = 33;
        let voice = LibraryInstrument::Fm(new_fm);
        let hash = content_hash(&voice);
        mgr.assign_library_instrument_to_track(track_id, &voice, &hash).unwrap();

        let after = first_fm_patch_bytes(&mgr.build_snapshot());
        assert_ne!(before, after, "the swap must reach the audible patch");

        let track = mgr.list_tracks().iter().find(|t| t.id == track_id).unwrap();
        for region in &track.regions {
            assert_eq!(region.instrument_id, None, "region override must be cleared");
            for note in &region.notes {
                assert_eq!(note.instrument_id, None, "note override must be cleared");
            }
        }

        cleanup(&path);
    }

    // --- Undo / redo (song edits) ---

    /// Fresh project + one empty region on a new track; returns
    /// (mgr, path, track_id, region_id).
    fn undo_fixture(name: &str) -> (ProjectManager, PathBuf, Uuid, Uuid) {
        let path = temp_project_path(name);
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Undo", "flamedriver", 120.0, (4, 4)).unwrap();
        let track_id = mgr.add_track("FM1".into(), ChannelAssignment::Fm(0), None);
        let region_id = mgr.add_region(track_id, 0, 1920).unwrap();
        (mgr, path, track_id, region_id)
    }

    fn region_notes(mgr: &ProjectManager, track_id: Uuid, region_id: Uuid) -> Vec<Note> {
        mgr.list_tracks().iter()
            .find(|t| t.id == track_id).expect("track present")
            .regions.iter()
            .find(|r| r.id == region_id).expect("region present")
            .notes.clone()
    }

    #[test]
    fn test_undo_redo_note_add_round_trip() {
        let (mut mgr, path, track_id, region_id) = undo_fixture("undo_note_add");

        mgr.add_note(track_id, region_id, 0, 60, 100, 240).unwrap();
        assert_eq!(region_notes(&mgr, track_id, region_id).len(), 1);
        assert!(mgr.can_undo());

        let tracks = mgr.undo();
        assert!(
            region_notes(&mgr, track_id, region_id).is_empty(),
            "undo must remove the added note"
        );
        // undo returns the restored state
        let ret_region = tracks.iter().find(|t| t.id == track_id).unwrap()
            .regions.iter().find(|r| r.id == region_id).unwrap().clone();
        assert!(ret_region.notes.is_empty());
        assert!(mgr.can_redo());

        mgr.redo();
        let notes = region_notes(&mgr, track_id, region_id);
        assert_eq!(notes.len(), 1, "redo must restore the note");
        assert_eq!(notes[0].pitch, 60);
        assert_eq!(notes[0].velocity, 100);

        cleanup(&path);
    }

    #[test]
    fn test_undo_redo_note_update_and_delete() {
        let (mut mgr, path, track_id, region_id) = undo_fixture("undo_note_upd");
        mgr.add_note(track_id, region_id, 0, 60, 100, 240).unwrap();

        mgr.update_note(track_id, region_id, 0, 480, 64, 90, 120).unwrap();
        assert_eq!(region_notes(&mgr, track_id, region_id)[0].pitch, 64);
        mgr.undo();
        let n = &region_notes(&mgr, track_id, region_id)[0];
        assert_eq!((n.tick, n.pitch, n.velocity, n.duration_ticks), (0, 60, 100, 240),
            "undo restores the pre-update note");
        mgr.redo();
        let n = &region_notes(&mgr, track_id, region_id)[0];
        assert_eq!((n.tick, n.pitch, n.velocity, n.duration_ticks), (480, 64, 90, 120));

        mgr.delete_note(track_id, region_id, 0).unwrap();
        assert!(region_notes(&mgr, track_id, region_id).is_empty());
        mgr.undo();
        assert_eq!(region_notes(&mgr, track_id, region_id).len(), 1,
            "undo restores the deleted note");

        cleanup(&path);
    }

    #[test]
    fn test_undo_redo_region_ops() {
        let (mut mgr, path, track_id, region_id) = undo_fixture("undo_region");
        let track2 = mgr.add_track("FM2".into(), ChannelAssignment::Fm(1), None);

        // update_region
        mgr.update_region(track_id, region_id, 480, 960).unwrap();
        mgr.undo();
        let t = mgr.list_tracks().iter().find(|t| t.id == track_id).unwrap();
        assert_eq!(t.regions[0].start_tick, 0, "undo restores region start");
        assert_eq!(t.regions[0].duration_ticks, 1920);

        // move_region across tracks
        mgr.move_region(track_id, region_id, track2, 240).unwrap();
        assert!(mgr.list_tracks().iter().find(|t| t.id == track2).unwrap()
            .regions.iter().any(|r| r.id == region_id));
        mgr.undo();
        assert!(mgr.list_tracks().iter().find(|t| t.id == track_id).unwrap()
            .regions.iter().any(|r| r.id == region_id),
            "undo returns the region to its source track");

        // delete_region
        mgr.delete_region(track_id, region_id).unwrap();
        mgr.undo();
        assert!(mgr.list_tracks().iter().find(|t| t.id == track_id).unwrap()
            .regions.iter().any(|r| r.id == region_id),
            "undo restores the deleted region");

        cleanup(&path);
    }

    #[test]
    fn test_undo_redo_track_ops() {
        let (mut mgr, path, track_id, _region_id) = undo_fixture("undo_track");

        // update_track (mute)
        mgr.update_track(track_id, "FM1".into(), ChannelAssignment::Fm(0), None,
            true, false, 80, Pan::Left, 2).unwrap();
        mgr.undo();
        let t = mgr.list_tracks().iter().find(|t| t.id == track_id).unwrap();
        assert!(!t.muted, "undo restores mute state");
        assert_eq!(t.volume, 100);
        assert_eq!(t.pitch_offset, 0);
        mgr.redo();
        let t = mgr.list_tracks().iter().find(|t| t.id == track_id).unwrap();
        assert!(t.muted);
        assert_eq!(t.volume, 80);

        // delete_track / undo restores it with its regions
        let count = mgr.list_tracks().len();
        mgr.delete_track(track_id).unwrap();
        assert_eq!(mgr.list_tracks().len(), count - 1);
        mgr.undo();
        assert_eq!(mgr.list_tracks().len(), count, "undo restores the deleted track");
        assert!(mgr.list_tracks().iter().any(|t| t.id == track_id));

        // add_track / undo removes it
        let new_id = mgr.add_track("Extra".into(), ChannelAssignment::Fm(2), None);
        mgr.undo();
        assert!(mgr.list_tracks().iter().all(|t| t.id != new_id),
            "undo removes the added track");

        cleanup(&path);
    }

    #[test]
    fn test_undo_group_coalesces_to_one_step() {
        let (mut mgr, path, track_id, region_id) = undo_fixture("undo_group");
        mgr.add_note(track_id, region_id, 0, 60, 100, 240).unwrap();

        // A drag gesture: one update per mousemove, bracketed by the group.
        mgr.begin_undo_group();
        for pitch in [61u8, 62, 63, 64] {
            mgr.update_note(track_id, region_id, 0, 0, pitch, 100, 240).unwrap();
        }
        mgr.end_undo_group();
        assert_eq!(region_notes(&mgr, track_id, region_id)[0].pitch, 64);

        // ONE undo step jumps all the way back to the pre-gesture state.
        mgr.undo();
        assert_eq!(region_notes(&mgr, track_id, region_id)[0].pitch, 60,
            "grouped updates must coalesce into a single undo step");
        // And redo restores the gesture's final state.
        mgr.redo();
        assert_eq!(region_notes(&mgr, track_id, region_id)[0].pitch, 64);

        cleanup(&path);
    }

    #[test]
    fn test_undo_group_unbalanced_begin_end_are_safe() {
        let (mut mgr, path, track_id, region_id) = undo_fixture("undo_group_unbal");
        mgr.add_note(track_id, region_id, 0, 60, 100, 240).unwrap();

        // Nested begin is a no-op; the inner end closes the (single) group.
        mgr.begin_undo_group();
        mgr.begin_undo_group();
        mgr.update_note(track_id, region_id, 0, 0, 61, 100, 240).unwrap();
        mgr.update_note(track_id, region_id, 0, 0, 62, 100, 240).unwrap();
        mgr.end_undo_group();
        mgr.end_undo_group(); // end without open group: no-op

        mgr.undo();
        assert_eq!(region_notes(&mgr, track_id, region_id)[0].pitch, 60,
            "nested begins must not fragment the group");

        // After the stray end, normal per-mutation snapshots resume.
        mgr.redo();
        mgr.update_note(track_id, region_id, 0, 0, 70, 100, 240).unwrap();
        mgr.update_note(track_id, region_id, 0, 0, 71, 100, 240).unwrap();
        mgr.undo();
        assert_eq!(region_notes(&mgr, track_id, region_id)[0].pitch, 70,
            "post-group mutations get individual undo steps again");

        cleanup(&path);
    }

    #[test]
    fn test_redo_cleared_on_new_mutation() {
        let (mut mgr, path, track_id, region_id) = undo_fixture("undo_redo_clear");
        mgr.add_note(track_id, region_id, 0, 60, 100, 240).unwrap();
        mgr.undo();
        assert!(mgr.can_redo());

        // A fresh mutation invalidates the redo branch.
        mgr.add_note(track_id, region_id, 480, 72, 100, 240).unwrap();
        assert!(!mgr.can_redo(), "new mutation must clear the redo stack");
        let before = region_notes(&mgr, track_id, region_id);
        mgr.redo(); // must be a no-op
        assert_eq!(region_notes(&mgr, track_id, region_id).len(), before.len());

        cleanup(&path);
    }

    #[test]
    fn test_undo_stack_cap_enforced() {
        let (mut mgr, path, track_id, region_id) = undo_fixture("undo_cap");
        mgr.add_note(track_id, region_id, 0, 60, 100, 240).unwrap();

        // Well past the cap (setup already pushed a few snapshots).
        for i in 0..(MAX_UNDO_DEPTH + 50) {
            mgr.update_note(track_id, region_id, 0, 0, (i % 90) as u8 + 20, 100, 240).unwrap();
        }
        let mut undos = 0;
        while mgr.can_undo() {
            mgr.undo();
            undos += 1;
            assert!(undos <= MAX_UNDO_DEPTH, "stack must be capped at MAX_UNDO_DEPTH");
        }
        assert_eq!(undos, MAX_UNDO_DEPTH, "exactly MAX_UNDO_DEPTH steps retained");

        cleanup(&path);
    }

    #[test]
    fn test_dirty_transitions() {
        let path = temp_project_path("dirty");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Dirty", "flamedriver", 120.0, (4, 4)).unwrap();
        assert!(!mgr.is_dirty(), "fresh project starts clean");

        let track_id = mgr.add_track("FM1".into(), ChannelAssignment::Fm(0), None);
        assert!(mgr.is_dirty(), "song edit marks dirty");
        mgr.save().unwrap();
        assert!(!mgr.is_dirty(), "save clears dirty");

        let region_id = mgr.add_region(track_id, 0, 1920).unwrap();
        mgr.save().unwrap();
        mgr.add_note(track_id, region_id, 0, 60, 100, 240).unwrap();
        assert!(mgr.is_dirty());
        mgr.undo();
        assert!(mgr.is_dirty(), "undo does NOT clear dirty — state differs from disk");

        // Out-of-undo-scope mutations still mark dirty (dirty is about saving).
        mgr.save().unwrap();
        let fm_id = mgr.add_fm_instrument(assign_test_fm());
        assert!(mgr.is_dirty(), "instrument add marks dirty");
        mgr.save().unwrap();
        assert!(!mgr.is_dirty());
        let mut inst = mgr.get_fm_instrument(&fm_id).unwrap().clone();
        inst.feedback = 5;
        mgr.update_fm_instrument(fm_id, inst).unwrap();
        assert!(mgr.is_dirty(), "instrument update marks dirty");

        // Re-open clears dirty and the undo history.
        mgr.save().unwrap();
        mgr.close();
        mgr.open(&path).unwrap();
        assert!(!mgr.is_dirty(), "open starts clean");
        assert!(!mgr.can_undo(), "undo history does not cross open");
        assert!(!mgr.can_redo());

        cleanup(&path);
    }

    #[test]
    fn test_instrument_ops_do_not_enter_undo_stack() {
        let path = temp_project_path("undo_scope");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Scope", "flamedriver", 120.0, (4, 4)).unwrap();
        assert!(!mgr.can_undo());

        // Instrument library ops are out of undo scope even though the add
        // path binds a track lane.
        let fm_id = mgr.add_fm_instrument(assign_test_fm());
        assert!(!mgr.can_undo(), "instrument add is not undoable");
        let mut inst = mgr.get_fm_instrument(&fm_id).unwrap().clone();
        inst.feedback = 7;
        mgr.update_fm_instrument(fm_id, inst).unwrap();
        assert!(!mgr.can_undo(), "instrument update is not undoable");

        cleanup(&path);
    }

    #[test]
    fn test_assign_library_voice_is_undoable_track_binding() {
        use crate::library::entry::{content_hash, LibraryInstrument};

        let path = temp_project_path("undo_assign");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Assign", "flamedriver", 120.0, (4, 4)).unwrap();
        let track_id = mgr.add_track("Empty".into(), ChannelAssignment::Fm(0), None);

        let voice = LibraryInstrument::Fm(assign_test_fm());
        let hash = content_hash(&voice);
        mgr.assign_library_instrument_to_track(track_id, &voice, &hash).unwrap();
        let t = mgr.list_tracks().iter().find(|t| t.id == track_id).unwrap();
        assert!(t.instrument_id.is_some());

        mgr.undo();
        let t = mgr.list_tracks().iter().find(|t| t.id == track_id).unwrap();
        assert_eq!(t.instrument_id, None, "undo restores the pre-assign binding");
        assert_eq!(t.name, "Empty", "undo restores the pre-assign track name");

        cleanup(&path);
    }

    #[test]
    fn test_failed_mutation_pushes_no_snapshot() {
        let (mut mgr, path, track_id, region_id) = undo_fixture("undo_failed");
        mgr.add_note(track_id, region_id, 0, 60, 100, 240).unwrap();
        mgr.save().unwrap();
        assert!(!mgr.is_dirty());
        let depth_probe = mgr.can_undo(); // history exists from setup
        assert!(depth_probe);

        // Failing calls must not push snapshots (or mark dirty).
        assert!(mgr.update_note(track_id, region_id, 99, 0, 60, 100, 240).is_err());
        assert!(mgr.delete_note(Uuid::new_v4(), region_id, 0).is_err());
        assert!(mgr.delete_region(track_id, Uuid::new_v4()).is_err());
        assert!(mgr.delete_track(Uuid::new_v4()).is_err());
        assert!(!mgr.is_dirty(), "failed mutations must not mark dirty");

        // The next undo must revert the add_note, not a phantom snapshot.
        mgr.undo();
        assert!(region_notes(&mgr, track_id, region_id).is_empty(),
            "failed mutations must not have pushed snapshots");

        cleanup(&path);
    }
}
