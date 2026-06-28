/// Parser for Zyrinx "Advanced Z80 Player" format (Batman & Robin, Genesis).
/// Reads binary ROM data and extracts song structures, sequences, and FM voice patches.

const BANK_ROM_BASES: [usize; 3] = [0x1E8000, 0x1F0000, 0x1F8000];

#[allow(dead_code)]
pub struct ZyrinxSong {
    pub name: String,
    pub game_id: u8,
    pub tempo_base: u8,
    pub detune: i8,
    pub voice_base: u8,
    pub loop_point: u8,
    pub channels: Vec<ZyrinxChannel>,
    pub voices: Vec<ZyrinxVoice>,
}

#[allow(dead_code)]
pub struct ZyrinxChannel {
    pub index: u8,
    pub events: Vec<ZyrinxEvent>,
}

pub enum ZyrinxEvent {
    Note { pitch: u8, duration_frames: u16 },
    Rest { duration_frames: u16 },
    Voice(u8),
    Pan(ZyrinxPan),
    FmOp { operator: u8, value: u8 },
    Volume { levels: Vec<u8>, duration: u8 },
    ChSelect(u8),
    Flag,
}

#[derive(Clone, Copy)]
pub enum ZyrinxPan {
    Off,
    Right,
    Left,
    Center,
}

#[allow(dead_code)]
pub struct ZyrinxVoice {
    pub index: u8,
    pub fb_alg: u8,
    pub dt_mul: [u8; 4],
    pub tl: [u8; 4],
    pub ks_ar: [u8; 4],
    pub am_d1r: [u8; 4],
    pub d2r: [u8; 4],
    pub sl_rr: [u8; 4],
    pub ams_fms_pan: u8,
}

struct BankInfo {
    song_count: usize,
    song_addrs: Vec<usize>,
    seq_table_rom: usize,
    seq_count: usize,
    voice_table_rom: usize,
    voice_count: usize,
}

fn signed8(v: u8) -> i8 {
    v as i8
}

fn parse_bank_header(rom: &[u8], bank_idx: usize) -> Result<BankInfo, String> {
    let base = BANK_ROM_BASES[bank_idx];
    if base + 18 > rom.len() {
        return Err(format!("ROM too short for bank {} header", bank_idx + 1));
    }

    let hdr = &rom[base..base + 18];
    let song_ptr_rom = ((hdr[0] as usize) << 24) | ((hdr[1] as usize) << 16)
        | ((hdr[2] as usize) << 8) | hdr[3] as usize;
    let song_count = ((hdr[4] as usize) << 8) | hdr[5] as usize;

    let seq_bank = hdr[7] as usize;
    let seq_z80 = ((hdr[8] as usize) << 8) | hdr[9] as usize;
    let seq_count = ((hdr[10] as usize) << 8) | hdr[11] as usize;
    let seq_table_rom = (seq_bank << 15) + seq_z80 - 0x8000;

    let voice_bank = hdr[13] as usize;
    let voice_z80 = ((hdr[14] as usize) << 8) | hdr[15] as usize;
    let voice_count = ((hdr[16] as usize) << 8) | hdr[17] as usize;
    let voice_table_rom = (voice_bank << 15) + voice_z80 - 0x8000;

    let mut song_addrs = Vec::with_capacity(song_count);
    for i in 0..song_count {
        let off = song_ptr_rom + i * 3;
        if off + 3 > rom.len() {
            return Err("song pointer table past ROM end".into());
        }
        let addr = ((rom[off] as usize) << 16) | ((rom[off + 1] as usize) << 8) | rom[off + 2] as usize;
        song_addrs.push(addr);
    }

    Ok(BankInfo { song_count, song_addrs, seq_table_rom, seq_count, voice_table_rom, voice_count })
}

fn resolve_sequence(rom: &[u8], bank: &BankInfo, seq_idx: usize) -> Option<usize> {
    if seq_idx >= bank.seq_count {
        return None;
    }
    let off = bank.seq_table_rom + seq_idx * 3;
    if off + 3 > rom.len() {
        return None;
    }
    let seq_bank = rom[off] as usize;
    let z80_hi = rom[off + 1] as usize;
    let z80_lo = rom[off + 2] as usize;
    let z80_addr = (z80_hi << 8) | z80_lo;
    Some((seq_bank << 15) + z80_addr - 0x8000)
}

fn read_voice(rom: &[u8], bank: &BankInfo, voice_idx: usize) -> Option<ZyrinxVoice> {
    if voice_idx >= bank.voice_count {
        return None;
    }
    let ptr_off = bank.voice_table_rom + voice_idx * 3;
    if ptr_off + 3 > rom.len() {
        return None;
    }
    let vbank = rom[ptr_off] as usize;
    let vz80 = ((rom[ptr_off + 1] as usize) << 8) | rom[ptr_off + 2] as usize;
    let rom_addr = (vbank << 15) + vz80 - 0x8000;
    if rom_addr + 30 > rom.len() {
        return None;
    }
    let d = &rom[rom_addr..rom_addr + 30];
    Some(ZyrinxVoice {
        index: voice_idx as u8,
        fb_alg: d[0],
        dt_mul: [d[1], d[2], d[3], d[4]],
        tl: [d[5], d[6], d[7], d[8]],
        ks_ar: [d[9], d[10], d[11], d[12]],
        am_d1r: [d[13], d[14], d[15], d[16]],
        d2r: [d[17], d[18], d[19], d[20]],
        sl_rr: [d[21], d[22], d[23], d[24]],
        ams_fms_pan: d[25],
    })
}

#[allow(dead_code)]
struct SequenceEvents {
    events: Vec<ZyrinxEvent>,
    total_frames: u16,
}

fn decode_sequence(rom: &[u8], start: usize) -> SequenceEvents {
    let mut events = Vec::new();
    let mut pos = start;
    let end = (start + 512).min(rom.len());
    let mut in_table2 = false;
    let mut total_frames: u16 = 0;
    let mut pending_pitch: Option<u8> = None;

    while pos < end {
        let b = rom[pos];

        if b >= 0x80 {
            let wait = (0xFF - b) as u16;
            if let Some(pitch) = pending_pitch.take() {
                events.push(ZyrinxEvent::Note { pitch, duration_frames: wait });
            } else {
                events.push(ZyrinxEvent::Rest { duration_frames: wait });
            }
            total_frames += wait;
            pos += 1;
            in_table2 = false;
            continue;
        }

        if in_table2 && b <= 0x08 {
            let n = match b { 0x00 => 1, 0x02 => 2, 0x04 => 3, 0x06 => 4, 0x08 => 5, _ => 1 };
            if pos + n >= rom.len() { break; }
            pending_pitch = Some(rom[pos + 1]);
            pos += 1 + n;
            in_table2 = true;
            continue;
        }

        if in_table2 && b == 0x1C {
            pos += 1;
            continue;
        }
        if in_table2 && b == 0x1E {
            pos += 1;
            continue;
        }

        if b <= 0x08 {
            let n = match b { 0x00 => 1, 0x02 => 2, 0x04 => 3, 0x06 => 4, 0x08 => 5, _ => 1 };
            if pos + n >= rom.len() { break; }
            pending_pitch = Some(rom[pos + 1]);
            pos += 1 + n;
            in_table2 = true;
            continue;
        }

        if (0x0A..=0x12).contains(&b) {
            let n = match b { 0x0A => 1, 0x0C => 2, 0x0E => 3, 0x10 => 4, 0x12 => 5, _ => 1 };
            let total = n + 1;
            if pos + total >= rom.len() { break; }
            let levels: Vec<u8> = (0..n).map(|i| rom[pos + 1 + i]).collect();
            let dur = rom[pos + 1 + n];
            events.push(ZyrinxEvent::Volume { levels, duration: dur });
            pos += 1 + total;
            in_table2 = true;
            continue;
        }

        if b == 0x14 {
            if pos + 1 >= rom.len() { break; }
            pos += 2;
            in_table2 = true;
            continue;
        }

        if b == 0x16 || b == 0x1E {
            pos += 1;
            in_table2 = true;
            continue;
        }

        if b == 0x18 {
            if pos + 1 >= rom.len() { break; }
            events.push(ZyrinxEvent::Voice(rom[pos + 1]));
            pos += 2;
            in_table2 = true;
            continue;
        }

        if b == 0x1A {
            events.push(ZyrinxEvent::Flag);
            pos += 1;
            in_table2 = true;
            continue;
        }

        if b == 0x1C {
            break;
        }

        if (0x20..=0x2E).contains(&b) {
            let ch = (b - 0x20) / 2 + 1;
            events.push(ZyrinxEvent::ChSelect(ch));
            pos += 1;
            in_table2 = true;
            continue;
        }

        if (0x30..=0x36).contains(&b) {
            let pan = match b {
                0x30 => ZyrinxPan::Off,
                0x32 => ZyrinxPan::Right,
                0x34 => ZyrinxPan::Left,
                0x36 => ZyrinxPan::Center,
                _ => ZyrinxPan::Center,
            };
            events.push(ZyrinxEvent::Pan(pan));
            pos += 1;
            in_table2 = true;
            continue;
        }

        if (0x38..=0x3E).contains(&b) {
            let op = (b - 0x38) / 2 + 1;
            if pos + 1 >= rom.len() { break; }
            events.push(ZyrinxEvent::FmOp { operator: op, value: rom[pos + 1] });
            pos += 2;
            in_table2 = true;
            continue;
        }

        pos += 1;
        in_table2 = true;
    }

    SequenceEvents { events, total_frames }
}

// Game song index: (bank 0-2, song_within_bank)
const SONG_INDEX: [(usize, usize); 20] = [
    (0, 0), (0, 0), (0, 1), (0, 2), (1, 5),
    (0, 3), (0, 4), (0, 5), (0, 6), (1, 0),
    (1, 1), (1, 2), (1, 3), (1, 4), (2, 0),
    (2, 1), (2, 4), (2, 3), (2, 2), (1, 6),
];

pub const GAME_SONG_NAMES: [&str; 20] = [
    "Silence", "Main Title", "Gotham by Night", "Big Boss",
    "Joker's Theme", "Moving Trucks", "Two-Face's Theme", "Not Much Time",
    "Flying Over The City", "Moving Boss", "Extreme Boss", "Dark Studio",
    "Animal Boss", "Psycho Section", "Space Boss", "The Lab",
    "Big Machines", "Final Boss", "Ending", "Continue",
];

pub fn parse_zyrinx_song(rom: &[u8], game_id: u8) -> Result<ZyrinxSong, String> {
    if game_id as usize >= SONG_INDEX.len() {
        return Err(format!("invalid game song ID: {} (valid: 0-19)", game_id));
    }

    let (bank_idx, song_idx) = SONG_INDEX[game_id as usize];
    let bank = parse_bank_header(rom, bank_idx)?;

    if song_idx >= bank.song_count {
        return Err(format!("song {} not in bank {} (has {})", song_idx, bank_idx + 1, bank.song_count));
    }

    let song_addr = bank.song_addrs[song_idx];
    let song_end = if song_idx + 1 < bank.song_count {
        bank.song_addrs[song_idx + 1]
    } else {
        bank.seq_table_rom
    };
    let song_size = song_end - song_addr;

    if song_addr + 8 > rom.len() {
        return Err("song data past ROM end".into());
    }

    let data = &rom[song_addr..song_addr + song_size];
    let per_ch_count = data[0] as usize;
    let detune = signed8(data[3]);
    let tempo_base = data[5];
    let voice_base = data[6];
    let loop_point = data[7];

    let entry_count = (song_size - 8) / 4;

    // Collect all unique sequence indices and voice indices
    let mut seq_indices = std::collections::HashSet::new();
    let mut voice_indices = std::collections::HashSet::new();

    #[allow(dead_code)]
    struct PatternEntry {
        seq_idx: usize,
        pitch: i8,
        repeat: u8,
        tempo_delta: i8,
    }

    let mut all_entries = Vec::new();
    for i in 0..entry_count {
        let off = 8 + i * 4;
        let seq = data[off] as usize;
        let pitch = signed8(data[off + 1]);
        let repeat = data[off + 2];
        let tempo = signed8(data[off + 3]);
        if seq == 0 && pitch == 0 && repeat == 0 && tempo == 0 {
            continue;
        }
        seq_indices.insert(seq);
        all_entries.push(PatternEntry { seq_idx: seq, pitch, repeat, tempo_delta: tempo });
    }

    // Pre-decode all sequences to collect voices
    let mut decoded_seqs: std::collections::HashMap<usize, SequenceEvents> = std::collections::HashMap::new();
    for &seq_idx in &seq_indices {
        if let Some(rom_addr) = resolve_sequence(rom, &bank, seq_idx) {
            let seq_events = decode_sequence(rom, rom_addr);
            for evt in &seq_events.events {
                if let ZyrinxEvent::Voice(v) = evt {
                    voice_indices.insert(*v as usize);
                }
            }
            decoded_seqs.insert(seq_idx, seq_events);
        }
    }

    // Build channels by expanding pattern entries into flat event lists
    let num_channels = if per_ch_count > 0 {
        ((all_entries.len() + per_ch_count - 1) / per_ch_count).min(6)
    } else {
        1
    };

    let mut channels = Vec::new();
    for ch_idx in 0..num_channels {
        let start = ch_idx * per_ch_count;
        let end = (start + per_ch_count).min(all_entries.len());
        if start >= all_entries.len() {
            break;
        }

        let mut events = Vec::new();
        for entry in &all_entries[start..end] {
            if let Some(seq) = decoded_seqs.get(&entry.seq_idx) {
                for _ in 0..entry.repeat.max(1) {
                    for evt in &seq.events {
                        match evt {
                            ZyrinxEvent::Note { pitch, duration_frames } => {
                                let transposed = (*pitch as i16 + entry.pitch as i16)
                                    .clamp(0, 127) as u8;
                                events.push(ZyrinxEvent::Note {
                                    pitch: transposed,
                                    duration_frames: *duration_frames,
                                });
                            }
                            ZyrinxEvent::Rest { duration_frames } => {
                                events.push(ZyrinxEvent::Rest { duration_frames: *duration_frames });
                            }
                            ZyrinxEvent::Voice(v) => {
                                events.push(ZyrinxEvent::Voice(*v));
                            }
                            ZyrinxEvent::Pan(p) => {
                                events.push(ZyrinxEvent::Pan(*p));
                            }
                            ZyrinxEvent::FmOp { operator, value } => {
                                events.push(ZyrinxEvent::FmOp { operator: *operator, value: *value });
                            }
                            ZyrinxEvent::Volume { levels, duration } => {
                                events.push(ZyrinxEvent::Volume { levels: levels.clone(), duration: *duration });
                            }
                            ZyrinxEvent::ChSelect(ch) => {
                                events.push(ZyrinxEvent::ChSelect(*ch));
                            }
                            ZyrinxEvent::Flag => {
                                events.push(ZyrinxEvent::Flag);
                            }
                        }
                    }
                }
            }
        }

        channels.push(ZyrinxChannel { index: ch_idx as u8, events });
    }

    // Read all used voices
    let mut voices = Vec::new();
    for &vidx in &voice_indices {
        if let Some(voice) = read_voice(rom, &bank, vidx) {
            voices.push(voice);
        }
    }
    voices.sort_by_key(|v| v.index);

    Ok(ZyrinxSong {
        name: GAME_SONG_NAMES[game_id as usize].to_string(),
        game_id,
        tempo_base,
        detune,
        voice_base,
        loop_point,
        channels,
        voices,
    })
}
