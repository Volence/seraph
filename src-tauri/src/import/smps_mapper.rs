use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

use crate::model::driver::DriverProfile;
use crate::model::instrument::*;
use crate::model::song::*;
use crate::import::ImportWarning;
use crate::import::psg_envelopes;
use crate::import::smps_parser::*;

pub struct MappedSong {
    pub song: Song,
    pub warnings: Vec<ImportWarning>,
    pub dac_pcm: HashMap<Uuid, Vec<u8>>,
}

const SMPS_TICKS_PER_BEAT: f64 = 24.0;
const DAW_TICKS_PER_BEAT: u32 = 480;

pub fn map_smps_to_song(smps: &SmpsFile, driver: &dyn DriverProfile) -> Result<MappedSong, String> {
    map_smps_to_song_with_dac(smps, driver, None)
}

pub fn map_smps_to_song_with_dac(smps: &SmpsFile, driver: &dyn DriverProfile, dac_dir: Option<&Path>) -> Result<MappedSong, String> {
    let mut warnings = Vec::new();
    let daw_per_smps = DAW_TICKS_PER_BEAT as f64 / SMPS_TICKS_PER_BEAT;

    let divider = smps.tempo_divider.max(1) as f64;
    let smps_ticks_per_sec = (60.0 / divider) * (256.0 - smps.tempo_modifier as f64) / 256.0;
    let bpm = smps_ticks_per_sec * 60.0 / SMPS_TICKS_PER_BEAT;

    let mut instruments = InstrumentBank::default();
    let mut voice_to_fm_id: HashMap<u8, Uuid> = HashMap::new();

    let voices = if smps.voices.is_empty() && matches!(smps.voice_ref, VoiceRef::Uvb) {
        static UVB_SOURCE: &str = include_str!("../../test_data/UniBank.asm");
        match parse_voice_bank(UVB_SOURCE) {
            Ok(v) => v,
            Err(e) => {
                warnings.push(ImportWarning {
                    channel: String::new(),
                    message: format!("UVB parse error: {e}"),
                });
                vec![]
            }
        }
    } else {
        smps.voices.clone()
    };

    for (i, voice_bytes) in voices.iter().enumerate() {
        match driver.fm_from_bytes(voice_bytes) {
            Ok(mut inst) => {
                inst.name = format!("Voice {}", i);
                inst.metadata.category = "Imported".into();
                voice_to_fm_id.insert(i as u8, inst.id);
                instruments.fm.push(inst);
            }
            Err(e) => {
                warnings.push(ImportWarning {
                    channel: String::new(),
                    message: format!("voice {} parse error: {}", i, e),
                });
            }
        }
    }

    let mut psg_env_to_id: HashMap<u8, Uuid> = HashMap::new();
    let mut dac_sample_to_id: HashMap<u8, Uuid> = HashMap::new();

    let mut tracks = Vec::new();

    let mut fm_idx = 0u8;
    let mut psg_idx = 0u8;

    for ch in &smps.channels {
        let (channel_assign, track_name) = match ch.kind {
            SmpsChannelKind::Dac => (ChannelAssignment::Dac(0), "DAC".into()),
            SmpsChannelKind::Fm => {
                let idx = fm_idx;
                fm_idx += 1;
                (ChannelAssignment::Fm(idx), format!("FM{}", idx + 1))
            }
            SmpsChannelKind::Psg => {
                let idx = psg_idx;
                psg_idx += 1;
                if idx == 2 {
                    (ChannelAssignment::PsgNoise, "PSG3 (Noise)".into())
                } else {
                    (ChannelAssignment::Psg(idx), format!("PSG{}", idx + 1))
                }
            }
        };

        let pan = match ch.kind {
            SmpsChannelKind::Psg => Pan::Center,
            SmpsChannelKind::Fm | SmpsChannelKind::Dac => {
                ch.events.iter().find_map(|e| match e {
                    SmpsEvent::SetPan(p) => Some(pan_from_byte(*p)),
                    _ => None,
                }).unwrap_or(Pan::Center)
            }
        };

        let modulation = ch.events.iter().find_map(|e| match e {
            SmpsEvent::SetModulation { wait, speed, delta, steps } => Some(TrackModulation {
                wait: *wait, speed: *speed, delta: *delta, steps: *steps,
            }),
            _ => None,
        });

        if ch.kind == SmpsChannelKind::Fm {
            let (notes, first_inst, track_warnings, total_daw_ticks) = map_fm_channel_flat(
                ch, daw_per_smps, &voice_to_fm_id, &mut instruments, &mut warnings,
            );
            warnings.extend(track_warnings);

            let duration = total_daw_ticks.max(notes.last().map(|n| n.tick + n.duration_ticks).unwrap_or(0));
            let regions = if notes.is_empty() {
                vec![]
            } else {
                vec![Region {
                    id: Uuid::new_v4(),
                    start_tick: 0,
                    duration_ticks: duration,
                    notes,
                    instrument_id: first_inst,
                }]
            };

            let track_vol = (127i16 - ch.initial_volume as i16).clamp(0, 127) as u8;
            tracks.push(Track {
                id: Uuid::new_v4(),
                name: track_name,
                channel: channel_assign.clone(),
                instrument_id: first_inst,
                regions,
                muted: false,
                solo: false,
                volume: track_vol,
                pan: pan.clone(),
                pitch_offset: 0,
                modulation: modulation.clone(),
            });
        } else if ch.kind == SmpsChannelKind::Dac {
            let (notes, track_warnings, total_daw_ticks) = map_channel_events(ch, daw_per_smps, &mut dac_sample_to_id, &mut psg_env_to_id, &mut instruments);
            warnings.extend(track_warnings);

            let first_inst = dac_sample_to_id.values().next().copied();
            let duration = total_daw_ticks.max(notes.last().map(|n| n.tick + n.duration_ticks).unwrap_or(0));
            let regions = if notes.is_empty() {
                vec![]
            } else {
                vec![Region {
                    id: Uuid::new_v4(),
                    start_tick: 0,
                    duration_ticks: duration,
                    notes,
                    instrument_id: first_inst,
                }]
            };

            tracks.push(Track {
                id: Uuid::new_v4(),
                name: track_name,
                channel: channel_assign,
                instrument_id: first_inst,
                regions,
                muted: false,
                solo: false,
                volume: (127i16 - ch.initial_volume as i16).clamp(0, 127) as u8,
                pan,
                pitch_offset: 0,
                modulation: modulation.clone(),
            });
        } else {
            let instrument_id = match ch.kind {
                SmpsChannelKind::Psg => {
                    resolve_psg_instrument(ch, &mut psg_env_to_id, &mut instruments)
                }
                _ => None,
            };

            let (notes, track_warnings, total_daw_ticks) = map_channel_events(ch, daw_per_smps, &mut dac_sample_to_id, &mut psg_env_to_id, &mut instruments);
            warnings.extend(track_warnings);

            let duration = total_daw_ticks.max(notes.last().map(|n| n.tick + n.duration_ticks).unwrap_or(0));
            let region = if notes.is_empty() {
                vec![]
            } else {
                vec![Region {
                    id: Uuid::new_v4(),
                    start_tick: 0,
                    duration_ticks: duration,
                    notes,
                    instrument_id: None,
                }]
            };

            let track_vol = if matches!(ch.kind, SmpsChannelKind::Psg) {
                ((15i16 - ch.initial_volume as i16).clamp(0, 15) * 127 / 15) as u8
            } else {
                (127i16 - ch.initial_volume as i16).clamp(0, 127) as u8
            };
            tracks.push(Track {
                id: Uuid::new_v4(),
                name: track_name,
                channel: channel_assign,
                instrument_id,
                regions: region,
                muted: false,
                solo: false,
                volume: track_vol,
                pan,
                pitch_offset: 0,
                modulation,
            });
        }
    }

    let mut dac_pcm: HashMap<Uuid, Vec<u8>> = HashMap::new();
    if let Some(dir) = dac_dir {
        for inst in &mut instruments.dac {
            let sample_byte = dac_sample_to_id.iter()
                .find(|(_, &id)| id == inst.id)
                .map(|(&byte, _)| byte);
            if let Some(byte) = sample_byte {
                if let Some(pcm) = load_dac_wav(dir, byte) {
                    let filename = format!("{}.pcm", inst.id);
                    inst.pcm_file = filename;
                    inst.original_file = format!("{:02X}.wav", byte);
                    inst.target_sample_rate = 18790;
                    dac_pcm.insert(inst.id, pcm);
                }
            }
        }
    }

    let song = Song {
        metadata: SongMetadata {
            name: smps.song_label.replace('_', " "),
            tempo: bpm.max(1.0),
            time_signature: (4, 4),
            ticks_per_beat: DAW_TICKS_PER_BEAT,
            driver_id: driver.id().to_string(),
        },
        tracks,
        instruments,
    };

    Ok(MappedSong { song, warnings, dac_pcm })
}

fn resolve_psg_instrument(
    ch: &SmpsChannel,
    psg_env_to_id: &mut HashMap<u8, Uuid>,
    instruments: &mut InstrumentBank,
) -> Option<Uuid> {
    let env_idx = ch.psg_envelope?;
    resolve_psg_env(env_idx, psg_env_to_id, instruments)
}

fn resolve_psg_env(
    env_idx: u8,
    psg_env_to_id: &mut HashMap<u8, Uuid>,
    instruments: &mut InstrumentBank,
) -> Option<Uuid> {
    if let Some(&id) = psg_env_to_id.get(&env_idx) {
        return Some(id);
    }

    let (volumes, loop_point) = match psg_envelopes::get_envelope(env_idx) {
        Some(entry) => {
            let vols: Vec<u8> = entry.volumes.iter().map(|&v| {
                let atten = (v as u8).min(15);
                15 - atten
            }).collect();
            (vols, entry.loop_point)
        }
        None => (vec![0], None),
    };

    let inst = PsgInstrument {
        id: Uuid::new_v4(),
        name: format!("PSG Env ${:02X}", env_idx),
        volume_sequence: volumes,
        loop_point,
        noise_mode: None,
        smps_envelope_index: Some(env_idx),
        metadata: InstrumentMetadata {
            category: "Imported".into(),
            ..Default::default()
        },
    };
    let id = inst.id;
    psg_env_to_id.insert(env_idx, id);
    instruments.psg.push(inst);
    Some(id)
}

fn map_fm_channel_flat(
    ch: &SmpsChannel,
    daw_per_smps: f64,
    voice_to_fm_id: &HashMap<u8, Uuid>,
    instruments: &mut InstrumentBank,
    warnings: &mut Vec<ImportWarning>,
) -> (Vec<Note>, Option<Uuid>, Vec<ImportWarning>, u64) {
    let mut notes: Vec<Note> = Vec::new();
    let mut track_warnings = Vec::new();
    let mut unsupported_counts: HashMap<String, u32> = HashMap::new();
    let mut cursor: f64 = 0.0;
    let mut transpose: i16 = ch.initial_pitch as i16;
    let mut volume_delta: i16 = 0;
    let mut tying = false;
    let mut current_inst_id: Option<Uuid> = None;
    let mut first_inst_id: Option<Uuid> = None;
    let mut unresolved_ids: HashMap<u8, Uuid> = HashMap::new();
    let mut current_detune: i8 = 0;
    let mut current_pan: Option<u8> = None;
    let mut current_mod: Option<NoteModulation> = None;

    for event in &ch.events {
        match event {
            SmpsEvent::SetVoice(v) => {
                current_inst_id = voice_to_fm_id.get(v).copied().or_else(|| {
                    if let Some(&id) = unresolved_ids.get(v) {
                        return Some(id);
                    }
                    let inst = FmInstrument {
                        id: Uuid::new_v4(),
                        name: format!("Voice {} (unresolved)", v),
                        algorithm: 0,
                        feedback: 0,
                        operators: [FmOperator::default(); 4],
                        metadata: InstrumentMetadata {
                            category: "Imported".into(),
                            ..Default::default()
                        },
                    };
                    let id = inst.id;
                    unresolved_ids.insert(*v, id);
                    instruments.fm.push(inst);
                    warnings.push(ImportWarning {
                        channel: ch.label.clone(),
                        message: format!("voice {} unresolved (UVB or external)", v),
                    });
                    Some(id)
                });
                if first_inst_id.is_none() {
                    first_inst_id = current_inst_id;
                }
            }
            SmpsEvent::Note { pitch, duration } => {
                let daw_dur = (*duration as f64 * daw_per_smps).round() as u64;
                let midi = smps_to_midi(*pitch, transpose);
                if tying {
                    if let Some(last) = notes.last_mut() {
                        last.duration_ticks += daw_dur;
                        tying = false;
                        cursor += *duration as f64 * daw_per_smps;
                        continue;
                    }
                }
                let velocity = smps_vol_to_velocity(volume_delta);
                notes.push(Note {
                    tick: cursor.round() as u64,
                    pitch: midi,
                    velocity,
                    duration_ticks: daw_dur.max(1),
                    instrument_id: current_inst_id,
                    detune: current_detune,
                    pan_override: current_pan,
                    modulation: current_mod.clone(),
                });
                tying = false;
                cursor += *duration as f64 * daw_per_smps;
            }
            SmpsEvent::Rest { duration } => {
                tying = false;
                cursor += *duration as f64 * daw_per_smps;
            }
            SmpsEvent::Transpose(offset) => {
                transpose += *offset as i16;
            }
            SmpsEvent::VolumeChange(delta) => {
                volume_delta = (volume_delta + *delta as i16).clamp(0, 127);
            }
            SmpsEvent::Tie => {
                tying = true;
            }
            SmpsEvent::Detune(val) => {
                current_detune = *val;
            }
            SmpsEvent::SetModulation { wait, speed, delta, steps } => {
                current_mod = Some(NoteModulation {
                    wait: *wait, speed: *speed, delta: *delta, steps: *steps,
                });
            }
            SmpsEvent::ModOff => {
                current_mod = None;
            }
            SmpsEvent::SetPan(p) => {
                current_pan = Some(*p);
            }
            SmpsEvent::Stop => break,
            SmpsEvent::Unsupported { name } => {
                *unsupported_counts.entry(name.clone()).or_insert(0u32) += 1;
            }
        }
    }

    for (name, count) in &unsupported_counts {
        track_warnings.push(ImportWarning {
            channel: ch.label.clone(),
            message: if *count == 1 {
                format!("unsupported: {name}")
            } else {
                format!("unsupported: {name} ({count} occurrences)")
            },
        });
    }

    let total_daw_ticks = cursor.round() as u64;
    (notes, first_inst_id, track_warnings, total_daw_ticks)
}

fn map_channel_events(
    ch: &SmpsChannel,
    daw_per_smps: f64,
    dac_sample_to_id: &mut HashMap<u8, Uuid>,
    psg_env_to_id: &mut HashMap<u8, Uuid>,
    instruments: &mut InstrumentBank,
) -> (Vec<Note>, Vec<ImportWarning>, u64) {
    let mut notes = Vec::new();
    let mut warnings = Vec::new();
    let mut unsupported_counts: HashMap<String, u32> = HashMap::new();
    let mut cursor: f64 = 0.0;
    let mut transpose: i16 = ch.initial_pitch as i16;
    let mut volume_delta: i16 = 0;
    let mut tying = false;
    let mut current_psg_inst: Option<Uuid> = None;
    let mut current_pan: Option<u8> = None;

    if ch.kind == SmpsChannelKind::Psg {
        if let Some(env_idx) = ch.psg_envelope {
            current_psg_inst = resolve_psg_env(env_idx, psg_env_to_id, instruments);
        }
    }

    for event in &ch.events {
        match event {
            SmpsEvent::Note { pitch, duration } => {
                let daw_dur = (*duration as f64 * daw_per_smps).round() as u64;

                if ch.kind == SmpsChannelKind::Dac {
                    let _sample_id = resolve_dac_sample(*pitch, dac_sample_to_id, instruments);
                    notes.push(Note {
                        tick: cursor.round() as u64,
                        pitch: *pitch,
                        velocity: 100,
                        duration_ticks: daw_dur.max(1),
                        instrument_id: None,
                        detune: 0,
                        pan_override: current_pan,
                        modulation: None,
                    });
                } else {
                    let midi = if ch.kind == SmpsChannelKind::Psg {
                        smps_to_midi_psg(*pitch, transpose)
                    } else {
                        smps_to_midi(*pitch, transpose)
                    };
                    if tying {
                        if let Some(last) = notes.last_mut() {
                            last.duration_ticks += daw_dur;
                            tying = false;
                            cursor += *duration as f64 * daw_per_smps;
                            continue;
                        }
                    }
                    let velocity = smps_vol_to_velocity(volume_delta);
                    notes.push(Note {
                        tick: cursor.round() as u64,
                        pitch: midi,
                        velocity,
                        duration_ticks: daw_dur.max(1),
                        instrument_id: current_psg_inst,
                        detune: 0,
                        pan_override: None,
                        modulation: None,
                    });
                }
                tying = false;
                cursor += *duration as f64 * daw_per_smps;
            }
            SmpsEvent::Rest { duration } => {
                tying = false;
                cursor += *duration as f64 * daw_per_smps;
            }
            SmpsEvent::Transpose(offset) => {
                transpose += *offset as i16;
            }
            SmpsEvent::VolumeChange(delta) => {
                volume_delta = (volume_delta + *delta as i16).clamp(0, 127);
            }
            SmpsEvent::Tie => {
                tying = true;
            }
            SmpsEvent::SetVoice(v) => {
                if ch.kind == SmpsChannelKind::Psg {
                    current_psg_inst = resolve_psg_env(*v, psg_env_to_id, instruments);
                }
            }
            SmpsEvent::Detune(_) => {}
            SmpsEvent::SetPan(p) => {
                current_pan = Some(*p);
            }
            SmpsEvent::SetModulation { .. } | SmpsEvent::ModOff => {}
            SmpsEvent::Stop => break,
            SmpsEvent::Unsupported { name } => {
                *unsupported_counts.entry(name.clone()).or_insert(0u32) += 1;
            }
        }
    }

    for (name, count) in &unsupported_counts {
        warnings.push(ImportWarning {
            channel: ch.label.clone(),
            message: if *count == 1 {
                format!("unsupported: {name}")
            } else {
                format!("unsupported: {name} ({count} occurrences)")
            },
        });
    }

    let total_daw_ticks = cursor.round() as u64;
    (notes, warnings, total_daw_ticks)
}

fn smps_vol_to_velocity(smps_volume: i16) -> u8 {
    (127 - smps_volume).clamp(0, 127) as u8
}

fn smps_to_midi(smps_byte: u8, transpose: i16) -> u8 {
    if smps_byte < 0x81 {
        return 0;
    }
    // Emulate Z80 unsigned 8-bit wrapping: add a,(ix+Transpose)
    let z80_index = ((smps_byte as i16 - 0x81) + transpose).rem_euclid(256) as u8;
    // FM octave loop: subtract 12 until underflow, count octaves
    let block = ((z80_index / 12) % 8) as u8;
    let semitone = z80_index % 12;
    let midi = (block as u16 + 1) * 12 + semitone as u16;
    midi.min(127) as u8
}

fn smps_to_midi_psg(smps_byte: u8, transpose: i16) -> u8 {
    if smps_byte < 0x81 {
        return 0;
    }
    // PSG uses the same z80 index but looks up a finite table — clamp rather than wrap
    let raw = (smps_byte as i16 - 0x81) + 36 + transpose;
    raw.clamp(0, 127) as u8
}

fn pan_from_byte(b: u8) -> Pan {
    match b {
        0x80 => Pan::Left,
        0x40 => Pan::Right,
        0xC0 => Pan::Center,
        _ => Pan::Center,
    }
}

fn resolve_dac_sample(
    sample_byte: u8,
    dac_sample_to_id: &mut HashMap<u8, Uuid>,
    instruments: &mut InstrumentBank,
) -> Uuid {
    if let Some(&id) = dac_sample_to_id.get(&sample_byte) {
        return id;
    }
    let friendly = dac_name_for_byte(sample_byte);
    let name = if friendly.is_empty() {
        format!("DAC ${:02X}", sample_byte)
    } else {
        format!("DAC {} (${:02X})", friendly, sample_byte)
    };
    let inst = DacInstrument {
        id: Uuid::new_v4(),
        name,
        target_sample_rate: 18790,
        loop_start: None,
        loop_length: None,
        original_file: String::new(),
        pcm_file: String::new(),
        source_is_raw: true,
        metadata: InstrumentMetadata {
            category: "Imported".into(),
            ..Default::default()
        },
    };
    let id = inst.id;
    dac_sample_to_id.insert(sample_byte, id);
    instruments.dac.push(inst);
    id
}

fn load_dac_wav(dac_dir: &Path, sample_byte: u8) -> Option<Vec<u8>> {
    let hex = format!("{:02X}", sample_byte);
    let exact_path = dac_dir.join(format!("{hex}.wav"));
    if exact_path.exists() {
        return read_wav_as_unsigned_u8(&exact_path);
    }

    let hex_upper = hex.to_uppercase();
    if let Ok(entries) = std::fs::read_dir(dac_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".wav") { continue; }
            let stem = name.trim_end_matches(".wav");
            let stem_clean = stem.split(" (").next().unwrap_or(stem);
            if let Some((start_s, end_s)) = stem_clean.split_once('-') {
                let start = u8::from_str_radix(start_s.trim(), 16).ok();
                let end = u8::from_str_radix(end_s.trim(), 16).ok();
                if let (Some(s), Some(e)) = (start, end) {
                    if sample_byte >= s && sample_byte <= e {
                        return read_wav_as_unsigned_u8(&entry.path());
                    }
                }
            }
            if stem_clean.to_uppercase() == hex_upper {
                return read_wav_as_unsigned_u8(&entry.path());
            }
        }
    }
    None
}

fn read_wav_as_unsigned_u8(path: &Path) -> Option<Vec<u8>> {
    let data = std::fs::read(path).ok()?;
    if data.len() < 44 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return None;
    }
    let mut pos = 12;
    while pos + 8 <= data.len() {
        let chunk_id = &data[pos..pos + 4];
        let chunk_size = u32::from_le_bytes(data[pos + 4..pos + 8].try_into().ok()?) as usize;
        if chunk_id == b"data" {
            let start = pos + 8;
            let end = (start + chunk_size).min(data.len());
            return Some(data[start..end].to_vec());
        }
        pos += 8 + chunk_size;
        if pos % 2 != 0 { pos += 1; }
    }
    None
}

fn dac_name_for_byte(byte: u8) -> &'static str {
    match byte {
        0x81 => "Snare (S3)", 0x82 => "High Tom", 0x83 => "Mid Tom (S3)",
        0x84 => "Low Tom (S3)", 0x85 => "Floor Tom (S3)", 0x86 => "Kick (S3)",
        0x87 => "Muffled Snare", 0x88 => "Crash Cymbal", 0x89 => "Ride Cymbal",
        0x8F => "Clap (S3)", 0x9C => "Click", 0x9D => "Power Kick",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::flamedriver::FlamedriverProfile;
    use crate::import::smps_parser::{SmpsFile, SmpsChannel, SmpsChannelKind, SmpsEvent, VoiceRef};

    fn make_simple_smps() -> SmpsFile {
        SmpsFile {
            song_label: "Test_Song".into(),
            voice_ref: VoiceRef::Inline("Test_Song_Voices".into()),
            fm_count: 1,
            psg_count: 0,
            tempo_divider: 1,
            tempo_modifier: 0x18,
            channels: vec![
                SmpsChannel {
                    kind: SmpsChannelKind::Fm,
                    label: "Test_Song_FM1".into(),
                    initial_pitch: 0,
                    initial_volume: 0x0F,
                    psg_envelope: None,
                    events: vec![
                        SmpsEvent::SetVoice(0),
                        SmpsEvent::Note { pitch: 0x93, duration: 0x18 },
                        SmpsEvent::Rest { duration: 0x06 },
                        SmpsEvent::Note { pitch: 0x95, duration: 0x0C },
                        SmpsEvent::Stop,
                    ],
                },
            ],
            voices: vec![[0u8; 25]],
        }
    }

    #[test]
    fn test_map_creates_tracks() {
        let smps = make_simple_smps();
        let driver = FlamedriverProfile;
        let result = map_smps_to_song(&smps, &driver).unwrap();
        assert_eq!(result.song.tracks.len(), 1);
        assert_eq!(result.song.tracks[0].name, "FM1");
    }

    #[test]
    fn test_map_creates_metadata() {
        let smps = make_simple_smps();
        let driver = FlamedriverProfile;
        let result = map_smps_to_song(&smps, &driver).unwrap();
        assert_eq!(result.song.metadata.driver_id, "flamedriver");
        assert_eq!(result.song.metadata.ticks_per_beat, 480);
        assert!(result.song.metadata.tempo > 0.0);
    }

    #[test]
    fn test_map_creates_fm_instrument() {
        let smps = make_simple_smps();
        let driver = FlamedriverProfile;
        let result = map_smps_to_song(&smps, &driver).unwrap();
        assert_eq!(result.song.instruments.fm.len(), 1);
        assert!(result.song.tracks[0].instrument_id.is_some());
    }

    #[test]
    fn test_map_note_pitches() {
        let smps = make_simple_smps();
        let driver = FlamedriverProfile;
        let result = map_smps_to_song(&smps, &driver).unwrap();
        let notes = &result.song.tracks[0].regions[0].notes;
        // 0x93 - 0x81 = 0x12 = 18. MIDI = 12 + 18 = 30.
        assert_eq!(notes[0].pitch, 30);
        // 0x95 - 0x81 = 0x14 = 20. MIDI = 12 + 20 = 32.
        assert_eq!(notes[1].pitch, 32);
    }

    #[test]
    fn test_map_creates_region() {
        let smps = make_simple_smps();
        let driver = FlamedriverProfile;
        let result = map_smps_to_song(&smps, &driver).unwrap();
        assert_eq!(result.song.tracks[0].regions.len(), 1);
        assert_eq!(result.song.tracks[0].regions[0].start_tick, 0);
        assert!(result.song.tracks[0].regions[0].duration_ticks > 0);
    }

    #[test]
    fn test_map_tie_extends_previous_note() {
        let smps = SmpsFile {
            song_label: "Tie_Test".into(),
            voice_ref: VoiceRef::Inline("Tie_Test_Voices".into()),
            fm_count: 1,
            psg_count: 0,
            tempo_divider: 1,
            tempo_modifier: 0x18,
            channels: vec![SmpsChannel {
                kind: SmpsChannelKind::Fm,
                label: "Tie_Test_FM1".into(),
                initial_pitch: 0,
                initial_volume: 0x0F,
                psg_envelope: None,
                events: vec![
                    SmpsEvent::SetVoice(0),
                    SmpsEvent::Note { pitch: 0xB1, duration: 0x7F },
                    SmpsEvent::Tie,
                    SmpsEvent::Note { pitch: 0xB1, duration: 0x29 },
                    SmpsEvent::Stop,
                ],
            }],
            voices: vec![[0u8; 25]],
        };
        let driver = FlamedriverProfile;
        let result = map_smps_to_song(&smps, &driver).unwrap();
        let notes = &result.song.tracks[0].regions[0].notes;
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn test_fm_note_matches_flamedriver() {
        use crate::audio::frequency::midi_to_fm_freq;
        fn flamedriver_block_fnum_z80(note_index: u8) -> (u8, u16) {
            let fm_fnum: [u16; 12] = [0x284, 0x2AB, 0x2D3, 0x2FE, 0x32D, 0x35C, 0x38F, 0x3C5, 0x3FF, 0x43C, 0x47C, 0x4C0];
            let mut idx = note_index;
            let mut block_acc: u8 = 0;
            while idx >= 12 { idx -= 12; block_acc = block_acc.wrapping_add(8); }
            let fnum = fm_fnum[idx as usize];
            let block = (block_acc | ((fnum >> 8) as u8)) >> 3 & 7;
            (block, fnum)
        }
        for smps_byte in 0x81u8..=0xDF {
            for transpose in [-23i16, -13, -12, -5, 0, 5, 12, 24] {
                let z80_index = ((smps_byte as i16 - 0x81) + transpose).rem_euclid(256) as u8;
                let (fd_block, fd_fnum) = flamedriver_block_fnum_z80(z80_index);
                let our_midi = smps_to_midi(smps_byte, transpose);
                let (our_block, our_fnum) = midi_to_fm_freq(our_midi);
                assert_eq!((fd_block, fd_fnum), (our_block, our_fnum),
                    "Mismatch for note ${:02X} transpose={} z80_idx={}", smps_byte, transpose, z80_index);
            }
        }
    }

    #[test]
    fn test_dac_notes_grouped_by_sample() {
        let driver = FlamedriverProfile;
        let smps = SmpsFile {
            song_label: "DAC_Test".into(),
            voice_ref: VoiceRef::Inline("DAC_Test_Voices".into()),
            fm_count: 0, psg_count: 0,
            tempo_divider: 1, tempo_modifier: 0x18,
            channels: vec![SmpsChannel {
                kind: SmpsChannelKind::Dac,
                label: "DAC_Test_DAC".into(),
                initial_pitch: 0, initial_volume: 0,
                psg_envelope: None,
                events: vec![
                    SmpsEvent::Note { pitch: 0x86, duration: 0x0C },
                    SmpsEvent::Note { pitch: 0x81, duration: 0x0C },
                    SmpsEvent::Note { pitch: 0x86, duration: 0x0C },
                    SmpsEvent::Stop,
                ],
            }],
            voices: vec![],
        };
        let result = map_smps_to_song(&smps, &driver).unwrap();
        let dac_track = &result.song.tracks[0];
        assert_eq!(dac_track.regions.len(), 1, "DAC should have a single region");
        assert_eq!(dac_track.regions[0].notes.len(), 3);
        assert!(dac_track.regions[0].instrument_id.is_some());
    }

    #[test]
    fn test_aiz1_full_pipeline_diagnostic() {
        use crate::audio::frequency::midi_to_fm_freq;
        use crate::import::smps_parser::parse_smps;

        let source = include_str!("../../test_data/AIZ1.asm");
        let smps = parse_smps(source).unwrap();
        let driver = FlamedriverProfile;
        let result = map_smps_to_song(&smps, &driver).unwrap();
        let song = &result.song;

        // Basic structure: DAC + 5 FM + 3 PSG = 9 channels
        assert_eq!(song.tracks.len(), 9, "AIZ1 should have 9 tracks");

        // Verify channel types
        assert_eq!(song.tracks[0].name, "DAC");
        for i in 1..=5 {
            assert_eq!(song.tracks[i].name, format!("FM{}", i));
        }

        // Verify UVB voice resolution: AIZ1 FM1 uses voices $15 and $14
        // Voice $15 = index 21 (algorithm 0, "Picked Bass")
        // Voice $14 = index 20 (algorithm 2)
        let voice_15 = song.instruments.fm.iter().find(|i| i.name == "Voice 21");
        assert!(voice_15.is_some(), "Voice 21 ($15) should exist");
        assert_eq!(voice_15.unwrap().algorithm, 0, "Voice $15 should be algo 0");
        assert_eq!(voice_15.unwrap().feedback, 5, "Voice $15 should have fb 5");

        let voice_14 = song.instruments.fm.iter().find(|i| i.name == "Voice 20");
        assert!(voice_14.is_some(), "Voice 20 ($14) should exist");
        assert_eq!(voice_14.unwrap().algorithm, 2, "Voice $14 should be algo 2");

        // Verify FM1 first note: nC1 ($8D) with initial transpose $18 (24)
        // smps_to_midi(0x8D, 24): z80_index = 12+24 = 36, block=3, semi=0, midi=48
        let fm1 = &song.tracks[1];
        let fm1_notes = &fm1.regions[0].notes;
        assert!(!fm1_notes.is_empty(), "FM1 should have notes");
        assert_eq!(fm1_notes[0].pitch, 48, "FM1 first note should be MIDI 48 (C3)");

        // Verify MIDI 48 produces correct FM frequency
        let (block, fnum) = midi_to_fm_freq(48);
        assert_eq!(block, 3, "MIDI 48 → block 3");
        assert_eq!(fnum, 644, "MIDI 48 → fnum 644 (C)");

        // Verify FM2 note pitches with transpose accumulation
        // FM2 initial pitch = $0C (12)
        // FM2 has multiple smpsAlterNote events that accumulate
        let fm2 = &song.tracks[2];
        let fm2_notes = &fm2.regions[0].notes;
        assert!(!fm2_notes.is_empty(), "FM2 should have notes");
        // All FM2 notes should have valid MIDI pitches (>0, <=127)
        for (i, note) in fm2_notes.iter().enumerate() {
            assert!(note.pitch > 0 && note.pitch <= 127,
                "FM2 note {} pitch {} out of range", i, note.pitch);
        }

        // Verify each FM note produces a valid block/fnum pair
        for (track_idx, track) in song.tracks.iter().enumerate() {
            if !track.name.starts_with("FM") { continue; }
            for region in &track.regions {
                for (i, note) in region.notes.iter().enumerate() {
                    let (b, f) = midi_to_fm_freq(note.pitch);
                    assert!(b <= 7, "{} note {} block {} > 7", track.name, i, b);
                    assert!(f >= 644 && f <= 1216,
                        "{} note {} fnum {} out of range", track.name, i, f);
                }
            }
        }

        // Verify instrument round-trip: fm_to_bytes → matches what sequencer needs
        for inst in &song.instruments.fm {
            let bytes = driver.fm_to_bytes(inst);
            assert_eq!(bytes.len(), 25, "FM bytes should be 25");
            // Verify carrier bits are set correctly
            let carrier_masks: [u8; 8] = [
                0b0001, 0b0001, 0b0001, 0b0001, 0b0101, 0b0111, 0b0111, 0b1111,
            ];
            let mask = carrier_masks[inst.algorithm as usize];
            for i in 0..4 {
                let has_carrier_bit = bytes[20 + i] & 0x80 != 0;
                let should_be_carrier = (mask >> i) & 1 == 1;
                assert_eq!(has_carrier_bit, should_be_carrier,
                    "Voice '{}' op{} carrier bit mismatch (algo={})",
                    inst.name, i + 1, inst.algorithm);
            }
        }

        // Verify all channels have reasonable durations (> 100 DAW ticks)
        for track in &song.tracks {
            for region in &track.regions {
                assert!(region.duration_ticks > 100,
                    "{} has suspiciously short region: {} ticks",
                    track.name, region.duration_ticks);
            }
        }

        // Print diagnostic summary
        eprintln!("=== AIZ1 Import Diagnostic ===");
        eprintln!("BPM: {:.1}", song.metadata.tempo);
        eprintln!("Instruments: {} FM, {} PSG, {} DAC",
            song.instruments.fm.len(),
            song.instruments.psg.len(),
            song.instruments.dac.len());
        for track in &song.tracks {
            let notes: usize = track.regions.iter().map(|r| r.notes.len()).sum();
            let dur: u64 = track.regions.iter().map(|r| r.duration_ticks).sum();
            eprintln!("  {}: {} notes, {} ticks", track.name, notes, dur);
        }
        eprintln!("Warnings: {}", result.warnings.len());
        for w in &result.warnings {
            eprintln!("  [{}] {}", w.channel, w.message);
        }
    }

    #[test]
    fn test_aiz1_psg_periods_match_flamedriver() {
        use crate::audio::frequency::midi_to_psg_period;
        use crate::import::smps_parser::parse_smps;

        // Flamedriver PSG frequency table (periods for note indices 0-83)
        let fd_psg_table: [u16; 84] = [
            0x3FF,0x3FF,0x3FF,0x3FF,0x3FF,0x3FF,0x3FF,0x3FF,0x3FF,0x3F7,0x3BE,0x388,
            0x356,0x326,0x2F9,0x2CE,0x2A5,0x280,0x25C,0x23A,0x21A,0x1FB,0x1DF,0x1C4,
            0x1AB,0x193,0x17D,0x167,0x153,0x140,0x12E,0x11D,0x10D,0x0FE,0x0EF,0x0E2,
            0x0D6,0x0C9,0x0BE,0x0B4,0x0A9,0x0A0,0x097,0x08F,0x087,0x07F,0x078,0x071,
            0x06B,0x065,0x05F,0x05A,0x055,0x050,0x04B,0x047,0x043,0x040,0x03C,0x039,
            0x036,0x033,0x030,0x02D,0x02B,0x028,0x026,0x024,0x022,0x020,0x01F,0x01D,
            0x01B,0x01A,0x018,0x017,0x016,0x015,0x013,0x012,0x011,0x010,0x000,0x000,
        ];

        // Check that our midi_to_psg_period values are close to Flamedriver's table
        let mut max_diff: u16 = 0;
        for note_idx in 9u8..82 { // Skip clamped entries at start and end
            let midi = note_idx as u8 + 36;
            let our_period = midi_to_psg_period(midi);
            let fd_period = fd_psg_table[note_idx as usize];
            let diff = (our_period as i32 - fd_period as i32).unsigned_abs() as u16;
            if diff > max_diff { max_diff = diff; }
            assert!(diff <= 2,
                "PSG note {} (MIDI {}): ours={}, Flamedriver={}, diff={}",
                note_idx, midi, our_period, fd_period, diff);
        }
        eprintln!("PSG max period difference: {}", max_diff);
    }

    #[test]
    fn test_map_uvb_resolves_from_bundled_bank() {
        let smps = SmpsFile {
            song_label: "UVB_Test".into(),
            voice_ref: VoiceRef::Uvb,
            fm_count: 1,
            psg_count: 0,
            tempo_divider: 1,
            tempo_modifier: 0x18,
            channels: vec![SmpsChannel {
                kind: SmpsChannelKind::Fm,
                label: "UVB_Test_FM1".into(),
                initial_pitch: 0,
                initial_volume: 0x0F,
                psg_envelope: None,
                events: vec![
                    SmpsEvent::SetVoice(0),
                    SmpsEvent::Note { pitch: 0xB1, duration: 0x18 },
                    SmpsEvent::Stop,
                ],
            }],
            voices: vec![],
        };
        let driver = FlamedriverProfile;
        let result = map_smps_to_song(&smps, &driver).unwrap();
        assert!(result.song.instruments.fm.len() >= 1);
        assert_eq!(result.song.instruments.fm[0].name, "Voice 0");
        assert_eq!(result.song.instruments.fm[0].algorithm, 4);
    }
}
