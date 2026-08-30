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

/// One tagged span in a channel-overlap scan: (start_tick, end_tick, tag).
/// The tag is caller-chosen — a track id for the post-hoc diagnostic
/// (`build_snapshot`'s `OverlapWarning`s), an effective voice for the
/// edit-time voice-overlap gate.
type TaggedSpan<T> = (u64, u64, T);

/// Pairwise interval-intersection sweep shared by `build_snapshot`'s
/// overlap diagnostics and the voice-overlap edit gate, so both are
/// correct by the same construction. Sorts `spans` by start tick, then
/// invokes `emit(earlier, later, conflict_start, conflict_end)` once per
/// intersecting pair, where the conflict span is the intersection
/// (later start, earlier end).
fn for_each_conflicting_span<T>(
    spans: &mut [TaggedSpan<T>],
    mut emit: impl FnMut(&TaggedSpan<T>, &TaggedSpan<T>, u64, u64),
) {
    spans.sort_by_key(|s| s.0);
    for i in 0..spans.len() {
        for j in (i + 1)..spans.len() {
            if spans[j].0 >= spans[i].1 {
                break;
            }
            let start = spans[j].0;
            let end = spans[i].1.min(spans[j].1);
            emit(&spans[i], &spans[j], start, end);
        }
    }
}

/// One authored note staged for a single hardware channel, before the merged
/// event list is emitted. `note_on` is `None` when no instrument could be
/// resolved for the note (an instrument-less track, which is how every lane a
/// fresh project seeds starts out). Such a note is INERT — see
/// `emit_channel_events`.
struct StagedNote {
    start: u64,
    end: u64,
    note_on: Option<SequencerEvent>,
}

/// Emit the merged event list for one hardware channel under LAST-NOTE
/// PRIORITY, the semantics the Memra driver actually has.
///
/// The driver has no note-off event and no note identity: channel state is
/// one `sc_note` byte plus one `SCF_KEYED` bit, and `Fm_NoteOnFreqExact`
/// force-keys-off a sounding channel before keying on (aeon `1ee8f8e6`,
/// `sound_fm.emp:1092-1099`) — gated on *keyed*, not on pitch. PSG does the
/// same through `Psg_EnvCursorReset`. So a note's effective duration is
/// `min(authored, next onset on that channel)`, and a note-off at the
/// authored end of a note that a successor already took over is a state the
/// hardware can never occupy — no serialization of the song would produce an
/// event there. Emitting it anyway keys off the SUCCESSOR (the key-off is
/// pitch-blind, correctly so), truncating it.
///
/// `notes` may come from SEVERAL author-side tracks merged onto this one
/// channel, so the successor that terminates a note is frequently not in the
/// same track. The sweep is therefore over the merged, start-sorted order.
///
/// Suppressing at `end_i == start_{i+1}` rather than emitting there is
/// audibly identical: the event sort places NoteOff before NoteOn at equal
/// ticks, so emitting gives key-off then key-on at the same tick, while
/// suppressing gives a note-on onto a still-keyed channel, which itself
/// key-offs first (`process_event`, mirroring `do_keyon`). Same two register
/// actions, same tick, no samples rendered in between.
///
/// A note that resolved no instrument is INERT: the emitted list is exactly
/// what the resolvable notes alone would produce. It emits nothing; it is not
/// counted as superseding; and it cannot hide a terminating successor from
/// the scan, because the list is start-sorted, so anything sitting behind a
/// note that starts past `end` also starts past `end`. Nothing can therefore
/// be left ringing: every emitted note is either superseded by a successor's
/// forced key-off or emits its own note-off, and the last resolvable note
/// always falls into the second case.
fn emit_channel_events(mut notes: Vec<StagedNote>) -> Vec<SequencerEvent> {
    // Stable, so notes sharing a start tick keep authoring order — the same
    // order the (also stable) event sort below gives their NoteOns, which
    // makes the surviving note-off the one belonging to the note that wins.
    notes.sort_by_key(|n| n.start);

    let mut events: Vec<SequencerEvent> = Vec::new();
    for (i, note) in notes.iter().enumerate() {
        // A note that resolved no instrument contributes NOTHING — not a
        // note-on, and not a note-off either. It keys nothing on, so a
        // note-off in its name would key off whatever the channel happens to
        // be sounding (the key-off is pitch-blind), which is the same
        // divergence last-note-priority removes, reached from the other side.
        // On hardware such a note produces no events at all: no serialization
        // would emit a lone key-off.
        let Some(ref on) = note.note_on else { continue };
        events.push(on.clone());
        let superseded = notes[i + 1..]
            .iter()
            .take_while(|next| next.start <= note.end)
            .any(|next| next.note_on.is_some());
        if !superseded {
            events.push(SequencerEvent::NoteOff { tick: note.end });
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
    events
}

/// The BTreeMap key `build_snapshot` groups tracks by — one key per output
/// channel. The voice-overlap gate groups by the same key so "same channel"
/// means the same thing in both places.
fn channel_key(channel: &ChannelAssignment) -> String {
    match channel {
        ChannelAssignment::Fm(n) => format!("fm_{n}"),
        ChannelAssignment::Psg(n) => format!("psg_{n}"),
        ChannelAssignment::PsgNoise => "psg_noise".to_string(),
        ChannelAssignment::Dac(n) => format!("dac_{n}"),
    }
}

/// Human-readable channel name for diagnostics ("FM1", "PSG2", …), matching
/// the names `build_snapshot` puts in `OverlapWarning::channel_name`.
fn channel_display_name(channel: &ChannelAssignment) -> String {
    match channel {
        ChannelAssignment::Fm(n) => format!("FM{}", n + 1),
        ChannelAssignment::Psg(n) => format!("PSG{}", n + 1),
        ChannelAssignment::PsgNoise => "PSG Noise".to_string(),
        ChannelAssignment::Dac(n) => format!("DAC{}", n + 1),
    }
}

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
                // Volume is TL-denominated (0.75 dB/step, added to carrier
                // TLs by the sequencer): 127 = no attenuation. See
                // test_default_track_volume_is_driver_faithful_full.
                volume: 127,
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

    // Kept: project-lifecycle predicate, referenced only from `#[cfg(test)]`
    // tests in this module.
    #[allow(dead_code)]
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

    /// Update the song-level tempo / time signature. Validate-first (a
    /// rejected edit changes nothing); persisted by the next `save()` since
    /// `save()` rewrites project.json from `self.metadata`.
    ///
    /// Tempo bounds mirror project creation (NewProjectDialog clamps
    /// 20-300 BPM): the sequencer itself tolerates any positive tempo
    /// (`ticks_per_sample = tempo/60 * tpb / sample_rate`), so creation's
    /// range is the binding contract, and it sits comfortably inside what
    /// SMPS export's `compute_tempo_params` search can represent.
    pub fn update_project_metadata(
        &mut self,
        tempo: f64,
        time_signature: (u8, u8),
    ) -> Result<SongMetadata, String> {
        if !tempo.is_finite() || !(20.0..=300.0).contains(&tempo) {
            return Err(format!("tempo must be between 20 and 300 BPM (got {tempo})"));
        }
        let (num, den) = time_signature;
        if !(1..=16).contains(&num) {
            return Err(format!("time signature numerator must be 1-16 (got {num})"));
        }
        if !matches!(den, 2 | 4 | 8 | 16) {
            return Err(format!("time signature denominator must be 2, 4, 8 or 16 (got {den})"));
        }
        let meta = self.metadata.as_mut().ok_or("no project open")?;
        // NOTE: metadata sits outside the undo snapshot (tracks only) — this
        // edit marks dirty but is NOT undoable in v1.
        meta.tempo = tempo;
        meta.time_signature = time_signature;
        self.dirty = true;
        Ok(self.metadata.as_ref().unwrap().clone())
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

    /// The seeded lane name for `channel` under the current project's
    /// driver — the same `ChannelLayout` source `default_tracks_for_layout`
    /// reads (never hardcoded). None when no project is open, the driver is
    /// unknown, or the channel isn't in the layout.
    fn default_lane_name(&self, channel: &ChannelAssignment) -> Option<String> {
        let driver_id = &self.metadata.as_ref()?.driver_id;
        let layout = self.driver_registry.get(driver_id)?.channel_layout();
        match channel {
            ChannelAssignment::Fm(n) => layout
                .fm_channels.iter().find(|c| c.index == *n).map(|c| c.name.clone()),
            ChannelAssignment::Psg(n) => layout
                .psg_channels.iter().find(|c| !c.is_noise && c.index == *n).map(|c| c.name.clone()),
            ChannelAssignment::PsgNoise => layout
                .psg_channels.iter().find(|c| c.is_noise).map(|c| c.name.clone()),
            ChannelAssignment::Dac(n) => layout
                .dac_channels.iter().find(|c| c.index == *n).map(|c| c.name.clone()),
        }
    }

    /// Clear `instrument_id` on any track bound to `id`. Lanes survive
    /// instrument deletion (the seeded roster is the channel layout).
    ///
    /// Binding names lanes after their instrument, so a lane still carrying
    /// the deleted instrument's name would masquerade as bound (F2c): when
    /// the lane's name equals `instrument_name` it is reset to its channel
    /// default. A name that differs is a user-custom rename and is kept.
    fn unbind_instrument_from_tracks(&mut self, id: Uuid, instrument_name: &str) {
        let bound: Vec<usize> = self.tracks.iter().enumerate()
            .filter(|(_, t)| t.instrument_id == Some(id))
            .map(|(i, _)| i)
            .collect();
        for i in bound {
            self.tracks[i].instrument_id = None;
            if self.tracks[i].name == instrument_name {
                let channel = self.tracks[i].channel.clone();
                if let Some(default) = self.default_lane_name(&channel) {
                    self.tracks[i].name = default;
                }
            }
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
        let inst = self.instruments.fm.remove(pos);
        self.dirty_instruments.remove(&id);
        self.dirty = true;
        self.unbind_instrument_from_tracks(id, &inst.name);
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
        let inst = self.instruments.psg.remove(pos);
        self.dirty_instruments.remove(&id);
        self.dirty = true;
        self.unbind_instrument_from_tracks(id, &inst.name);
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
        use crate::library::entry::LibraryInstrument;

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

        let (id, name) = self.ensure_library_instrument_in_bank(inst, hash);

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

    /// Add-or-reuse a library voice in the project's instrument bank
    /// WITHOUT touching any track: a project instrument of the same kind
    /// whose content hash equals `hash` is reused; otherwise the library
    /// instrument is added with a fresh id. Returns (id, name). No undo
    /// snapshot — instrument operations are out of undo scope — but an
    /// added instrument marks the project dirty. Backs both the
    /// drag-to-track swap and the piano-roll note-voice drop
    /// (`set_note_instrument` wants a project instrument id).
    pub fn ensure_library_instrument_in_bank(
        &mut self,
        inst: &crate::library::entry::LibraryInstrument,
        hash: &str,
    ) -> (Uuid, String) {
        use crate::library::entry::{content_hash, LibraryInstrument};

        let existing = match inst {
            LibraryInstrument::Fm(_) => self.instruments.fm.iter()
                .find(|i| content_hash(&LibraryInstrument::Fm((*i).clone())) == hash)
                .map(|i| (i.id, i.name.clone())),
            LibraryInstrument::Psg(_) => self.instruments.psg.iter()
                .find(|i| content_hash(&LibraryInstrument::Psg((*i).clone())) == hash)
                .map(|i| (i.id, i.name.clone())),
        };

        match existing {
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
                self.dirty = true;
                (id, name)
            }
        }
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
        self.unbind_instrument_from_tracks(id, &inst.name);
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
            channel_map.entry(channel_key(&track.channel)).or_default().push(track);
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

            let mut staged: Vec<StagedNote> = Vec::new();
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
                        let note_on = note_inst.as_ref().map(|data| {
                            let note_mod = if let Some(ref m) = note.modulation {
                                Some(ModulationParams { wait: m.wait, speed: m.speed, delta: m.delta, steps: m.steps })
                            } else if let Some(ref m) = track.modulation {
                                Some(ModulationParams { wait: m.wait, speed: m.speed, delta: m.delta, steps: m.steps })
                            } else {
                                None
                            };
                            SequencerEvent::NoteOn {
                                tick: abs_tick,
                                pitch: pitched,
                                velocity: note.velocity,
                                detune: note.detune,
                                duration_ticks: note.duration_ticks,
                                instrument: data.clone(),
                                modulation: note_mod,
                                pan_override: note.pan_override,
                            }
                        });
                        staged.push(StagedNote { start: abs_tick, end: end_tick, note_on });
                        overlap_sources.push((abs_tick, end_tick, track.id.to_string()));
                    }
                }
            }

            let events = emit_channel_events(staged);

            // The overlap diagnostics are UNCHANGED by last-note-priority.
            // The ambiguity is real and still the author's to resolve; it now
            // resolves at compile time (deterministically, the way the driver
            // resolves it) instead of at playback, which is a reason to keep
            // surfacing it, not to stop.
            let mut overlaps = Vec::new();
            for_each_conflicting_span(&mut overlap_sources, |earlier, later, start, end| {
                overlaps.push(OverlapWarning {
                    channel_name: channel_display_name(&tracks[0].channel),
                    tick_start: start,
                    tick_end: end,
                    track_ids: vec![earlier.2.clone(), later.2.clone()],
                });
            });

            let volume = tracks[0].volume;
            let pan_byte = match tracks[0].pan {
                crate::model::song::Pan::Left => 0x80u8,
                crate::model::song::Pan::Right => 0x40,
                crate::model::song::Pan::Center => 0xC0,
            };
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
            // TL-denominated: 127 = no attenuation (see
            // test_default_track_volume_is_driver_faithful_full).
            volume: 127,
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

    /// One track's instrument binding — the ONE field the piano roll's
    /// audition path reads out of the track list (F26). Same answer
    /// `list_tracks().iter().find(...).and_then(|t| t.instrument_id)` gives;
    /// exists so the interactive path can ask for it without serializing
    /// every region and note in the song across IPC. `None` for an unknown
    /// track id as well as for an unbound one — both mean "this audition has
    /// nothing to play".
    pub fn track_instrument_id(&self, track_id: Uuid) -> Option<Uuid> {
        self.tracks
            .iter()
            .find(|t| t.id == track_id)
            .and_then(|t| t.instrument_id)
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
        instrument_id: Option<Uuid>,
    ) -> Result<usize, String> {
        let t_idx = self.tracks.iter().position(|t| t.id == track_id)
            .ok_or("track not found")?;
        let r_idx = self.tracks[t_idx].regions.iter().position(|r| r.id == region_id)
            .ok_or("region not found")?;
        // An EXPLICIT per-note voice is validated like set_note_instrument
        // (kind gate, then voice-overlap gate). `None` keeps the historical
        // behavior for every existing caller: no gates, note inherits
        // region/track voice.
        if let Some(inst_id) = instrument_id {
            self.check_instrument_kind(inst_id, t_idx, "add a note with")?;
            let region = &self.tracks[t_idx].regions[r_idx];
            let span = (region.start_tick + tick, region.start_tick + tick + duration_ticks);
            self.check_voice_overlap(t_idx, r_idx, &HashSet::new(), Some(inst_id), Some(span))?;
        }
        self.record_song_edit();
        let region = &mut self.tracks[t_idx].regions[r_idx];
        let idx = region.notes.len();
        region.notes.push(Note { tick, pitch, velocity, duration_ticks, instrument_id, detune: 0, pan_override: None, modulation: None });
        Ok(idx)
    }

    /// Set (or clear, with `None`) the per-note voice override on a batch of
    /// notes in one region. One undoable edit for the whole batch.
    ///
    /// Validate-first, in order: track/region/indices exist; the instrument
    /// (when `Some`) exists and its kind matches the track's channel kind
    /// (FM voice ↔ FM channel, PSG envelope ↔ PSG channel, DAC ↔ DAC — the
    /// same gate as `assign_library_instrument_to_track`); then the
    /// voice-overlap gate. Only then is the edit recorded and applied.
    /// `None` clears the override so the notes fall back to the
    /// region/track default (note > region > track precedence).
    pub fn set_note_instrument(
        &mut self,
        track_id: Uuid,
        region_id: Uuid,
        note_indices: &[usize],
        instrument_id: Option<Uuid>,
    ) -> Result<(), String> {
        let t_idx = self.tracks.iter().position(|t| t.id == track_id)
            .ok_or("track not found")?;
        let r_idx = self.tracks[t_idx].regions.iter().position(|r| r.id == region_id)
            .ok_or("region not found")?;
        if note_indices.is_empty() {
            return Err("no notes selected".into());
        }
        let notes_len = self.tracks[t_idx].regions[r_idx].notes.len();
        if let Some(&bad) = note_indices.iter().find(|&&i| i >= notes_len) {
            return Err(format!("note index {bad} out of range"));
        }
        if let Some(inst_id) = instrument_id {
            self.check_instrument_kind(inst_id, t_idx, "set")?;
        }
        let edited: HashSet<usize> = note_indices.iter().copied().collect();
        self.check_voice_overlap(t_idx, r_idx, &edited, instrument_id, None)?;
        self.record_song_edit();
        let region = &mut self.tracks[t_idx].regions[r_idx];
        for &i in note_indices {
            region.notes[i].instrument_id = instrument_id;
        }
        Ok(())
    }

    /// Kind gate shared by `set_note_instrument` and voiced `add_note`:
    /// the instrument must exist in the bank, and its kind must match the
    /// track's channel kind — mirroring
    /// `assign_library_instrument_to_track`'s gate (FM voice ↔ FM channel,
    /// PSG envelope ↔ PSG/noise channel, DAC sample ↔ DAC channel).
    fn check_instrument_kind(&self, inst_id: Uuid, t_idx: usize, verb: &str) -> Result<(), String> {
        let inst_kind = if self.instruments.fm.iter().any(|i| i.id == inst_id) {
            "FM"
        } else if self.instruments.psg.iter().any(|i| i.id == inst_id) {
            "PSG"
        } else if self.instruments.dac.iter().any(|i| i.id == inst_id) {
            "DAC"
        } else {
            return Err("instrument not found".into());
        };
        let track_kind = match self.tracks[t_idx].channel {
            ChannelAssignment::Fm(_) => "FM",
            ChannelAssignment::Psg(_) | ChannelAssignment::PsgNoise => "PSG",
            ChannelAssignment::Dac(_) => "DAC",
        };
        if inst_kind != track_kind {
            return Err(format!(
                "cannot {verb} an {inst_kind} voice on a non-{inst_kind} track"
            ));
        }
        Ok(())
    }

    /// Voice-overlap gate (named rule: "voice-overlap"). Rejects an edit
    /// that would leave an EDITED note overlapping, on the same output
    /// channel (`channel_key` — the grouping `build_snapshot` uses), a note
    /// whose effective voice (note > region > track) is a DIFFERENT
    /// concrete instrument. Same-voice overlaps keep today's status quo
    /// (allowed here, surfaced post-hoc by `get_channel_overlaps`), and a
    /// note with no effective voice is silent, so only Some-vs-Some
    /// disagreements are conflicts. Pre-existing conflicts between
    /// untouched notes (imported projects) do not block unrelated edits.
    ///
    /// The hypothetical edit: notes at `edited_indices` of
    /// (`t_idx`, `r_idx`) take `new_voice` as their note-level override
    /// (falling back region > track when `None`); `extra_span`, when given,
    /// is a not-yet-inserted note (add_note) carrying `new_voice`.
    fn check_voice_overlap(
        &self,
        t_idx: usize,
        r_idx: usize,
        edited_indices: &HashSet<usize>,
        new_voice: Option<Uuid>,
        extra_span: Option<(u64, u64)>,
    ) -> Result<(), String> {
        let target = &self.tracks[t_idx];
        let key = channel_key(&target.channel);
        let target_region_id = target.regions[r_idx].id;

        // (start, end, (effective_voice, is_edited)) for every note on the
        // channel, with the hypothetical edit applied.
        let mut spans: Vec<TaggedSpan<(Option<Uuid>, bool)>> = Vec::new();
        for track in self.tracks.iter().filter(|t| channel_key(&t.channel) == key) {
            for region in &track.regions {
                let is_target_region = track.id == target.id && region.id == target_region_id;
                for (i, note) in region.notes.iter().enumerate() {
                    let is_edited = is_target_region && edited_indices.contains(&i);
                    let note_voice = if is_edited { new_voice } else { note.instrument_id };
                    let effective = note_voice
                        .or(region.instrument_id)
                        .or(track.instrument_id);
                    let start = region.start_tick + note.tick;
                    spans.push((start, start + note.duration_ticks, (effective, is_edited)));
                }
            }
        }
        if let Some((start, end)) = extra_span {
            let effective = new_voice
                .or(target.regions[r_idx].instrument_id)
                .or(target.instrument_id);
            spans.push((start, end, (effective, true)));
        }

        let mut conflict: Option<(u64, u64)> = None;
        for_each_conflicting_span(&mut spans, |a, b, start, end| {
            if conflict.is_some() {
                return;
            }
            let (voice_a, edited_a) = &a.2;
            let (voice_b, edited_b) = &b.2;
            if !(edited_a | edited_b) {
                return;
            }
            if let (Some(va), Some(vb)) = (voice_a, voice_b) {
                if va != vb {
                    conflict = Some((start, end));
                }
            }
        });
        match conflict {
            Some((start, end)) => Err(format!(
                "voice-overlap: notes with different voices would overlap on {} at ticks {start}-{end}",
                channel_display_name(&target.channel)
            )),
            None => Ok(()),
        }
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
        crate::driver::default_registry()
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

    /// Track volume is TL-denominated (the sequencer adds `127 - volume`
    /// straight to every FM carrier TL, 0.75 dB per step; SMPS import's
    /// `fm_effective_velocity` and the audition path share that convention).
    /// 127 is therefore the only "no attenuation" default. A default of 100
    /// silently costs ~20 dB per control; stacked with the piano roll's note
    /// velocity default it rendered hand-placed FM notes ~35 dB quieter than
    /// audition (measured in `audio::rendered_rms` — the "Animal Boss voice
    /// 151 is extremely quiet" report).
    #[test]
    fn test_default_track_volume_is_driver_faithful_full() {
        let path = temp_project_path("default_volume");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Vol", "flamedriver", 120.0, (4, 4)).unwrap();

        for track in mgr.list_tracks() {
            assert_eq!(
                track.volume, 127,
                "seeded track '{}' must default to driver-faithful full volume",
                track.name
            );
        }

        let id = mgr.add_track("Extra".into(), ChannelAssignment::Fm(1), None);
        let track = mgr.list_tracks().into_iter().find(|t| t.id == id).unwrap();
        assert_eq!(track.volume, 127, "add_track must default to full volume");

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

    /// Deleting an instrument unbinds its lane — and must also reset the
    /// lane's name to the channel default (derived from the driver layout,
    /// the same source `default_tracks_for_layout` reads), or the dead
    /// instrument's name keeps masquerading as a live binding (F2c).
    #[test]
    fn test_delete_instrument_resets_lane_name_to_channel_default() {
        use crate::model::driver::DriverProfile as _;

        let path = temp_project_path("unbind_name_reset");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Unbind", "flamedriver", 120.0, (4, 4)).unwrap();

        let id = mgr.add_fm_instrument(assign_test_fm());
        let track_id = mgr.list_tracks().iter()
            .find(|t| t.instrument_id == Some(id))
            .map(|t| t.id)
            .expect("instrument bound to a lane");
        // Binding renamed the lane to the instrument.
        assert_eq!(
            mgr.list_tracks().iter().find(|t| t.id == track_id).unwrap().name,
            "Library Lead"
        );

        mgr.delete_fm_instrument(id).unwrap();

        let track = mgr.list_tracks().iter().find(|t| t.id == track_id).unwrap();
        assert_eq!(track.instrument_id, None);
        // Expected name comes from the driver's own layout — never hardcoded.
        let layout = FlamedriverProfile.channel_layout();
        let default_name = match track.channel {
            ChannelAssignment::Fm(n) => layout.fm_channels.iter()
                .find(|c| c.index == n).map(|c| c.name.clone()),
            _ => None,
        }.expect("lane channel present in the driver layout");
        assert_eq!(
            track.name, default_name,
            "unbound lane must fall back to its channel-default name, not keep the dead instrument's"
        );

        cleanup(&path);
    }

    /// A lane the USER renamed after binding keeps its custom name on
    /// instrument delete — only the binding-convention name (lane named
    /// exactly after the instrument) is reset (F2c).
    #[test]
    fn test_delete_instrument_preserves_user_renamed_lane() {
        let path = temp_project_path("unbind_name_custom");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Unbind", "flamedriver", 120.0, (4, 4)).unwrap();

        let id = mgr.add_fm_instrument(assign_test_fm());
        let track = mgr.list_tracks().iter()
            .find(|t| t.instrument_id == Some(id))
            .cloned()
            .expect("instrument bound to a lane");
        mgr.update_track(
            track.id, "My Custom Bass".into(), track.channel.clone(), Some(id),
            track.muted, track.solo, track.volume, track.pan.clone(), track.pitch_offset,
        ).unwrap();

        mgr.delete_fm_instrument(id).unwrap();

        let after = mgr.list_tracks().iter().find(|t| t.id == track.id).unwrap();
        assert_eq!(after.instrument_id, None);
        assert_eq!(after.name, "My Custom Bass", "user rename must survive the unbind");

        cleanup(&path);
    }

    #[test]
    fn test_update_project_metadata_validates_marks_dirty_and_persists() {
        let path = temp_project_path("update_meta");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Meta", "flamedriver", 120.0, (4, 4)).unwrap();
        assert!(!mgr.is_dirty(), "fresh project starts clean");

        // Valid update: applied, returned, dirty.
        let meta = mgr.update_project_metadata(150.0, (3, 8)).unwrap();
        assert_eq!(meta.tempo, 150.0);
        assert_eq!(meta.time_signature, (3, 8));
        assert_eq!(mgr.metadata().unwrap().tempo, 150.0);
        assert!(mgr.is_dirty(), "metadata edit marks the project dirty");

        // Persisted through save → project.json → reopen.
        mgr.save().unwrap();
        let mut mgr2 = ProjectManager::new(test_registry());
        let song = mgr2.open(&path).unwrap();
        assert_eq!(song.metadata.tempo, 150.0);
        assert_eq!(song.metadata.time_signature, (3, 8));

        cleanup(&path);
    }

    #[test]
    fn test_update_project_metadata_rejects_invalid_values() {
        let path = temp_project_path("update_meta_invalid");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Meta", "flamedriver", 120.0, (4, 4)).unwrap();

        // Tempo bounds mirror project creation (NewProjectDialog: 20-300).
        assert!(mgr.update_project_metadata(19.9, (4, 4)).is_err());
        assert!(mgr.update_project_metadata(300.1, (4, 4)).is_err());
        assert!(mgr.update_project_metadata(f64::NAN, (4, 4)).is_err());
        // Numerator 1-16.
        assert!(mgr.update_project_metadata(120.0, (0, 4)).is_err());
        assert!(mgr.update_project_metadata(120.0, (17, 4)).is_err());
        // Denominator 2/4/8/16 only.
        assert!(mgr.update_project_metadata(120.0, (4, 3)).is_err());
        assert!(mgr.update_project_metadata(120.0, (4, 32)).is_err());

        // Validate-first: a rejected edit changes nothing and stays clean.
        assert_eq!(mgr.metadata().unwrap().tempo, 120.0);
        assert_eq!(mgr.metadata().unwrap().time_signature, (4, 4));
        assert!(!mgr.is_dirty(), "rejected edit must not mark dirty");

        // Boundary values are accepted.
        assert!(mgr.update_project_metadata(20.0, (1, 2)).is_ok());
        assert!(mgr.update_project_metadata(300.0, (16, 16)).is_ok());

        cleanup(&path);
    }

    #[test]
    fn test_update_project_metadata_requires_open_project() {
        let mut mgr = ProjectManager::new(test_registry());
        assert!(mgr.update_project_metadata(120.0, (4, 4)).is_err());
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
        mgr.add_note(track_id, region_id, 0, 60, 100, 240, None).unwrap();
        mgr.add_note(track_id, region_id, 480, 64, 80, 480, None).unwrap();

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
        mgr.add_note(track_id, region_id, 0, 60, 100, 240, None).unwrap();

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

        let idx = mgr.add_note(track_id, region_id, 0, 60, 100, 240, None).unwrap();
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
        mgr.add_note(track_id, region_id, 0, 60, 100, 480, None).unwrap();
        mgr.add_note(track_id, region_id, 480, 64, 80, 240, None).unwrap();

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

    /// Boundary shapes of the last-note-priority suppression, stated directly
    /// on `emit_channel_events` so each case is visible as an event list.
    /// The AUDIBLE consequences are asserted on rendered samples in
    /// `audio::overlap_audibility`; this pins the boundaries those renders
    /// cannot each afford a separate song for.
    #[test]
    fn test_emit_channel_events_last_note_priority_boundaries() {
        fn staged(start: u64, end: u64) -> StagedNote {
            StagedNote {
                start,
                end,
                note_on: Some(SequencerEvent::NoteOn {
                    tick: start,
                    pitch: 60,
                    velocity: 100,
                    detune: 0,
                    duration_ticks: end - start,
                    instrument: InstrumentData::FmPatch { bytes: [0; 25], ssg_eg: [0; 4] },
                    modulation: None,
                    pan_override: None,
                }),
            }
        }
        fn shape(events: &[SequencerEvent]) -> Vec<(u64, bool)> {
            events
                .iter()
                .map(|e| (e.tick(), matches!(e, SequencerEvent::NoteOn { .. })))
                .collect()
        }

        // Gap: nothing takes the channel over, so both note-offs survive.
        assert_eq!(
            shape(&emit_channel_events(vec![staged(0, 480), staged(960, 1440)])),
            vec![(0, true), (480, false), (960, true), (1440, false)],
            "a note-off with no successor to replace it must still be emitted"
        );

        // Overlap: the successor's note-on terminates the first note, so the
        // first note's off — an event no serialization would ever produce —
        // is suppressed.
        assert_eq!(
            shape(&emit_channel_events(vec![staged(0, 480), staged(240, 720)])),
            vec![(0, true), (240, true), (720, false)],
            "an overlapped note-off must not survive to key off its successor"
        );

        // Abutting (end_i == start_{i+1}): suppressed. Emitting there would
        // give key-off-then-key-on at the same tick; suppressing gives a
        // note-on onto a still-keyed channel, which key-offs first itself.
        // Same two actions, same tick, no samples rendered in between.
        assert_eq!(
            shape(&emit_channel_events(vec![staged(0, 480), staged(480, 960)])),
            vec![(0, true), (480, true), (960, false)],
            "abutting notes resolve through the successor's own forced key-off"
        );

        // --- notes that resolved no instrument are INERT ---
        //
        // The emitted list must be exactly what the resolvable notes alone
        // would produce. Two properties carry that, and both are tested:
        // an unresolved note emits nothing, and it cannot change any other
        // note's outcome.
        fn unvoiced(start: u64, end: u64) -> StagedNote {
            StagedNote { start, end, note_on: None }
        }

        // It emits no note-on, so it cannot take the channel over: the
        // predecessor's own off still has to fire. And it emits no note-off
        // of its own.
        assert_eq!(
            shape(&emit_channel_events(vec![staged(0, 480), unvoiced(240, 720)])),
            vec![(0, true), (480, false)],
            "a successor that keys nothing on can neither terminate its predecessor \
             nor emit an off of its own"
        );

        // The reachable break: an unresolved note ENDING INSIDE a sounding
        // note. Its off would be pitch-blind and would cut a note it does not
        // own — the divergence this whole parcel removes, from the other side.
        assert_eq!(
            shape(&emit_channel_events(vec![staged(0, 960), unvoiced(240, 480)])),
            vec![(0, true), (960, false)],
            "an instrument-less note must not key off the note that is sounding"
        );

        // Load-bearing for the inertness argument: an unresolved note sitting
        // BETWEEN a note and the successor that terminates it must not hide
        // that successor from the scan. (It cannot — the list is start-sorted,
        // so anything after a note starting past `end` also starts past it —
        // but the suppression would silently over-emit if that ever changed.)
        assert_eq!(
            shape(&emit_channel_events(vec![
                staged(0, 480),
                unvoiced(200, 300),
                staged(240, 720),
            ])),
            vec![(0, true), (240, true), (720, false)],
            "an interleaved unresolved note must not hide the terminating successor"
        );

        // Stated as the invariant itself: dropping the unresolved notes from
        // the input cannot change the output.
        let with_unresolved = vec![
            unvoiced(0, 1200),
            staged(0, 480),
            unvoiced(200, 300),
            staged(240, 720),
            unvoiced(700, 780),
            staged(960, 1440),
        ];
        let without: Vec<StagedNote> = vec![staged(0, 480), staged(240, 720), staged(960, 1440)];
        assert_eq!(
            shape(&emit_channel_events(with_unresolved)),
            shape(&emit_channel_events(without)),
            "unresolved notes must be inert — the event list is the last-note-priority \
             list over the resolvable notes alone"
        );
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

        mgr.add_note(track_id, region_id, 0, 60, 100, 240, None).unwrap();
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
        mgr.add_note(track_id, region_id, 0, 60, 100, 240, None).unwrap();

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
        assert_eq!(t.volume, 127, "undo restores the (driver-faithful) default volume");
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
        mgr.add_note(track_id, region_id, 0, 60, 100, 240, None).unwrap();

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
        mgr.add_note(track_id, region_id, 0, 60, 100, 240, None).unwrap();

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
        mgr.add_note(track_id, region_id, 0, 60, 100, 240, None).unwrap();
        mgr.undo();
        assert!(mgr.can_redo());

        // A fresh mutation invalidates the redo branch.
        mgr.add_note(track_id, region_id, 480, 72, 100, 240, None).unwrap();
        assert!(!mgr.can_redo(), "new mutation must clear the redo stack");
        let before = region_notes(&mgr, track_id, region_id);
        mgr.redo(); // must be a no-op
        assert_eq!(region_notes(&mgr, track_id, region_id).len(), before.len());

        cleanup(&path);
    }

    #[test]
    fn test_undo_stack_cap_enforced() {
        let (mut mgr, path, track_id, region_id) = undo_fixture("undo_cap");
        mgr.add_note(track_id, region_id, 0, 60, 100, 240, None).unwrap();

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
        mgr.add_note(track_id, region_id, 0, 60, 100, 240, None).unwrap();
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
        mgr.add_note(track_id, region_id, 0, 60, 100, 240, None).unwrap();
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

    // --- Per-note voice (set_note_instrument / voiced add_note) ---

    /// Fresh project with two FM voices in the bank. Voice A is bound (by
    /// `add_fm_instrument`'s lane binding) to the seeded FM1 lane, which
    /// gets one empty region. Returns
    /// (mgr, path, track_id, region_id, voice_a, voice_b).
    fn voice_fixture(name: &str) -> (ProjectManager, PathBuf, Uuid, Uuid, Uuid, Uuid) {
        let path = temp_project_path(name);
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Voices", "flamedriver", 120.0, (4, 4)).unwrap();
        let mut a = assign_test_fm();
        a.name = "Voice A".into();
        let voice_a = mgr.add_fm_instrument(a);
        let mut b = assign_test_fm();
        b.name = "Voice B".into();
        b.algorithm = 2;
        let voice_b = mgr.add_fm_instrument(b);
        let track_id = mgr.tracks.iter()
            .find(|t| t.instrument_id == Some(voice_a))
            .expect("voice A bound to a lane").id;
        let region_id = mgr.add_region(track_id, 0, 1920).unwrap();
        (mgr, path, track_id, region_id, voice_a, voice_b)
    }

    /// F26: the narrow read the audition path uses must answer EXACTLY what
    /// the fat `list_tracks` read it replaces answered — at every point the
    /// binding can change, not just at rest. The expectation is DERIVED from
    /// `list_tracks` on each line rather than written out, so the two cannot
    /// drift apart without this failing.
    #[test]
    fn test_track_instrument_id_agrees_with_list_tracks_through_every_rebind() {
        let (mut mgr, path, track_id, _region_id, voice_a, voice_b) =
            voice_fixture("track_instrument_narrow_read");

        /// What the frontend used to compute out of the whole track list.
        fn via_list_tracks(mgr: &ProjectManager, track_id: Uuid) -> Option<Uuid> {
            mgr.list_tracks()
                .iter()
                .find(|t| t.id == track_id)
                .and_then(|t| t.instrument_id)
        }

        // Bound at rest (the fixture binds voice A to this lane).
        assert_eq!(mgr.track_instrument_id(track_id), via_list_tracks(&mgr, track_id));
        assert_eq!(mgr.track_instrument_id(track_id), Some(voice_a));

        // Rebound to another voice — the case a cached id would get wrong.
        let t = mgr.tracks.iter().position(|t| t.id == track_id).unwrap();
        mgr.tracks[t].instrument_id = Some(voice_b);
        assert_eq!(mgr.track_instrument_id(track_id), via_list_tracks(&mgr, track_id));
        assert_eq!(mgr.track_instrument_id(track_id), Some(voice_b));

        // Unbound — the silent-lane state.
        mgr.tracks[t].instrument_id = None;
        assert_eq!(mgr.track_instrument_id(track_id), via_list_tracks(&mgr, track_id));
        assert_eq!(mgr.track_instrument_id(track_id), None);

        // Deleted out from under an open piano roll: "nothing to play", not
        // an error and not a stale id.
        mgr.tracks[t].instrument_id = Some(voice_a);
        mgr.delete_track(track_id).unwrap();
        assert_eq!(mgr.track_instrument_id(track_id), via_list_tracks(&mgr, track_id));
        assert_eq!(mgr.track_instrument_id(track_id), None);

        // An id that never existed is also None, never a panic.
        assert_eq!(mgr.track_instrument_id(Uuid::new_v4()), None);

        cleanup(&path);
    }

    #[test]
    fn test_set_note_instrument_stamps_batch_clears_and_is_one_undo_step() {
        let (mut mgr, path, track_id, region_id, _a, voice_b) = voice_fixture("voice_set");
        // Non-overlapping notes: the gate must not interfere.
        mgr.add_note(track_id, region_id, 0, 60, 100, 240, None).unwrap();
        mgr.add_note(track_id, region_id, 240, 62, 100, 240, None).unwrap();
        mgr.add_note(track_id, region_id, 480, 64, 100, 240, None).unwrap();

        mgr.set_note_instrument(track_id, region_id, &[0, 2], Some(voice_b)).unwrap();
        let notes = region_notes(&mgr, track_id, region_id);
        assert_eq!(notes[0].instrument_id, Some(voice_b));
        assert_eq!(notes[1].instrument_id, None, "unselected note untouched");
        assert_eq!(notes[2].instrument_id, Some(voice_b));

        // The whole batch is ONE undo step.
        mgr.undo();
        let notes = region_notes(&mgr, track_id, region_id);
        assert!(notes.iter().all(|n| n.instrument_id.is_none()),
            "a single undo reverts the whole batch");
        mgr.redo();

        // None clears back to the track/region default.
        mgr.set_note_instrument(track_id, region_id, &[0, 2], None).unwrap();
        let notes = region_notes(&mgr, track_id, region_id);
        assert!(notes.iter().all(|n| n.instrument_id.is_none()),
            "None clears the per-note override");

        cleanup(&path);
    }

    #[test]
    fn test_set_note_instrument_kind_gate() {
        let (mut mgr, path, _t, _r, _a, voice_b) = voice_fixture("voice_kind");
        let psg_track = mgr.add_track("PSG".into(), ChannelAssignment::Psg(0), None);
        let psg_region = mgr.add_region(psg_track, 0, 1920).unwrap();
        mgr.add_note(psg_track, psg_region, 0, 60, 100, 240, None).unwrap();
        mgr.save().unwrap();
        assert!(!mgr.is_dirty());

        // FM voice on a PSG track: rejected by the kind gate.
        let err = mgr.set_note_instrument(psg_track, psg_region, &[0], Some(voice_b)).unwrap_err();
        assert!(err.contains("FM voice"), "error names the kind mismatch: {err}");
        // Unknown instrument id: rejected before any mutation.
        let err = mgr.set_note_instrument(psg_track, psg_region, &[0], Some(Uuid::new_v4())).unwrap_err();
        assert!(err.contains("instrument not found"), "unknown id named: {err}");
        // Out-of-range index: rejected.
        let err = mgr.set_note_instrument(psg_track, psg_region, &[5], None).unwrap_err();
        assert!(err.contains("out of range"), "bad index named: {err}");

        assert!(!mgr.is_dirty(), "failed set_note_instrument must not mark dirty");
        let notes = region_notes(&mgr, psg_track, psg_region);
        assert_eq!(notes[0].instrument_id, None, "note untouched after rejections");

        cleanup(&path);
    }

    #[test]
    fn test_set_note_instrument_voice_overlap_gate() {
        let (mut mgr, path, track_id, region_id, voice_a, voice_b) = voice_fixture("voice_overlap");
        // Two overlapping notes on the same channel (0-480 and 240-720),
        // both inheriting the track voice (A) — today's allowed status quo.
        mgr.add_note(track_id, region_id, 0, 60, 100, 480, None).unwrap();
        mgr.add_note(track_id, region_id, 240, 64, 100, 480, None).unwrap();
        mgr.save().unwrap();

        // Giving ONE of them a different voice would put A and B on one
        // channel at ticks 240-480 — rejected by the named rule.
        let err = mgr.set_note_instrument(track_id, region_id, &[1], Some(voice_b)).unwrap_err();
        assert!(err.starts_with("voice-overlap:"), "rule is named: {err}");
        assert!(err.contains("FM1"), "channel is named: {err}");
        assert!(err.contains("240"), "conflict span start is reported: {err}");
        assert!(!mgr.is_dirty(), "rejected edit must not mark dirty");
        assert!(region_notes(&mgr, track_id, region_id).iter().all(|n| n.instrument_id.is_none()));

        // Same-voice overlap keeps the status quo: BOTH notes to B is fine…
        mgr.set_note_instrument(track_id, region_id, &[0, 1], Some(voice_b)).unwrap();
        // …and so is an override equal to the other note's effective voice.
        mgr.set_note_instrument(track_id, region_id, &[0, 1], None).unwrap();
        mgr.set_note_instrument(track_id, region_id, &[1], Some(voice_a)).unwrap();
        let notes = region_notes(&mgr, track_id, region_id);
        assert_eq!(notes[1].instrument_id, Some(voice_a));

        cleanup(&path);
    }

    #[test]
    fn test_add_note_carries_voice_and_gates_overlap() {
        let (mut mgr, path, track_id, region_id, _a, voice_b) = voice_fixture("voice_add");
        // Explicit voice is stored on the new note.
        let idx = mgr.add_note(track_id, region_id, 0, 60, 100, 480, Some(voice_b)).unwrap();
        assert_eq!(region_notes(&mgr, track_id, region_id)[idx].instrument_id, Some(voice_b));

        // An overlapping note inheriting the track voice (A ≠ B) is only
        // gated when it CARRIES an explicit voice: None keeps today's
        // behavior (post-hoc overlap warning, no rejection)…
        mgr.add_note(track_id, region_id, 240, 64, 100, 480, None).unwrap();
        // …but an explicit differing voice in the same span is rejected.
        let err = mgr.add_note(track_id, region_id, 700, 65, 100, 480, Some(voice_b)).unwrap_err();
        assert!(err.starts_with("voice-overlap:"), "rule is named: {err}");
        // Same explicit voice overlapping the voice-B note (0-120, clear of
        // the voice-A note at 240) is allowed.
        mgr.add_note(track_id, region_id, 0, 66, 100, 120, Some(voice_b)).unwrap();

        cleanup(&path);
    }
}
