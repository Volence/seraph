use std::collections::HashMap;
use uuid::Uuid;

use crate::model::instrument::*;
use crate::model::song::*;
use crate::import::ImportWarning;
use crate::import::zyrinx_parser::*;

const DAW_TICKS_PER_BEAT: u32 = 480;
const ZYRINX_TICK_SUB: f64 = 16.0;
const NTSC_FPS: f64 = 60.0;
const ZYRINX_PITCH_OFFSET: u8 = 16;

// Zyrinx voices store operators in YM2612 register slot order [OP1, OP3, OP2, OP4]
// (slots +0x00, +0x04, +0x08, +0x0C — the hardware 1/3/2/4 ordering per Furnace/Yamaha).
// Seraph's PACKED_OP_SLOTS maps operators[0..3] to slots [+0C, +04, +08, +00].
const ZY_TO_SMPS: [usize; 4] = [3, 1, 2, 0];

const CARRIER_MASKS: [u8; 8] = [
    0b0001, 0b0001, 0b0001, 0b0001,
    0b0101, 0b0111, 0b0111, 0b1111,
];

pub struct MappedZyrinxSong {
    pub song: Song,
    pub warnings: Vec<ImportWarning>,
}

pub fn map_zyrinx_to_song(zy: &ZyrinxSong) -> MappedZyrinxSong {
    let mut warnings = Vec::new();

    let tempo_base = zy.tempo_base.max(1) as f64;
    let frames_per_beat = 8.0;
    let bpm = NTSC_FPS * ZYRINX_TICK_SUB * 60.0 / (tempo_base * frames_per_beat);
    let frame_to_daw = DAW_TICKS_PER_BEAT as f64 / frames_per_beat;

    let mut instruments = InstrumentBank::default();
    let mut voice_to_fm_id: HashMap<u8, Uuid> = HashMap::new();

    for voice in &zy.voices {
        let algo = voice.fb_alg & 0x07;
        let fb = (voice.fb_alg >> 3) & 0x07;

        let mut operators = [
            FmOperator::default(),
            FmOperator::default(),
            FmOperator::default(),
            FmOperator::default(),
        ];

        for i in 0..4 {
            let dst = ZY_TO_SMPS[i];
            operators[dst].detune = voice.dt_mul[i] >> 4;
            operators[dst].multiple = voice.dt_mul[i] & 0x0F;
            operators[dst].total_level = (voice.tl[i] ^ 0x7F) & 0x7F;
            operators[dst].rate_scale = voice.ks_ar[i] >> 6;
            operators[dst].attack_rate = voice.ks_ar[i] & 0x1F;
            operators[dst].amp_mod = voice.am_d1r[i] & 0x80 != 0;
            operators[dst].d1r = voice.am_d1r[i] & 0x1F;
            operators[dst].d2r = voice.d2r[i] & 0x1F;
            operators[dst].sustain_level = voice.sl_rr[i] >> 4;
            operators[dst].release_rate = voice.sl_rr[i] & 0x0F;
        }

        let cm = CARRIER_MASKS[algo as usize];
        for i in 0..4 {
            if (cm >> i) & 1 == 1 {
                operators[i].total_level = 0;
            }
        }

        let inst = FmInstrument {
            id: Uuid::new_v4(),
            name: format!("Zyrinx Voice {}", voice.index),
            algorithm: algo,
            feedback: fb,
            operators,
            metadata: InstrumentMetadata {
                category: "Zyrinx Import".into(),
                ..InstrumentMetadata::default()
            },
        };
        voice_to_fm_id.insert(voice.index, inst.id);
        instruments.fm.push(inst);
    }

    let mut tracks = Vec::new();

    for (ch_num, ch) in zy.channels.iter().enumerate() {
        let (notes, first_inst, ch_warnings) = map_channel_events(
            ch, frame_to_daw, &voice_to_fm_id, zy.detune,
        );
        for w in ch_warnings {
            warnings.push(ImportWarning {
                channel: format!("Ch{}", ch_num),
                message: w,
            });
        }

        let initial_pan = ch.events.iter().find_map(|e| match e {
            ZyrinxEvent::Pan(p) => Some(zyrinx_pan_to_daw(*p)),
            _ => None,
        }).unwrap_or(Pan::Center);

        let duration = notes.last()
            .map(|n| n.tick + n.duration_ticks)
            .unwrap_or(0);

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
            name: format!("Ch{}", ch_num + 1),
            channel: ChannelAssignment::Fm(ch_num as u8),
            instrument_id: first_inst,
            regions,
            muted: false,
            solo: false,
            volume: 127,
            pan: initial_pan,
            pitch_offset: 0,
            modulation: None,
        });
    }

    let metadata = SongMetadata {
        name: zy.name.clone(),
        tempo: bpm,
        time_signature: (4, 4),
        ticks_per_beat: DAW_TICKS_PER_BEAT,
        driver_id: "zyrinx".into(),
    };

    MappedZyrinxSong {
        song: Song { metadata, tracks, instruments },
        warnings,
    }
}

fn map_channel_events(
    ch: &ZyrinxChannel,
    frame_to_daw: f64,
    voice_map: &HashMap<u8, Uuid>,
    global_detune: i8,
) -> (Vec<Note>, Option<Uuid>, Vec<String>) {
    let mut notes = Vec::new();
    let mut warnings = Vec::new();
    let mut current_tick: f64 = 0.0;
    let mut current_voice: Option<Uuid> = None;
    let mut first_inst: Option<Uuid> = None;
    let mut current_pan: Option<u8> = None;

    for evt in &ch.events {
        match evt {
            ZyrinxEvent::Note { pitch, duration_frames } => {
                let daw_tick = current_tick as u64;
                let daw_dur = (*duration_frames as f64 * frame_to_daw).round().max(1.0) as u64;
                let midi_pitch = (*pitch as i16 - ZYRINX_PITCH_OFFSET as i16).clamp(0, 127) as u8;

                notes.push(Note {
                    tick: daw_tick,
                    pitch: midi_pitch,
                    velocity: 100,
                    duration_ticks: daw_dur,
                    instrument_id: current_voice,
                    detune: global_detune,
                    pan_override: current_pan,
                    modulation: None,
                });

                current_tick += *duration_frames as f64 * frame_to_daw;
            }
            ZyrinxEvent::Rest { duration_frames } => {
                current_tick += *duration_frames as f64 * frame_to_daw;
            }
            ZyrinxEvent::Voice(v) => {
                if let Some(&id) = voice_map.get(v) {
                    current_voice = Some(id);
                    if first_inst.is_none() {
                        first_inst = Some(id);
                    }
                } else {
                    warnings.push(format!("voice {} not found in voice map", v));
                }
            }
            ZyrinxEvent::Pan(p) => {
                current_pan = match p {
                    ZyrinxPan::Left => Some(0x40),
                    ZyrinxPan::Right => Some(0x80),
                    ZyrinxPan::Center => Some(0xC0),
                    ZyrinxPan::Off => Some(0x00),
                };
            }
            _ => {}
        }
    }

    (notes, first_inst, warnings)
}

fn zyrinx_pan_to_daw(pan: ZyrinxPan) -> Pan {
    match pan {
        ZyrinxPan::Left => Pan::Left,
        ZyrinxPan::Right => Pan::Right,
        ZyrinxPan::Center | ZyrinxPan::Off => Pan::Center,
    }
}
