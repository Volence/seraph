use std::collections::HashMap;
use uuid::Uuid;

use crate::driver::flamedriver::FlamedriverProfile;
use crate::model::driver::DriverProfile;
use crate::model::instrument::*;
use crate::model::song::*;
use crate::import::ImportWarning;
use crate::import::psg_envelopes;
use crate::import::smps_parser::*;

pub struct MappedSong {
    pub song: Song,
    pub warnings: Vec<ImportWarning>,
}

const SMPS_TICKS_PER_BEAT: f64 = 24.0;
const DAW_TICKS_PER_BEAT: u32 = 480;

pub fn map_smps_to_song(smps: &SmpsFile, driver: &dyn DriverProfile) -> Result<MappedSong, String> {
    let mut warnings = Vec::new();
    let daw_per_smps = DAW_TICKS_PER_BEAT as f64 / SMPS_TICKS_PER_BEAT;

    let smps_ticks_per_sec = (smps.tempo_modifier as f64 / 256.0) * 60.0;
    let bpm = smps_ticks_per_sec * 60.0 / SMPS_TICKS_PER_BEAT;

    let mut instruments = InstrumentBank::default();
    let mut voice_to_fm_id: HashMap<u8, Uuid> = HashMap::new();

    for (i, voice_bytes) in smps.voices.iter().enumerate() {
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

        let instrument_id = match ch.kind {
            SmpsChannelKind::Fm => {
                resolve_fm_instrument(ch, smps, &voice_to_fm_id, &mut instruments, &mut warnings)
            }
            SmpsChannelKind::Psg => {
                resolve_psg_instrument(ch, &mut psg_env_to_id, &mut instruments)
            }
            SmpsChannelKind::Dac => None,
        };

        let pan = match ch.kind {
            SmpsChannelKind::Psg | SmpsChannelKind::Dac => Pan::Center,
            SmpsChannelKind::Fm => {
                ch.events.iter().find_map(|e| match e {
                    SmpsEvent::SetPan(p) => Some(pan_from_byte(*p)),
                    _ => None,
                }).unwrap_or(Pan::Center)
            }
        };

        let (notes, track_warnings) = map_channel_events(ch, daw_per_smps, &mut dac_sample_to_id, &mut instruments);
        warnings.extend(track_warnings);

        let duration = notes.last().map(|n| n.tick + n.duration_ticks).unwrap_or(0);
        let region = if notes.is_empty() {
            vec![]
        } else {
            vec![Region {
                id: Uuid::new_v4(),
                start_tick: 0,
                duration_ticks: duration,
                notes,
            }]
        };

        tracks.push(Track {
            id: Uuid::new_v4(),
            name: track_name,
            channel: channel_assign,
            instrument_id,
            regions: region,
            muted: false,
            solo: false,
            volume: ch.initial_volume.min(127),
            pan,
        });
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

    Ok(MappedSong { song, warnings })
}

fn resolve_fm_instrument(
    ch: &SmpsChannel,
    smps: &SmpsFile,
    voice_to_fm_id: &HashMap<u8, Uuid>,
    instruments: &mut InstrumentBank,
    warnings: &mut Vec<ImportWarning>,
) -> Option<Uuid> {
    let voice_idx = ch.events.iter().find_map(|e| match e {
        SmpsEvent::SetVoice(v) => Some(*v),
        _ => None,
    })?;

    if let Some(&id) = voice_to_fm_id.get(&voice_idx) {
        return Some(id);
    }

    let inst = FmInstrument {
        id: Uuid::new_v4(),
        name: format!("Voice {} (unresolved)", voice_idx),
        algorithm: 0,
        feedback: 0,
        operators: [FmOperator::default(); 4],
        metadata: InstrumentMetadata {
            category: "Imported".into(),
            ..Default::default()
        },
    };
    let id = inst.id;
    instruments.fm.push(inst);
    warnings.push(ImportWarning {
        channel: ch.label.clone(),
        message: format!("voice {} unresolved (UVB or external)", voice_idx),
    });
    Some(id)
}

fn resolve_psg_instrument(
    ch: &SmpsChannel,
    psg_env_to_id: &mut HashMap<u8, Uuid>,
    instruments: &mut InstrumentBank,
) -> Option<Uuid> {
    let env_idx = ch.psg_envelope?;
    if let Some(&id) = psg_env_to_id.get(&env_idx) {
        return Some(id);
    }

    let (volumes, loop_point) = match psg_envelopes::get_envelope(env_idx) {
        Some(entry) => {
            let vols: Vec<u8> = entry.volumes.iter().map(|&v| (v as u8).min(15)).collect();
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

fn map_channel_events(
    ch: &SmpsChannel,
    daw_per_smps: f64,
    dac_sample_to_id: &mut HashMap<u8, Uuid>,
    instruments: &mut InstrumentBank,
) -> (Vec<Note>, Vec<ImportWarning>) {
    let mut notes = Vec::new();
    let mut warnings = Vec::new();
    let mut cursor: f64 = 0.0;
    let mut transpose: i16 = ch.initial_pitch as i16;
    let mut tying = false;

    for event in &ch.events {
        match event {
            SmpsEvent::Note { pitch, duration } => {
                let daw_dur = (*duration as f64 * daw_per_smps).round() as u64;

                if ch.kind == SmpsChannelKind::Dac {
                    let sample_id = resolve_dac_sample(*pitch, dac_sample_to_id, instruments);
                    notes.push(Note {
                        tick: cursor.round() as u64,
                        pitch: *pitch,
                        velocity: 100,
                        duration_ticks: daw_dur.max(1),
                    });
                } else {
                    let midi = smps_to_midi(*pitch, transpose);
                    if tying {
                        if let Some(last) = notes.last_mut() {
                            last.duration_ticks += daw_dur;
                            tying = false;
                            cursor += *duration as f64 * daw_per_smps;
                            continue;
                        }
                    }
                    notes.push(Note {
                        tick: cursor.round() as u64,
                        pitch: midi,
                        velocity: 100,
                        duration_ticks: daw_dur.max(1),
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
            SmpsEvent::Tie => {
                tying = true;
            }
            SmpsEvent::SetVoice(_) | SmpsEvent::SetPan(_) => {}
            SmpsEvent::Stop => break,
            SmpsEvent::Unsupported { name } => {
                warnings.push(ImportWarning {
                    channel: ch.label.clone(),
                    message: format!("unsupported: {}", name),
                });
            }
        }
    }

    (notes, warnings)
}

fn smps_to_midi(smps_byte: u8, transpose: i16) -> u8 {
    if smps_byte < 0x81 {
        return 0;
    }
    let raw = (smps_byte as i16 - 0x81) + 12 + transpose;
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
    let inst = DacInstrument {
        id: Uuid::new_v4(),
        name: format!("DAC ${:02X}", sample_byte),
        target_sample_rate: 16000,
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

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_map_uvb_creates_placeholder_instruments() {
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
        assert_eq!(result.song.instruments.fm.len(), 1);
        assert!(result.song.instruments.fm[0].name.contains("unresolved"));
        assert!(result.warnings.iter().any(|w| w.message.contains("unresolved")));
    }
}
