use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;
use uuid::Uuid;

use crate::audio::frequency::{midi_to_fm_freq, midi_to_psg_period};
use crate::audio::{AudioCommand, AudioThread};
use crate::dac;
use crate::export::{ExportResult, ExportError};
use crate::library::entry::{content_hash, LibraryEntryFile, LibraryInstrument, Provenance, LIBRARY_SCHEMA};
use crate::library::state::{self, LibraryState, RootInfo};
use crate::library::store::{self, LibraryFilter, LibraryListEntry};
use crate::model::driver::{ChannelLayout, DriverFeature};
use crate::model::instrument::*;
use crate::model::song::{Song, SongMetadata};
use crate::project::ProjectManager;
use crate::sequencer::OverlapWarning;

pub struct AudioState {
    pub thread: Mutex<AudioThread>,
}

pub struct ProjectState {
    pub manager: Mutex<ProjectManager>,
}

// --- FM register helpers ---
// YM2612 register write sequence:
//   port 0 = Part I address
//   port 1 = Part I data
//   port 2 = Part II address (channels 3-5)
//   port 3 = Part II data

/// Send (addr, data) to YM2612 Part I (channels 0-2).
fn ym_write(thread: &mut AudioThread, addr: u8, data: u8) {
    thread.send(AudioCommand::Ym2612Write { port: 0, data: addr });
    thread.send(AudioCommand::Ym2612Write { port: 1, data: data });
}

#[tauri::command]
#[specta::specta]
pub fn play_fm_test_tone(state: State<'_, AudioState>) -> Result<String, String> {
    let mut thread = state
        .thread
        .lock()
        .map_err(|e| format!("mutex poisoned: {e}"))?;

    // --- Global FM enable ---
    // Register $27: channel 3 mode 0, timers off
    ym_write(&mut thread, 0x27, 0x00);

    // === Channel 0 (Part I) operator layout ===
    // Operators are addressed per-channel:
    //   CH1: op1=$30+0, op2=$38+0, op3=$34+0, op4=$3C+0
    //   Offsets: op1=+0, op3=+4, op2=+8, op4=+12  (YM2612 numbering)

    // Algorithm 7 (all ops in parallel → pure additive), Feedback 0
    // Register $B0 + channel: [feedback(2:0) | algorithm(2:0)]
    ym_write(&mut thread, 0xB0, 0b000_00_111); // FB=0, ALG=7

    // Stereo L+R, AM/FM sensitivity defaults
    // Register $B4 + channel: [L|R | AMS(1:0) | FMS(2:0)]
    ym_write(&mut thread, 0xB4, 0xC0); // L+R on, no AMS/FMS

    // --- Operator 1 (slot 0, register base offset 0) ---
    // $30: DT1/MUL  DT=0, MUL=1
    ym_write(&mut thread, 0x30, 0x01);
    // $40: Total Level (attenuation). 0=max volume, 0x7F=silent.
    ym_write(&mut thread, 0x40, 0x10); // slight attenuation
    // $50: AR/RS  AR=0x1F (max attack), RS=0
    ym_write(&mut thread, 0x50, 0x1F);
    // $60: DR/AM  DR=5, AM=0
    ym_write(&mut thread, 0x60, 0x05);
    // $70: SR (second decay rate) = 2
    ym_write(&mut thread, 0x70, 0x02);
    // $80: SL/RR  SL=2, RR=10
    ym_write(&mut thread, 0x80, (2 << 4) | 10);

    // --- Operator 2 (slot 2, register base offset +8) ---
    ym_write(&mut thread, 0x38, 0x01); // DT=0, MUL=1
    ym_write(&mut thread, 0x48, 0x10); // TL
    ym_write(&mut thread, 0x58, 0x1F); // AR
    ym_write(&mut thread, 0x68, 0x05); // DR
    ym_write(&mut thread, 0x78, 0x02); // SR
    ym_write(&mut thread, 0x88, (2 << 4) | 10); // SL/RR

    // --- Operator 3 (slot 1, register base offset +4) ---
    ym_write(&mut thread, 0x34, 0x01);
    ym_write(&mut thread, 0x44, 0x10);
    ym_write(&mut thread, 0x54, 0x1F);
    ym_write(&mut thread, 0x64, 0x05);
    ym_write(&mut thread, 0x74, 0x02);
    ym_write(&mut thread, 0x84, (2 << 4) | 10);

    // --- Operator 4 (slot 3, register base offset +12) ---
    ym_write(&mut thread, 0x3C, 0x01);
    ym_write(&mut thread, 0x4C, 0x10);
    ym_write(&mut thread, 0x5C, 0x1F);
    ym_write(&mut thread, 0x6C, 0x05);
    ym_write(&mut thread, 0x7C, 0x02);
    ym_write(&mut thread, 0x8C, (2 << 4) | 10);

    // --- Frequency ~440 Hz ---
    // F-num formula: F-num = (freq * 2^20) / (clock / 144)
    // YM2612 clock = 7.67 MHz / 2 = 3.58 MHz (NTSC), /144 = ~24.8 kHz
    // Block 4, F-num ≈ 0x28A gives ~440 Hz
    // $A4: block/F-num MSB,  $A0: F-num LSB
    // Write MSB first (latches), then LSB (commits)
    ym_write(&mut thread, 0xA4, (4 << 3) | 0x02); // Block=4, F-num[9:8]=0x02 → 0x028A
    ym_write(&mut thread, 0xA0, 0x8A);              // F-num[7:0]

    // --- Key On: channel 0, all 4 operators ---
    // $28: [op-mask(7:4) | 0 | channel(2:0)]
    // op-mask 0xF0 = all four operators
    thread.send(AudioCommand::FmKeyOn { channel: 0, operators: 0xF0 });

    Ok("FM tone playing (~440 Hz, Algorithm 7)".to_string())
}

#[tauri::command]
#[specta::specta]
pub fn play_psg_test_tone(state: State<'_, AudioState>) -> Result<String, String> {
    let mut thread = state
        .thread
        .lock()
        .map_err(|e| format!("mutex poisoned: {e}"))?;

    // SN76489 tone period for ~440 Hz:
    // Period = clock / (32 * freq)
    // PSG clock on Genesis = master_clock / 15 ≈ 3.58 MHz / 15 ≈ 223,722 Hz
    // But the SN76489 is typically clocked at ~3.58 MHz and divides by 32:
    // Period = 3,579,545 / (32 * 440) ≈ 254 = 0x0FE
    //
    // Byte 1 (LATCH+DATA): 1 | channel(1:0)<<5 | 0 | data[3:0]
    //   channel 0 tone: 0x80 | (freq[3:0])
    // Byte 2 (DATA):    0 | freq[9:4]
    //
    // Period = 0x0FE: low nibble = 0xE, high 6 bits = 0x03
    let period: u16 = 0x00FE;
    let low_nibble = (period & 0x0F) as u8;
    let high_bits = ((period >> 4) & 0x3F) as u8;

    // Latch byte: channel 0, tone register, low 4 bits of period
    thread.send(AudioCommand::Sn76489Write {
        data: 0x80 | low_nibble,
    });
    // Data byte: remaining 6 bits of period
    thread.send(AudioCommand::Sn76489Write { data: high_bits });

    // Volume register for channel 0: 0x90 | attenuation
    // Attenuation 0 = maximum volume
    thread.send(AudioCommand::Sn76489Write { data: 0x90 | 0x00 });

    Ok("PSG tone playing (~440 Hz, channel 0)".to_string())
}

#[tauri::command]
#[specta::specta]
pub fn stop_all_sound(state: State<'_, AudioState>) -> Result<String, String> {
    let mut thread = state
        .thread
        .lock()
        .map_err(|e| format!("mutex poisoned: {e}"))?;

    thread.send(AudioCommand::Panic);

    Ok("All sound stopped".to_string())
}

// --- Project Management ---

#[tauri::command]
#[specta::specta]
pub fn create_project(
    state: State<'_, ProjectState>,
    path: String,
    name: String,
    driver_id: String,
    tempo: f64,
    time_sig_num: u8,
    time_sig_den: u8,
) -> Result<(), String> {
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.create(&PathBuf::from(path), &name, &driver_id, tempo, (time_sig_num, time_sig_den))
}

#[tauri::command]
#[specta::specta]
pub fn open_project(state: State<'_, ProjectState>, path: String) -> Result<Song, String> {
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.open(&PathBuf::from(path))
}

#[tauri::command]
#[specta::specta]
pub fn save_project(state: State<'_, ProjectState>) -> Result<(), String> {
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.save()
}

#[tauri::command]
#[specta::specta]
pub fn close_project(state: State<'_, ProjectState>) -> Result<(), String> {
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.close();
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_project_info(state: State<'_, ProjectState>) -> Result<Option<SongMetadata>, String> {
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    Ok(mgr.metadata().cloned())
}

// --- Driver Info ---

#[derive(serde::Serialize, specta::Type)]
pub struct DriverInfo {
    pub id: String,
    pub name: String,
}

#[tauri::command]
#[specta::specta]
pub fn list_drivers(state: State<'_, ProjectState>) -> Result<Vec<DriverInfo>, String> {
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    Ok(mgr
        .driver_registry()
        .list()
        .into_iter()
        .map(|(id, name)| DriverInfo {
            id: id.to_string(),
            name: name.to_string(),
        })
        .collect())
}

#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DriverDetail {
    pub id: String,
    pub name: String,
    pub layout: ChannelLayout,
    pub features: Vec<DriverFeature>,
}

#[tauri::command]
#[specta::specta]
pub fn get_driver_info(state: State<'_, ProjectState>, driver_id: String) -> Result<DriverDetail, String> {
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let driver = mgr
        .driver_registry()
        .get(&driver_id)
        .ok_or_else(|| format!("unknown driver: {driver_id}"))?;

    let all_features = [
        DriverFeature::SsgEg,
        DriverFeature::Fm3SpecialMode,
        DriverFeature::MultiDac,
        DriverFeature::Dpcm,
        DriverFeature::PseudoStereo,
    ];
    let features: Vec<DriverFeature> = all_features
        .into_iter()
        .filter(|&f| driver.supports_feature(f))
        .collect();

    Ok(DriverDetail {
        id: driver.id().to_string(),
        name: driver.name().to_string(),
        layout: driver.channel_layout(),
        features,
    })
}

// --- FM Instrument CRUD ---

#[tauri::command]
#[specta::specta]
pub fn add_fm_instrument(
    state: State<'_, ProjectState>,
    instrument: FmInstrument,
) -> Result<String, String> {
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let id = mgr.add_fm_instrument(instrument);
    Ok(id.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn update_fm_instrument(
    state: State<'_, ProjectState>,
    id: String,
    instrument: FmInstrument,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.update_fm_instrument(uuid, instrument)
}

#[tauri::command]
#[specta::specta]
pub fn delete_fm_instrument(state: State<'_, ProjectState>, id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.delete_fm_instrument(uuid)
}

#[tauri::command]
#[specta::specta]
pub fn list_fm_instruments(state: State<'_, ProjectState>) -> Result<Vec<FmInstrument>, String> {
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    Ok(mgr.list_fm_instruments().to_vec())
}

const OP_REG_OFFSETS: [u8; 4] = [0x00, 0x08, 0x04, 0x0C];

fn ym_write_port(thread: &mut AudioThread, port: u8, addr: u8, data: u8) {
    thread.send(AudioCommand::Ym2612Write { port: port as u32, data: addr });
    thread.send(AudioCommand::Ym2612Write { port: (port + 1) as u32, data });
}

/// Stateless FM preview: program the patch onto channel 0 and key on.
/// Shared by the project preview command and the library audition command.
fn do_preview_fm(audio_state: &AudioState, inst: &FmInstrument, midi_note: u8) -> Result<(), String> {
    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;

    let ch: u8 = 0;
    let port: u8 = 0;

    thread.send(AudioCommand::FmKeyOff { channel: ch });

    ym_write_port(&mut thread, port, 0xB0 + ch, (inst.feedback << 3) | inst.algorithm);
    ym_write_port(&mut thread, port, 0xB4 + ch, 0xC0);

    for (i, op) in inst.operators.iter().enumerate() {
        let slot = OP_REG_OFFSETS[i] + ch;
        ym_write_port(&mut thread, port, 0x30 + slot, (op.detune << 4) | op.multiple);
        ym_write_port(&mut thread, port, 0x40 + slot, op.total_level);
        ym_write_port(&mut thread, port, 0x50 + slot, (op.rate_scale << 6) | op.attack_rate);
        ym_write_port(&mut thread, port, 0x60 + slot, ((op.amp_mod as u8) << 7) | op.d1r);
        ym_write_port(&mut thread, port, 0x70 + slot, op.d2r);
        ym_write_port(&mut thread, port, 0x80 + slot, (op.sustain_level << 4) | op.release_rate);
    }

    let (block, fnum) = midi_to_fm_freq(midi_note);
    let freq_msb = (block << 3) | ((fnum >> 8) as u8 & 0x07);
    let freq_lsb = (fnum & 0xFF) as u8;
    ym_write_port(&mut thread, port, 0xA4 + ch, freq_msb);
    ym_write_port(&mut thread, port, 0xA0 + ch, freq_lsb);

    thread.send(AudioCommand::FmKeyOn { channel: ch, operators: 0xF0 });

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn preview_fm_instrument(
    audio_state: State<'_, AudioState>,
    project_state: State<'_, ProjectState>,
    id: String,
    midi_note: u8,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mgr = project_state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let inst = mgr
        .get_fm_instrument(&uuid)
        .ok_or("FM instrument not found")?
        .clone();
    drop(mgr);

    do_preview_fm(&audio_state, &inst, midi_note)
}

#[tauri::command]
#[specta::specta]
pub fn stop_fm_preview(
    audio_state: State<'_, AudioState>,
) -> Result<(), String> {
    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    thread.send(AudioCommand::FmKeyOff { channel: 0 });
    Ok(())
}

/// Stop a library audition (FM or PSG) without resetting the whole mix.
///
/// - Forces release rate $F on ch0's four operators before keying off:
///   imported patches (GYB/TFI) can carry RR=0 and would ring indefinitely on
///   `FmKeyOff` alone. `do_preview_fm` reprograms the full patch on the next
///   audition anyway, so clobbering SL/RR here is safe.
/// - Sends `StopPreview`, which clears a looping PSG envelope preview (and
///   any DAC preview) and invalidates the sequencer's ch0 FM patch cache so
///   playback recovers on ch0's next note-on.
///
/// `stop_all_sound` (`AudioCommand::Panic`) stays reserved for the global
/// panic button — it resets both chips but leaves the sequencer's patch cache
/// intact, which kills FM output until the next stop/seek.
#[tauri::command]
#[specta::specta]
pub fn library_stop_audition(
    audio_state: State<'_, AudioState>,
) -> Result<(), String> {
    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;

    let ch: u8 = 0;
    let port: u8 = 0;
    for &off in &OP_REG_OFFSETS {
        // SL/RR register ($80+slot): $FF = SL 15, RR 15 (fastest release).
        ym_write_port(&mut thread, port, 0x80 + off + ch, 0xFF);
    }
    thread.send(AudioCommand::FmKeyOff { channel: ch });
    thread.send(AudioCommand::StopPreview);

    Ok(())
}

// --- PSG Instrument CRUD ---

#[tauri::command]
#[specta::specta]
pub fn add_psg_instrument(
    state: State<'_, ProjectState>,
    instrument: PsgInstrument,
) -> Result<String, String> {
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let id = mgr.add_psg_instrument(instrument);
    Ok(id.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn update_psg_instrument(
    state: State<'_, ProjectState>,
    id: String,
    instrument: PsgInstrument,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.update_psg_instrument(uuid, instrument)
}

#[tauri::command]
#[specta::specta]
pub fn delete_psg_instrument(state: State<'_, ProjectState>, id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.delete_psg_instrument(uuid)
}

#[tauri::command]
#[specta::specta]
pub fn list_psg_instruments(state: State<'_, ProjectState>) -> Result<Vec<PsgInstrument>, String> {
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    Ok(mgr.list_psg_instruments().to_vec())
}

/// Stateless PSG preview: envelope playback on channel 0.
/// Shared by the project preview command and the library audition command.
fn do_preview_psg(audio_state: &AudioState, inst: &PsgInstrument, midi_note: u8) -> Result<(), String> {
    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;

    let period = midi_to_psg_period(midi_note);
    let channel: u8 = 0;

    thread.send(AudioCommand::PsgEnvelopePreview {
        channel,
        period,
        envelope: Arc::new(inst.volume_sequence.clone()),
        loop_point: inst.loop_point,
    });

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn preview_psg_instrument(
    audio_state: State<'_, AudioState>,
    project_state: State<'_, ProjectState>,
    id: String,
    midi_note: u8,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mgr = project_state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let inst = mgr
        .get_psg_instrument(&uuid)
        .ok_or("PSG instrument not found")?
        .clone();
    drop(mgr);

    do_preview_psg(&audio_state, &inst, midi_note)
}

// --- DAC Instrument CRUD ---

#[tauri::command]
#[specta::specta]
pub fn import_dac_wav(
    state: State<'_, ProjectState>,
    wav_path: String,
    target_rate: u32,
) -> Result<String, String> {
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let project_path = mgr.project_path().ok_or("no project open")?.to_path_buf();

    let pcm_data = dac::import_wav(std::path::Path::new(&wav_path), target_rate)?;

    let id = Uuid::new_v4();
    let original_filename = std::path::Path::new(&wav_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("{id}.wav"));

    let dest_wav = format!("{id}.wav");
    std::fs::copy(&wav_path, project_path.join("instruments/dac").join(&dest_wav))
        .map_err(|e| format!("failed to copy WAV: {e}"))?;

    let pcm_filename = format!("{id}.pcm");
    std::fs::write(
        project_path.join("instruments/dac").join(&pcm_filename),
        &pcm_data,
    )
    .map_err(|e| format!("failed to write PCM: {e}"))?;

    let inst = DacInstrument {
        id,
        name: original_filename,
        target_sample_rate: target_rate,
        loop_start: None,
        loop_length: None,
        original_file: dest_wav,
        pcm_file: pcm_filename,
        source_is_raw: false,
        metadata: InstrumentMetadata::default(),
    };

    mgr.add_dac_instrument(inst, pcm_data);
    Ok(id.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn import_dac_raw(
    state: State<'_, ProjectState>,
    pcm_path: String,
    sample_rate: u32,
) -> Result<String, String> {
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let project_path = mgr.project_path().ok_or("no project open")?.to_path_buf();

    let pcm_data = dac::import_raw(std::path::Path::new(&pcm_path))?;

    let id = Uuid::new_v4();
    let original_filename = std::path::Path::new(&pcm_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("{id}.raw"));

    let dest_raw = format!("{id}.raw");
    std::fs::copy(&pcm_path, project_path.join("instruments/dac").join(&dest_raw))
        .map_err(|e| format!("failed to copy raw PCM: {e}"))?;

    let pcm_filename = format!("{id}.pcm");
    std::fs::write(
        project_path.join("instruments/dac").join(&pcm_filename),
        &pcm_data,
    )
    .map_err(|e| format!("failed to write PCM: {e}"))?;

    let inst = DacInstrument {
        id,
        name: original_filename,
        target_sample_rate: sample_rate,
        loop_start: None,
        loop_length: None,
        original_file: dest_raw,
        pcm_file: pcm_filename,
        source_is_raw: true,
        metadata: InstrumentMetadata::default(),
    };

    mgr.add_dac_instrument(inst, pcm_data);
    Ok(id.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn update_dac_instrument(
    state: State<'_, ProjectState>,
    id: String,
    instrument: DacInstrument,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.update_dac_instrument(uuid, instrument)
}

#[tauri::command]
#[specta::specta]
pub fn reconvert_dac(state: State<'_, ProjectState>, id: String, new_rate: u32) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let project_path = mgr.project_path().ok_or("no project open")?.to_path_buf();

    let inst = mgr
        .get_dac_instrument(&uuid)
        .ok_or("DAC instrument not found")?
        .clone();

    if inst.source_is_raw {
        return Err("cannot reconvert raw PCM import (no higher-quality source)".into());
    }

    let wav_path = project_path.join("instruments/dac").join(&inst.original_file);
    let pcm_data = dac::import_wav(&wav_path, new_rate)?;

    std::fs::write(
        project_path.join("instruments/dac").join(&inst.pcm_file),
        &pcm_data,
    )
    .map_err(|e| format!("failed to write PCM: {e}"))?;

    let mut updated = inst;
    updated.target_sample_rate = new_rate;
    mgr.update_dac_instrument(uuid, updated)?;
    mgr.update_dac_pcm(uuid, pcm_data);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn delete_dac_instrument(state: State<'_, ProjectState>, id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.delete_dac_instrument(uuid)
}

#[tauri::command]
#[specta::specta]
pub fn list_dac_instruments(state: State<'_, ProjectState>) -> Result<Vec<DacInstrument>, String> {
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    Ok(mgr.list_dac_instruments().to_vec())
}

#[tauri::command]
#[specta::specta]
pub fn preview_dac(
    audio_state: State<'_, AudioState>,
    project_state: State<'_, ProjectState>,
    id: String,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mgr = project_state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let inst = mgr
        .get_dac_instrument(&uuid)
        .ok_or("DAC instrument not found")?;
    let pcm = mgr
        .get_dac_pcm(&uuid)
        .ok_or("DAC PCM data not loaded")?;
    let sample_rate = inst.target_sample_rate;
    drop(mgr);

    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    thread.send(AudioCommand::DacPlayback {
        samples: pcm,
        sample_rate,
    });

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_dac_pcm_data(
    state: State<'_, ProjectState>,
    id: String,
) -> Result<Vec<u8>, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let pcm = mgr
        .get_dac_pcm(&uuid)
        .ok_or("DAC PCM data not loaded")?;
    Ok(pcm.as_ref().clone())
}

// --- Track CRUD ---

#[tauri::command]
#[specta::specta]
pub fn add_track(
    state: State<'_, ProjectState>,
    name: String,
    channel: crate::model::song::ChannelAssignment,
    instrument_id: Option<String>,
) -> Result<String, String> {
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let inst_uuid = instrument_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| format!("invalid UUID: {e}"))?;
    let id = mgr.add_track(name, channel, inst_uuid);
    Ok(id.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn update_track(
    state: State<'_, ProjectState>,
    id: String,
    name: String,
    channel: crate::model::song::ChannelAssignment,
    instrument_id: Option<String>,
    muted: bool,
    solo: bool,
    volume: u8,
    pan: crate::model::song::Pan,
    pitch_offset: Option<i8>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let inst_uuid = instrument_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.update_track(uuid, name, channel, inst_uuid, muted, solo, volume, pan, pitch_offset.unwrap_or(0))
}

#[tauri::command]
#[specta::specta]
pub fn delete_track(state: State<'_, ProjectState>, id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.delete_track(uuid)
}

#[tauri::command]
#[specta::specta]
pub fn list_tracks(state: State<'_, ProjectState>) -> Result<Vec<crate::model::song::Track>, String> {
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    Ok(mgr.list_tracks().to_vec())
}

// --- Region CRUD ---

#[tauri::command]
#[specta::specta]
pub fn add_region(
    state: State<'_, ProjectState>,
    track_id: String,
    start_tick: u64,
    duration_ticks: u64,
) -> Result<String, String> {
    let uuid = Uuid::parse_str(&track_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let id = mgr.add_region(uuid, start_tick, duration_ticks)?;
    Ok(id.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn update_region(
    state: State<'_, ProjectState>,
    track_id: String,
    region_id: String,
    start_tick: u64,
    duration_ticks: u64,
) -> Result<(), String> {
    let t_uuid = Uuid::parse_str(&track_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let r_uuid = Uuid::parse_str(&region_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.update_region(t_uuid, r_uuid, start_tick, duration_ticks)
}

#[tauri::command]
#[specta::specta]
pub fn move_region(
    state: State<'_, ProjectState>,
    src_track_id: String,
    region_id: String,
    dst_track_id: String,
    start_tick: u64,
) -> Result<(), String> {
    let src = Uuid::parse_str(&src_track_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let r = Uuid::parse_str(&region_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let dst = Uuid::parse_str(&dst_track_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.move_region(src, r, dst, start_tick)
}

#[tauri::command]
#[specta::specta]
pub fn delete_region(
    state: State<'_, ProjectState>,
    track_id: String,
    region_id: String,
) -> Result<(), String> {
    let t_uuid = Uuid::parse_str(&track_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let r_uuid = Uuid::parse_str(&region_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.delete_region(t_uuid, r_uuid)
}

// --- Note CRUD ---

#[tauri::command]
#[specta::specta]
pub fn add_note(
    state: State<'_, ProjectState>,
    track_id: String,
    region_id: String,
    tick: u64,
    pitch: u8,
    velocity: u8,
    duration_ticks: u64,
) -> Result<usize, String> {
    let t_uuid = Uuid::parse_str(&track_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let r_uuid = Uuid::parse_str(&region_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.add_note(t_uuid, r_uuid, tick, pitch, velocity, duration_ticks)
}

#[tauri::command]
#[specta::specta]
pub fn update_note(
    state: State<'_, ProjectState>,
    track_id: String,
    region_id: String,
    note_index: usize,
    tick: u64,
    pitch: u8,
    velocity: u8,
    duration_ticks: u64,
) -> Result<(), String> {
    let t_uuid = Uuid::parse_str(&track_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let r_uuid = Uuid::parse_str(&region_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.update_note(t_uuid, r_uuid, note_index, tick, pitch, velocity, duration_ticks)
}

#[tauri::command]
#[specta::specta]
pub fn delete_note(
    state: State<'_, ProjectState>,
    track_id: String,
    region_id: String,
    note_index: usize,
) -> Result<(), String> {
    let t_uuid = Uuid::parse_str(&track_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let r_uuid = Uuid::parse_str(&region_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.delete_note(t_uuid, r_uuid, note_index)
}

// --- Transport ---

#[tauri::command]
#[specta::specta]
pub fn transport_play(
    audio_state: State<'_, AudioState>,
    project_state: State<'_, ProjectState>,
) -> Result<(), String> {
    let mgr = project_state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let snapshot = mgr.build_snapshot();
    drop(mgr);

    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    thread.send(AudioCommand::LoadSequence { snapshot });
    thread.send(AudioCommand::TransportPlay);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn transport_stop(audio_state: State<'_, AudioState>) -> Result<(), String> {
    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    thread.send(AudioCommand::TransportStop);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn transport_seek(audio_state: State<'_, AudioState>, tick: u64) -> Result<(), String> {
    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    thread.send(AudioCommand::TransportSeek { tick });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn reload_sequence(
    audio_state: State<'_, AudioState>,
    project_state: State<'_, ProjectState>,
) -> Result<(), String> {
    let mgr = project_state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let snapshot = mgr.build_snapshot();
    drop(mgr);
    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    thread.send(AudioCommand::ReloadSequence { snapshot });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn transport_set_loop(audio_state: State<'_, AudioState>, start_tick: u64, end_tick: u64) -> Result<(), String> {
    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    thread.send(AudioCommand::TransportSetLoop { start_tick, end_tick });
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn transport_clear_loop(audio_state: State<'_, AudioState>) -> Result<(), String> {
    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    thread.send(AudioCommand::TransportClearLoop);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn set_master_volume(audio_state: State<'_, AudioState>, volume: f32) -> Result<(), String> {
    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    thread.send(AudioCommand::SetMasterVolume { volume });
    Ok(())
}

#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackState {
    pub playing: bool,
    pub tick: u64,
    pub loop_start: Option<u64>,
    pub loop_end: Option<u64>,
    pub channel_levels: Vec<u8>,
}

#[tauri::command]
#[specta::specta]
pub fn get_playback_state(audio_state: State<'_, AudioState>) -> Result<PlaybackState, String> {
    let thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let tick = thread.position_tick().load(std::sync::atomic::Ordering::Relaxed);
    let levels: Vec<u8> = thread.channel_levels().iter()
        .map(|a| a.load(std::sync::atomic::Ordering::Relaxed))
        .collect();
    Ok(PlaybackState {
        playing: false,
        tick,
        loop_start: None,
        loop_end: None,
        channel_levels: levels,
    })
}

#[tauri::command]
#[specta::specta]
pub fn get_channel_overlaps(state: State<'_, ProjectState>) -> Result<Vec<OverlapWarning>, String> {
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    Ok(mgr.get_all_overlaps())
}

// --- Export ---

#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ExportFailure {
    pub errors: Vec<ExportError>,
}

#[tauri::command]
#[specta::specta]
pub fn export_song(
    state: State<'_, ProjectState>,
    output_dir: String,
) -> Result<ExportResult, ExportFailure> {
    let mgr = state.manager.lock().map_err(|e| ExportFailure {
        errors: vec![ExportError {
            track_name: String::new(), region_index: None, note_index: None,
            message: format!("mutex poisoned: {e}"),
        }],
    })?;

    let song = mgr.song().ok_or_else(|| ExportFailure {
        errors: vec![ExportError {
            track_name: String::new(), region_index: None, note_index: None,
            message: "No project open".into(),
        }],
    })?;

    let driver_id = &song.metadata.driver_id;
    let registry = mgr.driver_registry();
    let driver = registry.get(driver_id).ok_or_else(|| ExportFailure {
        errors: vec![ExportError {
            track_name: String::new(), region_index: None, note_index: None,
            message: format!("Driver '{driver_id}' not found"),
        }],
    })?;

    let path = std::path::PathBuf::from(&output_dir);
    driver.export_song(&song, &song.instruments, &path).map_err(|errors| ExportFailure { errors })
}

// --- WAV Export ---

#[tauri::command]
#[specta::specta]
pub fn export_wav(
    project_state: State<'_, ProjectState>,
    output_path: String,
    duration_seconds: f64,
) -> Result<String, String> {
    use crate::audio::engine::AudioEngine;

    let mgr = project_state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let snapshot = mgr.build_snapshot();
    drop(mgr);

    let sample_rate: u32 = 44100;
    let mut engine = AudioEngine::new(sample_rate);

    engine.process_command(AudioCommand::LoadSequence { snapshot });
    engine.process_command(AudioCommand::TransportPlay);

    let total_samples = (sample_rate as f64 * duration_seconds) as usize;
    let mut all_samples = Vec::with_capacity(total_samples * 2);
    let chunk_size = 1024;
    let mut buf = vec![0.0f32; chunk_size * 2];
    let mut rendered = 0;

    while rendered < total_samples {
        let frames_this_chunk = (total_samples - rendered).min(chunk_size);
        let slice = &mut buf[..frames_this_chunk * 2];
        for s in slice.iter_mut() { *s = 0.0; }
        engine.render(slice);
        all_samples.extend_from_slice(slice);
        rendered += frames_this_chunk;
    }

    let path = std::path::Path::new(&output_path);
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|e| format!("failed to create WAV: {e}"))?;

    for &sample in &all_samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let int_sample = (clamped * 32767.0) as i16;
        writer.write_sample(int_sample).map_err(|e| format!("WAV write error: {e}"))?;
    }
    writer.finalize().map_err(|e| format!("WAV finalize error: {e}"))?;

    Ok(format!("Exported {} seconds to {}", duration_seconds, output_path))
}

// --- Import ---

#[tauri::command]
#[specta::specta]
pub fn import_song(
    state: State<'_, ProjectState>,
    source_path: String,
    parent_dir: String,
    dac_dir: Option<String>,
) -> Result<crate::import::ImportResult, String> {
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let registry = mgr.driver_registry();
    let driver = registry.get("flamedriver")
        .ok_or("Flamedriver driver not found")?;

    let source = std::path::PathBuf::from(&source_path);
    let parent = std::path::PathBuf::from(&parent_dir);
    let dac_path = dac_dir.as_ref().map(std::path::PathBuf::from);

    crate::import::import_smps_file_with_dac(&source, &parent, driver, dac_path.as_deref())
}

// --- FM File Import ---

#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FmFileImportResponse {
    pub format: String,
    pub count: usize,
    pub ids: Vec<String>,
}

#[tauri::command]
#[specta::specta]
pub fn import_fm_file(
    state: State<'_, ProjectState>,
    file_path: String,
) -> Result<FmFileImportResponse, String> {
    let path = std::path::Path::new(&file_path);
    let data = std::fs::read(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    let filename = path.file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("unknown");

    let result = crate::import::fm_formats::import_fm_file(&data, filename)?;

    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let mut ids = Vec::new();
    for inst in result.instruments {
        let id = mgr.add_fm_instrument(inst);
        ids.push(id.to_string());
    }

    Ok(FmFileImportResponse {
        format: result.format,
        count: ids.len(),
        ids,
    })
}

// --- VGM Export ---

#[tauri::command]
#[specta::specta]
pub fn export_vgm(
    project_state: State<'_, ProjectState>,
    output_path: String,
    duration_seconds: f64,
) -> Result<String, String> {
    let mgr = project_state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let song = mgr.song().ok_or("No project open")?;
    let instruments = song.instruments.clone();
    let data = crate::export::vgm::export_vgm_data(&song, &instruments, Some(duration_seconds))?;
    drop(mgr);

    std::fs::write(&output_path, &data)
        .map_err(|e| format!("failed to write VGM: {e}"))?;

    Ok(format!("Exported VGM ({} bytes) to {}", data.len(), output_path))
}

#[tauri::command]
#[specta::specta]
pub fn import_zyrinx_song(
    rom_path: String,
    parent_dir: String,
    game_id: u8,
) -> Result<crate::import::ImportResult, String> {
    let rom = std::path::PathBuf::from(&rom_path);
    let parent = std::path::PathBuf::from(&parent_dir);
    crate::import::import_zyrinx_rom(&rom, &parent, game_id)
}

#[tauri::command]
#[specta::specta]
pub fn import_vgm(
    vgm_path: String,
    parent_dir: String,
) -> Result<crate::import::ImportResult, String> {
    let path = std::path::PathBuf::from(&vgm_path);
    let parent = std::path::PathBuf::from(&parent_dir);
    crate::import::vgm_import::import_vgm_file(&path, &parent)
}

// --- Instrument Library ---

#[tauri::command]
#[specta::specta]
pub fn library_list(
    lib: State<'_, LibraryState>,
    filter: LibraryFilter,
) -> Result<Vec<LibraryListEntry>, String> {
    let idx = lib.index.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let ov = lib.overrides.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let all: Vec<LibraryListEntry> = idx.iter().map(|e| store::to_list_entry(e, &ov)).collect();
    Ok(store::apply_filter(&all, &filter))
}

/// Full instrument payload for the selected entry's detail card. A separate
/// command (rather than fields on `LibraryListEntry`) keeps `library_list`
/// lean — the list carries hundreds of entries, the card needs exactly one.
#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntryDetail {
    pub name: String,
    pub game: String,
    pub tags: Vec<String>,
    pub instrument: LibraryInstrument,
}

#[tauri::command]
#[specta::specta]
pub fn library_get_entry(
    lib: State<'_, LibraryState>,
    hash: String,
) -> Result<LibraryEntryDetail, String> {
    let idx = lib.index.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let ov = lib.overrides.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let e = idx.iter().find(|e| e.file.provenance.hash == hash)
        .ok_or("library entry not found")?;
    // Same override precedence as `store::to_list_entry`.
    let o = ov.get(&hash);
    Ok(LibraryEntryDetail {
        name: e.file.name.clone(),
        game: e.file.provenance.game.clone(),
        tags: o.and_then(|o| o.tags.clone()).unwrap_or_else(|| e.file.tags.clone()),
        instrument: e.file.instrument.clone(),
    })
}

#[tauri::command]
#[specta::specta]
pub fn library_games(lib: State<'_, LibraryState>) -> Result<Vec<String>, String> {
    let idx = lib.index.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let mut games: Vec<String> = idx.iter().map(|e| e.file.provenance.game.clone()).collect();
    games.sort();
    games.dedup();
    Ok(games)
}

#[tauri::command]
#[specta::specta]
pub fn library_rescan(app: tauri::AppHandle, lib: State<'_, LibraryState>) -> Result<u32, String> {
    state::rescan(&app, &lib);
    Ok(lib.index.lock().map_err(|e| format!("mutex poisoned: {e}"))?.len() as u32)
}

/// Scan/parse warnings from the last rescan (corrupt overrides/roots files,
/// unreadable entries). The UI surfaces these so quarantine events are visible.
#[tauri::command]
#[specta::specta]
pub fn library_warnings(lib: State<'_, LibraryState>) -> Result<Vec<String>, String> {
    Ok(lib.warnings.lock().map_err(|e| format!("mutex poisoned: {e}"))?.clone())
}

#[tauri::command]
#[specta::specta]
pub fn library_audition(
    audio_state: State<'_, AudioState>,
    lib: State<'_, LibraryState>,
    hash: String,
    midi_note: u8,
) -> Result<(), String> {
    let idx = lib.index.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let e = idx.iter().find(|e| e.file.provenance.hash == hash)
        .ok_or("library entry not found")?;
    match &e.file.instrument {
        LibraryInstrument::Fm(i) => do_preview_fm(&audio_state, i, midi_note),
        LibraryInstrument::Psg(i) => do_preview_psg(&audio_state, i, midi_note),
    }
}

#[tauri::command]
#[specta::specta]
pub fn library_add_to_project(
    project_state: State<'_, ProjectState>,
    lib: State<'_, LibraryState>,
    hash: String,
) -> Result<String, String> {
    let inst = {
        let idx = lib.index.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
        idx.iter().find(|e| e.file.provenance.hash == hash)
            .map(|e| e.file.instrument.clone())
            .ok_or("library entry not found")?
    };
    // Reuse the existing add paths (they assign fresh UUIDs + mark dirty) —
    // same manager acquisition as add_fm_instrument / add_psg_instrument.
    let mut mgr = project_state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    match inst {
        LibraryInstrument::Fm(i) => {
            let id = mgr.add_fm_instrument(i);
            Ok(id.to_string())
        }
        LibraryInstrument::Psg(i) => {
            let id = mgr.add_psg_instrument(i);
            Ok(id.to_string())
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn library_save_from_project(
    app: tauri::AppHandle,
    project_state: State<'_, ProjectState>,
    lib: State<'_, LibraryState>,
    kind: String,
    id: String,
    name: Option<String>,
    tags: Vec<String>,
) -> Result<String, String> {
    // Fetch the instrument from the ProjectManager by kind+id (same lookup
    // shape as update_fm_instrument: parse UUID, lock manager, find by id).
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let instrument = {
        let mgr = project_state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
        match kind.as_str() {
            "fm" => {
                let mut i = mgr
                    .get_fm_instrument(&uuid)
                    .ok_or("FM instrument not found")?
                    .clone();
                // Library files carry nil ids — determinism, and hash-dedup in
                // write_entry needs identical bytes.
                i.id = Uuid::nil();
                LibraryInstrument::Fm(i)
            }
            "psg" => {
                let mut i = mgr
                    .get_psg_instrument(&uuid)
                    .ok_or("PSG instrument not found")?
                    .clone();
                i.id = Uuid::nil();
                LibraryInstrument::Psg(i)
            }
            other => return Err(format!("unknown instrument kind: {other}")),
        }
    };
    let inst_name = match &instrument {
        LibraryInstrument::Fm(i) => i.name.clone(),
        LibraryInstrument::Psg(i) => i.name.clone(),
    };
    let hash = content_hash(&instrument);
    let file = LibraryEntryFile {
        schema: LIBRARY_SCHEMA,
        name: name.unwrap_or(inst_name),
        tags,
        provenance: Provenance { game: "User".into(), songs: vec![], slot: None, hash: hash.clone() },
        instrument,
    };
    store::write_entry(&state::user_root(&app)?, &file)?;
    state::rescan(&app, &lib);
    Ok(hash)
}

/// Drag-to-track swap: bind a library voice to a track, reusing a project
/// instrument with the same content hash or adding the voice first. Returns
/// the bound project instrument id. Kind-checked (FM voice ↔ FM track, PSG
/// voice ↔ PSG/noise track) in the manager.
#[tauri::command]
#[specta::specta]
pub fn library_assign_to_track(
    project_state: State<'_, ProjectState>,
    lib: State<'_, LibraryState>,
    track_id: String,
    hash: String,
) -> Result<String, String> {
    let track_uuid = Uuid::parse_str(&track_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let inst = {
        let idx = lib.index.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
        idx.iter().find(|e| e.file.provenance.hash == hash)
            .map(|e| e.file.instrument.clone())
            .ok_or("library entry not found")?
    };
    let mut mgr = project_state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let id = mgr.assign_library_instrument_to_track(track_uuid, &inst, &hash)?;
    Ok(id.to_string())
}

#[derive(serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryImportResult {
    pub written: u32,
    pub errors: Vec<String>,
}

#[tauri::command]
#[specta::specta]
pub fn library_import_files(
    app: tauri::AppHandle,
    lib: State<'_, LibraryState>,
    paths: Vec<String>,
) -> Result<LibraryImportResult, String> {
    let root = state::user_root(&app)?;
    let mut written = 0u32;
    // Per-file tolerance: one unreadable/unparseable file must not fail the
    // whole batch — collect its error and keep going.
    let mut errors = Vec::new();
    for p in &paths {
        let data = match std::fs::read(p) {
            Ok(d) => d,
            Err(e) => { errors.push(format!("could not read {p}: {e}")); continue; }
        };
        let fname = std::path::Path::new(p).file_name()
            .map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let res = match crate::import::fm_formats::import_fm_file(&data, &fname) {
            Ok(r) => r,
            Err(e) => { errors.push(format!("could not import {p}: {e}")); continue; }
        };
        let game = format!("Imported: {}", res.format);
        for mut inst in res.instruments {
            inst.id = Uuid::nil(); // library files carry nil ids (determinism)
            let name = inst.name.clone();
            let li = LibraryInstrument::Fm(inst);
            let hash = content_hash(&li);
            let file = LibraryEntryFile {
                schema: LIBRARY_SCHEMA, name, tags: vec![],
                provenance: Provenance { game: game.clone(), songs: vec![], slot: None, hash },
                instrument: li,
            };
            match store::write_entry(&root, &file) {
                Ok(_) => written += 1,
                Err(e) => errors.push(format!("could not write library entry for {p}: {e}")),
            }
        }
    }
    // Always rescan: partial batches still wrote entries.
    state::rescan(&app, &lib);
    Ok(LibraryImportResult { written, errors })
}

#[tauri::command]
#[specta::specta]
pub fn library_set_tags(
    app: tauri::AppHandle, lib: State<'_, LibraryState>,
    hash: String, tags: Vec<String>,
) -> Result<(), String> {
    let mut ov = lib.overrides.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    ov.entry(hash).or_default().tags = Some(tags);
    store::save_overrides(&state::overrides_path(&app)?, &ov)
}

#[tauri::command]
#[specta::specta]
pub fn library_set_favorite(
    app: tauri::AppHandle, lib: State<'_, LibraryState>,
    hash: String, favorite: bool,
) -> Result<(), String> {
    let mut ov = lib.overrides.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    ov.entry(hash).or_default().favorite = favorite;
    store::save_overrides(&state::overrides_path(&app)?, &ov)
}

#[tauri::command]
#[specta::specta]
pub fn library_roots_get(lib: State<'_, LibraryState>) -> Result<Vec<RootInfo>, String> {
    Ok(lib.roots.lock().map_err(|e| format!("mutex poisoned: {e}"))?.clone())
}

#[tauri::command]
#[specta::specta]
pub fn library_root_add(
    app: tauri::AppHandle, lib: State<'_, LibraryState>, path: String,
) -> Result<(), String> {
    if !std::path::Path::new(&path).is_dir() { return Err(format!("{path} is not a directory")); }
    {
        let mut roots = lib.roots.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
        if roots.iter().any(|r| r.path == path) { return Ok(()); }
        roots.push(RootInfo { label: path.clone(), path, kind: "custom".into() });
        state::save_custom_roots(&app, &roots)?;
    }
    state::rescan(&app, &lib);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn library_root_remove(
    app: tauri::AppHandle, lib: State<'_, LibraryState>, path: String,
) -> Result<(), String> {
    {
        let mut roots = lib.roots.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
        roots.retain(|r| !(r.kind == "custom" && r.path == path));
        state::save_custom_roots(&app, &roots)?;
    }
    state::rescan(&app, &lib);
    Ok(())
}
