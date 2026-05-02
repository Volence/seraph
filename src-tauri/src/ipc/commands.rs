use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;
use uuid::Uuid;

use crate::audio::frequency::{midi_to_fm_freq, midi_to_psg_period};
use crate::audio::{AudioCommand, AudioThread};
use crate::dac;
use crate::model::driver::{ChannelLayout, DriverFeature};
use crate::model::instrument::*;
use crate::model::song::{Song, SongMetadata};
use crate::project::ProjectManager;

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
pub fn open_project(state: State<'_, ProjectState>, path: String) -> Result<Song, String> {
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.open(&PathBuf::from(path))
}

#[tauri::command]
pub fn save_project(state: State<'_, ProjectState>) -> Result<(), String> {
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.save()
}

#[tauri::command]
pub fn close_project(state: State<'_, ProjectState>) -> Result<(), String> {
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.close();
    Ok(())
}

#[tauri::command]
pub fn get_project_info(state: State<'_, ProjectState>) -> Result<Option<SongMetadata>, String> {
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    Ok(mgr.metadata().cloned())
}

// --- Driver Info ---

#[derive(serde::Serialize)]
pub struct DriverInfo {
    pub id: String,
    pub name: String,
}

#[tauri::command]
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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriverDetail {
    pub id: String,
    pub name: String,
    pub layout: ChannelLayout,
    pub features: Vec<DriverFeature>,
}

#[tauri::command]
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
pub fn add_fm_instrument(
    state: State<'_, ProjectState>,
    instrument: FmInstrument,
) -> Result<String, String> {
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let id = mgr.add_fm_instrument(instrument);
    Ok(id.to_string())
}

#[tauri::command]
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
pub fn delete_fm_instrument(state: State<'_, ProjectState>, id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.delete_fm_instrument(uuid)
}

#[tauri::command]
pub fn list_fm_instruments(state: State<'_, ProjectState>) -> Result<Vec<FmInstrument>, String> {
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    Ok(mgr.list_fm_instruments().to_vec())
}

const OP_REG_OFFSETS: [u8; 4] = [0x00, 0x08, 0x04, 0x0C];

fn ym_write_port(thread: &mut AudioThread, port: u8, addr: u8, data: u8) {
    thread.send(AudioCommand::Ym2612Write { port: port as u32, data: addr });
    thread.send(AudioCommand::Ym2612Write { port: (port + 1) as u32, data });
}

#[tauri::command]
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

// --- PSG Instrument CRUD ---

#[tauri::command]
pub fn add_psg_instrument(
    state: State<'_, ProjectState>,
    instrument: PsgInstrument,
) -> Result<String, String> {
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let id = mgr.add_psg_instrument(instrument);
    Ok(id.to_string())
}

#[tauri::command]
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
pub fn delete_psg_instrument(state: State<'_, ProjectState>, id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.delete_psg_instrument(uuid)
}

#[tauri::command]
pub fn list_psg_instruments(state: State<'_, ProjectState>) -> Result<Vec<PsgInstrument>, String> {
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    Ok(mgr.list_psg_instruments().to_vec())
}

#[tauri::command]
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

    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;

    let period = midi_to_psg_period(midi_note);
    let channel: u8 = 0;

    thread.send(AudioCommand::PsgEnvelopePreview {
        channel,
        period,
        envelope: Arc::new(inst.volume_sequence),
        loop_point: inst.loop_point,
    });

    Ok(())
}

// --- DAC Instrument CRUD ---

#[tauri::command]
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
pub fn delete_dac_instrument(state: State<'_, ProjectState>, id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.delete_dac_instrument(uuid)
}

#[tauri::command]
pub fn list_dac_instruments(state: State<'_, ProjectState>) -> Result<Vec<DacInstrument>, String> {
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    Ok(mgr.list_dac_instruments().to_vec())
}

#[tauri::command]
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
