pub mod fm_formats;
pub mod psg_envelopes;
pub mod smps_mapper;
pub mod smps_parser;
pub mod vgm_import;
pub mod zyrinx_mapper;
pub mod zyrinx_parser;

use serde::Serialize;

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub project_dir: String,
    pub metadata: crate::model::song::SongMetadata,
    pub track_count: usize,
    pub instrument_count: usize,
    pub warnings: Vec<ImportWarning>,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportWarning {
    pub channel: String,
    pub message: String,
}

/// Content hash → library entry name. Built by the command layer from the
/// library index so `import/` stays decoupled from `LibraryState`.
pub type RecognitionTable = std::collections::HashMap<String, String>;

/// Import-time recognition: an imported instrument whose content hash matches
/// a library entry takes the entry's name; its original generic name is
/// preserved in the category ("Imported · Voice 3"). Misses stay untouched.
/// Content hashes cover only sound fields, so instruments are hashed via a
/// straight clone (id/name/metadata are excluded from the canonical bytes).
pub fn recognize_instruments(
    bank: &mut crate::model::instrument::InstrumentBank,
    table: &RecognitionTable,
) {
    use crate::library::entry::{content_hash, LibraryInstrument};
    if table.is_empty() {
        return;
    }
    for inst in &mut bank.fm {
        let hash = content_hash(&LibraryInstrument::Fm(inst.clone()));
        if let Some(lib_name) = table.get(&hash) {
            apply_recognition(&mut inst.name, &mut inst.metadata.category, lib_name);
        }
    }
    for inst in &mut bank.psg {
        let hash = content_hash(&LibraryInstrument::Psg(inst.clone()));
        if let Some(lib_name) = table.get(&hash) {
            apply_recognition(&mut inst.name, &mut inst.metadata.category, lib_name);
        }
    }
}

/// Category keeps its existing origin marker ("Imported", "Zyrinx Import",
/// "VGM Import"…) and appends the pre-recognition name.
fn apply_recognition(name: &mut String, category: &mut String, lib_name: &str) {
    let origin = if category.is_empty() { "Imported" } else { category.as_str() };
    *category = format!("{origin} · {name}");
    *name = lib_name.to_string();
}

pub fn import_zyrinx_rom(
    rom_path: &std::path::Path,
    parent_dir: &std::path::Path,
    game_id: u8,
    recognition: &RecognitionTable,
) -> Result<ImportResult, String> {
    let rom = std::fs::read(rom_path)
        .map_err(|e| format!("failed to read ROM {}: {e}", rom_path.display()))?;

    let zy = zyrinx_parser::parse_zyrinx_song(&rom, game_id)?;
    let mapped = zyrinx_mapper::map_zyrinx_to_song(&zy);
    let mut song = mapped.song;
    recognize_instruments(&mut song.instruments, recognition);

    let dir_name = zyrinx_parser::GAME_SONG_NAMES
        .get(game_id as usize)
        .unwrap_or(&"Zyrinx Import")
        .replace(' ', "_");
    let project_dir = parent_dir.join(&dir_name);

    std::fs::create_dir_all(&project_dir)
        .map_err(|e| format!("create dir {}: {e}", project_dir.display()))?;
    std::fs::create_dir_all(project_dir.join("instruments/fm")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(project_dir.join("instruments/psg")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(project_dir.join("instruments/dac")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(project_dir.join("exports")).map_err(|e| e.to_string())?;

    let version = serde_json::json!({ "version": "0.1.0" });
    std::fs::write(
        project_dir.join(".seraph"),
        serde_json::to_string_pretty(&version).unwrap(),
    ).map_err(|e| e.to_string())?;

    let project_file = crate::model::song::ProjectFile {
        metadata: song.metadata.clone(),
        tracks: song.tracks.clone(),
    };
    let json = serde_json::to_string_pretty(&project_file).map_err(|e| e.to_string())?;
    std::fs::write(project_dir.join("project.json"), json).map_err(|e| e.to_string())?;

    let mut instrument_count = 0;
    for inst in &song.instruments.fm {
        let json = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
        std::fs::write(
            project_dir.join(format!("instruments/fm/{}.json", inst.id)),
            json,
        ).map_err(|e| e.to_string())?;
        instrument_count += 1;
    }

    Ok(ImportResult {
        project_dir: project_dir.to_string_lossy().into_owned(),
        metadata: song.metadata,
        track_count: song.tracks.len(),
        instrument_count,
        warnings: mapped.warnings,
    })
}

pub fn import_smps_file(
    source_path: &std::path::Path,
    parent_dir: &std::path::Path,
    driver: &dyn crate::model::driver::DriverProfile,
) -> Result<ImportResult, String> {
    import_smps_file_with_dac(source_path, parent_dir, driver, None, &RecognitionTable::new())
}

pub fn import_smps_file_with_dac(
    source_path: &std::path::Path,
    parent_dir: &std::path::Path,
    driver: &dyn crate::model::driver::DriverProfile,
    dac_dir: Option<&std::path::Path>,
    recognition: &RecognitionTable,
) -> Result<ImportResult, String> {
    let source = std::fs::read_to_string(source_path)
        .map_err(|e| format!("failed to read {}: {e}", source_path.display()))?;

    let smps = smps_parser::parse_smps(&source)?;
    let mapped = smps_mapper::map_smps_to_song_with_dac(&smps, driver, dac_dir)?;
    let mut song = mapped.song;
    recognize_instruments(&mut song.instruments, recognition);

    let dir_name = source_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Import")
        .trim_start_matches("Mus - ");
    let project_dir = parent_dir.join(dir_name);

    std::fs::create_dir_all(&project_dir)
        .map_err(|e| format!("create dir {}: {e}", project_dir.display()))?;
    std::fs::create_dir_all(project_dir.join("instruments/fm")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(project_dir.join("instruments/psg")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(project_dir.join("instruments/dac")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(project_dir.join("exports")).map_err(|e| e.to_string())?;

    let version = serde_json::json!({ "version": "0.1.0" });
    std::fs::write(
        project_dir.join(".seraph"),
        serde_json::to_string_pretty(&version).unwrap(),
    ).map_err(|e| e.to_string())?;

    let project_file = crate::model::song::ProjectFile {
        metadata: song.metadata.clone(),
        tracks: song.tracks.clone(),
    };
    let json = serde_json::to_string_pretty(&project_file).map_err(|e| e.to_string())?;
    std::fs::write(project_dir.join("project.json"), json).map_err(|e| e.to_string())?;

    let mut instrument_count = 0;
    for inst in &song.instruments.fm {
        let json = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
        std::fs::write(
            project_dir.join(format!("instruments/fm/{}.json", inst.id)),
            json,
        ).map_err(|e| e.to_string())?;
        instrument_count += 1;
    }
    for inst in &song.instruments.psg {
        let json = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
        std::fs::write(
            project_dir.join(format!("instruments/psg/{}.json", inst.id)),
            json,
        ).map_err(|e| e.to_string())?;
        instrument_count += 1;
    }
    for inst in &song.instruments.dac {
        let json = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
        std::fs::write(
            project_dir.join(format!("instruments/dac/{}.json", inst.id)),
            json,
        ).map_err(|e| e.to_string())?;
        if let Some(pcm) = mapped.dac_pcm.get(&inst.id) {
            std::fs::write(
                project_dir.join(format!("instruments/dac/{}", inst.pcm_file)),
                pcm,
            ).map_err(|e| e.to_string())?;
        }
        instrument_count += 1;
    }

    Ok(ImportResult {
        project_dir: project_dir.to_string_lossy().into_owned(),
        metadata: song.metadata,
        track_count: song.tracks.len(),
        instrument_count,
        warnings: mapped.warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::flamedriver::FlamedriverProfile;
    use std::path::PathBuf;

    #[test]
    fn test_import_creates_project_directory() {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/Mus - DEZ1.asm");
        let tmp = tempfile::tempdir().unwrap();

        let driver = FlamedriverProfile;
        let result = import_smps_file(&source, tmp.path(), &driver).unwrap();
        let project_dir = PathBuf::from(&result.project_dir);

        assert!(project_dir.join("project.json").exists());
        assert!(project_dir.join(".seraph").exists());
        assert!(project_dir.join("instruments/fm").exists());
        assert!(result.track_count >= 9);
        assert!(result.instrument_count > 0);
        assert!(result.project_dir.ends_with("DEZ1"));
    }

    #[test]
    fn test_import_saves_fm_instruments() {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/Mus - DEZ1.asm");
        let tmp = tempfile::tempdir().unwrap();

        let driver = FlamedriverProfile;
        let result = import_smps_file(&source, tmp.path(), &driver).unwrap();
        let project_dir = PathBuf::from(&result.project_dir);

        let fm_dir = project_dir.join("instruments/fm");
        let fm_files: Vec<_> = std::fs::read_dir(&fm_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
            .collect();
        assert_eq!(fm_files.len(), 4);
    }

    #[test]
    fn test_import_dez1_round_trip_opens() {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/Mus - DEZ1.asm");
        let tmp = tempfile::tempdir().unwrap();

        let driver = FlamedriverProfile;
        let result = import_smps_file(&source, tmp.path(), &driver).unwrap();
        let project_dir = PathBuf::from(&result.project_dir);

        let json = std::fs::read_to_string(project_dir.join("project.json")).unwrap();
        let project_file: crate::model::song::ProjectFile =
            serde_json::from_str(&json).unwrap();

        assert_eq!(project_file.metadata.driver_id, "flamedriver");
        assert_eq!(project_file.tracks.len(), result.track_count);

        let fm_dir = project_dir.join("instruments/fm");
        for entry in std::fs::read_dir(&fm_dir).unwrap() {
            let entry = entry.unwrap();
            if entry.path().extension().map_or(false, |ext| ext == "json") {
                let data = std::fs::read_to_string(entry.path()).unwrap();
                let _inst: crate::model::instrument::FmInstrument =
                    serde_json::from_str(&data).unwrap();
            }
        }

        let fm_tracks: Vec<_> = project_file.tracks.iter()
            .filter(|t| matches!(t.channel, crate::model::song::ChannelAssignment::Fm(_)))
            .collect();
        assert!(!fm_tracks.is_empty());
        let fm1 = fm_tracks.iter().find(|t| t.name.starts_with("FM1")).unwrap();
        assert!(!fm1.regions.is_empty());
        assert!(!fm1.regions[0].notes.is_empty());
    }

    #[test]
    fn dump_dez1_import_diagnostics() {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/Mus - DEZ1.asm");
        let src_text = std::fs::read_to_string(&source).unwrap();
        let smps = smps_parser::parse_smps(&src_text).unwrap();

        eprintln!("--- SMPS Header ---");
        eprintln!("tempo_divider={} tempo_modifier={}", smps.tempo_divider, smps.tempo_modifier);

        for (i, ch) in smps.channels.iter().enumerate() {
            let note_count = ch.events.iter().filter(|e| matches!(e, smps_parser::SmpsEvent::Note { .. })).count();
            let rest_count = ch.events.iter().filter(|e| matches!(e, smps_parser::SmpsEvent::Rest { .. })).count();
            let total_dur: u64 = ch.events.iter().map(|e| match e {
                smps_parser::SmpsEvent::Note { duration, .. } => *duration as u64,
                smps_parser::SmpsEvent::Rest { duration } => *duration as u64,
                _ => 0,
            }).sum();
            let unsup: Vec<_> = ch.events.iter().filter_map(|e| match e {
                smps_parser::SmpsEvent::Unsupported { name } => Some(name.as_str()),
                _ => None,
            }).collect();
            eprintln!("Ch{i} {:?} '{}': {} notes, {} rests, total_dur={}, unsup={:?}",
                ch.kind, ch.label, note_count, rest_count, total_dur, unsup);
        }

        let driver = FlamedriverProfile;
        let mapped = smps_mapper::map_smps_to_song(&smps, &driver).unwrap();

        eprintln!("\n--- Warnings ---");
        for w in &mapped.warnings { eprintln!("  [{}] {}", w.channel, w.message); }

        eprintln!("\n--- Tracks (BPM={:.1}) ---", mapped.song.metadata.tempo);
        for t in &mapped.song.tracks {
            let n: usize = t.regions.iter().map(|r| r.notes.len()).sum();
            let end: u64 = t.regions.iter().flat_map(|r| r.notes.iter()).map(|n| n.tick + n.duration_ticks).max().unwrap_or(0);
            eprintln!("  {} ({:?}): {} notes, ends@tick {}", t.name, t.channel, n, end);
        }
    }

    #[test]
    fn test_import_aiz1_uvb_song() {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/AIZ1.asm");
        let tmp = tempfile::tempdir().unwrap();

        let driver = FlamedriverProfile;
        let result = import_smps_file(&source, tmp.path(), &driver).unwrap();
        let project_dir = PathBuf::from(&result.project_dir);

        assert!(project_dir.join("project.json").exists());
        assert!(result.instrument_count > 0);
    }

    #[test]
    fn dump_aiz1_alignment_diagnostics() {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/AIZ1.asm");
        let src_text = std::fs::read_to_string(&source).unwrap();
        let smps = smps_parser::parse_smps(&src_text).unwrap();

        eprintln!("\n=== AIZ1 SMPS RAW CHANNEL DATA ===");
        for (i, ch) in smps.channels.iter().enumerate() {
            let total_smps_dur: u64 = ch.events.iter().map(|e| match e {
                smps_parser::SmpsEvent::Note { duration, .. } => *duration as u64,
                smps_parser::SmpsEvent::Rest { duration } => *duration as u64,
                _ => 0,
            }).sum();
            let first_5: Vec<String> = ch.events.iter().take(10).map(|e| format!("{:?}", e)).collect();
            eprintln!("Ch{i} {:?} '{}' pitch={} vol={}: total_smps_ticks={}\n  first events: {:?}",
                ch.kind, ch.label, ch.initial_pitch, ch.initial_volume, total_smps_dur, first_5);
        }

        let driver = FlamedriverProfile;
        let mapped = smps_mapper::map_smps_to_song(&smps, &driver).unwrap();

        eprintln!("\n=== MAPPED TRACKS (BPM={:.1}) ===", mapped.song.metadata.tempo);
        let note_names = ["C","C#","D","D#","E","F","F#","G","G#","A","A#","B"];
        for t in &mapped.song.tracks {
            let all_notes: Vec<_> = t.regions.iter().flat_map(|r| {
                r.notes.iter().map(move |n| (r.start_tick + n.tick, n.pitch, n.duration_ticks, n.detune))
            }).collect();
            let first_8: Vec<String> = all_notes.iter().take(8).map(|(tick, p, dur, det)| {
                let name = note_names[(*p % 12) as usize];
                let oct = (*p / 12) as i8 - 1;
                format!("({tick}: {name}{oct} [midi={p}] dur={dur} det={det})")
            }).collect();
            let last_tick = all_notes.iter().map(|(t, _, d, _)| t + d).max().unwrap_or(0);
            eprintln!("  {} ({:?}) pitch_off={}: {} regions, {} notes, last_tick={}\n    first 8: {:?}",
                t.name, t.channel, t.pitch_offset, t.regions.len(), all_notes.len(), last_tick, first_8);
        }

        eprintln!("\n=== WARNINGS ===");
        for w in &mapped.warnings { eprintln!("  [{}] {}", w.channel, w.message); }
    }

    #[test]
    fn test_aiz1_roundtrip_export() {
        use crate::export::smps::{
            build_voice_index, compute_tempo_params, generate_music_asm,
            generate_voice_bank_asm, sanitize_label,
        };
        use crate::model::song::ChannelAssignment;

        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/AIZ1.asm");
        let tmp = tempfile::tempdir().unwrap();
        let driver = FlamedriverProfile;

        let result = import_smps_file(&source, tmp.path(), &driver).unwrap();
        let project_dir = PathBuf::from(&result.project_dir);
        let json = std::fs::read_to_string(project_dir.join("project.json")).unwrap();
        let pf: crate::model::song::ProjectFile = serde_json::from_str(&json).unwrap();

        let mut instruments = crate::model::instrument::InstrumentBank {
            fm: vec![], psg: vec![], dac: vec![],
        };
        for entry in std::fs::read_dir(project_dir.join("instruments/fm")).unwrap() {
            let entry = entry.unwrap();
            if entry.path().extension().map_or(false, |ext| ext == "json") {
                let data = std::fs::read_to_string(entry.path()).unwrap();
                let inst: crate::model::instrument::FmInstrument = serde_json::from_str(&data).unwrap();
                instruments.fm.push(inst);
            }
        }
        for entry in std::fs::read_dir(project_dir.join("instruments/psg")).unwrap() {
            let entry = entry.unwrap();
            if entry.path().extension().map_or(false, |ext| ext == "json") {
                let data = std::fs::read_to_string(entry.path()).unwrap();
                let inst: crate::model::instrument::PsgInstrument = serde_json::from_str(&data).unwrap();
                instruments.psg.push(inst);
            }
        }
        for entry in std::fs::read_dir(project_dir.join("instruments/dac")).unwrap() {
            let entry = entry.unwrap();
            if entry.path().extension().map_or(false, |ext| ext == "json") {
                let data = std::fs::read_to_string(entry.path()).unwrap();
                let inst: crate::model::instrument::DacInstrument = serde_json::from_str(&data).unwrap();
                instruments.dac.push(inst);
            }
        }

        let song = crate::model::song::Song {
            metadata: pf.metadata,
            tracks: pf.tracks,
            instruments,
        };

        let params = compute_tempo_params(song.metadata.tempo, song.metadata.ticks_per_beat);
        let (voice_map, voices) = build_voice_index(&song.tracks, &song.instruments);
        let music_asm = generate_music_asm(&song, &song.instruments, &voice_map, &params).unwrap();
        let voice_asm = generate_voice_bank_asm(&sanitize_label(&song.metadata.name), &voices, &driver);

        let dump_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_output");
        std::fs::create_dir_all(&dump_dir).unwrap();
        std::fs::write(dump_dir.join("roundtrip_music.asm"), &music_asm).unwrap();
        std::fs::write(dump_dir.join("roundtrip_voices.asm"), &voice_asm).unwrap();
        eprintln!("Wrote roundtrip ASM to {:?}", dump_dir);

        assert!(music_asm.contains("smpsHeaderStartSong 3"));
        assert!(music_asm.contains("smpsHeaderTempo\t\t$01, $1F"));

        let fm_tracks: Vec<_> = song.tracks.iter()
            .filter(|t| matches!(t.channel, ChannelAssignment::Fm(_)))
            .collect();
        assert_eq!(fm_tracks.len(), 5);

        assert!(music_asm.contains("smpsSetvoice"), "export must have mid-channel voice changes");

        let tl_lines: Vec<&str> = voice_asm.lines()
            .filter(|l| l.contains("smpsVcTotalLevel"))
            .collect();
        for line in &tl_lines {
            let vals: Vec<u8> = line.split('\t').last().unwrap_or("")
                .split(',')
                .filter_map(|s| {
                    let s = s.trim().trim_start_matches('$');
                    u8::from_str_radix(s, 16).ok()
                })
                .collect();
            for v in &vals {
                assert!(*v <= 0x7F, "TL value must not have carrier flag ($80): {line}");
            }
        }

        assert!(music_asm.contains("smpsPan"), "FM tracks need pan");
        assert!(music_asm.contains("smpsStop"), "tracks need stop");

        eprintln!("\n=== ROUNDTRIP SUMMARY ===");
        eprintln!("Tracks: {}", song.tracks.len());
        eprintln!("FM voices: {}", voices.len());
        eprintln!("Music ASM lines: {}", music_asm.lines().count());
        eprintln!("Voice ASM lines: {}", voice_asm.lines().count());

        let orig_source = std::fs::read_to_string(&source).unwrap();
        let orig_fm1_notes: Vec<&str> = orig_source.lines()
            .skip_while(|l| !l.contains("Snd_AIZ1_FM1:"))
            .skip(1)
            .take_while(|l| !l.contains("Snd_AIZ1_FM2:") && !l.contains("Snd_AIZ1_"))
            .filter(|l| l.trim().starts_with("dc.b"))
            .collect();
        let export_fm1_lines: Vec<&str> = music_asm.lines()
            .skip_while(|l| !l.contains("_FM1:"))
            .skip(1)
            .take_while(|l| !l.contains("_FM") && !l.contains("_PSG") && !l.contains("_DAC") && !l.trim().starts_with("; ---"))
            .filter(|l| l.trim().starts_with("dc.b") || l.contains("nC") || l.contains("nD") || l.contains("nE") || l.contains("nF") || l.contains("nG") || l.contains("nA") || l.contains("nB") || l.contains("nRst"))
            .collect();
        eprintln!("\nOriginal FM1 dc.b lines: {}", orig_fm1_notes.len());
        eprintln!("Export FM1 note lines: {}", export_fm1_lines.len());
    }

    #[test]
    fn test_aiz1_roundtrip_deep_comparison() {
        use crate::export::smps::{
            build_voice_index, compute_tempo_params, generate_music_asm,
            generate_voice_bank_asm, sanitize_label,
        };
        use crate::import::smps_parser::{
            SmpsChannelKind, SmpsEvent as ParsedEvent,
        };

        // ----------------------------------------------------------------
        // Helper: compute Flamedriver Z80 frequency (block, fnum) from an
        // SMPS note byte + signed transpose, exactly as the Z80 driver does.
        // ----------------------------------------------------------------
        fn z80_freq(smps_byte: u8, transpose: i16) -> (u8, u16) {
            let fm_fnum: [u16; 12] = [
                0x284, 0x2AB, 0x2D3, 0x2FE, 0x32D, 0x35C,
                0x38F, 0x3C5, 0x3FF, 0x43C, 0x47C, 0x4C0,
            ];
            let z80_index =
                ((smps_byte as i16 - 0x81) + transpose).rem_euclid(256) as u8;
            let semitone = (z80_index % 12) as usize;
            let block = (z80_index / 12) % 8;
            (block, fm_fnum[semitone])
        }

        // ----------------------------------------------------------------
        // 1. Parse original AIZ1.asm
        // ----------------------------------------------------------------
        let orig_source = std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/AIZ1.asm"),
        )
        .unwrap();
        let orig = smps_parser::parse_smps(&orig_source).unwrap();

        // ----------------------------------------------------------------
        // 2. Import -> Song -> Export -> re-parse
        // ----------------------------------------------------------------
        let source_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_data/AIZ1.asm");
        let tmp = tempfile::tempdir().unwrap();
        let driver = FlamedriverProfile;

        let result = import_smps_file(&source_path, tmp.path(), &driver).unwrap();
        let project_dir = PathBuf::from(&result.project_dir);

        // Reload song from disk (same as existing test)
        let pf: crate::model::song::ProjectFile = serde_json::from_str(
            &std::fs::read_to_string(project_dir.join("project.json")).unwrap(),
        )
        .unwrap();
        let mut instruments = crate::model::instrument::InstrumentBank {
            fm: vec![],
            psg: vec![],
            dac: vec![],
        };
        for entry in std::fs::read_dir(project_dir.join("instruments/fm")).unwrap() {
            let entry = entry.unwrap();
            if entry.path().extension().map_or(false, |ext| ext == "json") {
                let data = std::fs::read_to_string(entry.path()).unwrap();
                let inst: crate::model::instrument::FmInstrument =
                    serde_json::from_str(&data).unwrap();
                instruments.fm.push(inst);
            }
        }
        for entry in std::fs::read_dir(project_dir.join("instruments/psg")).unwrap() {
            let entry = entry.unwrap();
            if entry.path().extension().map_or(false, |ext| ext == "json") {
                let data = std::fs::read_to_string(entry.path()).unwrap();
                let inst: crate::model::instrument::PsgInstrument =
                    serde_json::from_str(&data).unwrap();
                instruments.psg.push(inst);
            }
        }
        for entry in std::fs::read_dir(project_dir.join("instruments/dac")).unwrap() {
            let entry = entry.unwrap();
            if entry.path().extension().map_or(false, |ext| ext == "json") {
                let data = std::fs::read_to_string(entry.path()).unwrap();
                let inst: crate::model::instrument::DacInstrument =
                    serde_json::from_str(&data).unwrap();
                instruments.dac.push(inst);
            }
        }

        let song = crate::model::song::Song {
            metadata: pf.metadata,
            tracks: pf.tracks,
            instruments,
        };

        let params =
            compute_tempo_params(song.metadata.tempo, song.metadata.ticks_per_beat);
        let (voice_map, voices) = build_voice_index(&song.tracks, &song.instruments);
        let music_asm =
            generate_music_asm(&song, &song.instruments, &voice_map, &params).unwrap();
        let voice_asm = generate_voice_bank_asm(
            &sanitize_label(&song.metadata.name),
            &voices,
            &driver,
        );

        // Re-parse the combined exported ASM as a single SMPS file.
        // The music_asm references the voice label, and voice_asm defines it.
        //
        // NOTE: The export has a known bug where PSG noise header lines
        // reference "_PSGn" but channel data is labeled "_PSG_Noisen".
        // We patch this before re-parsing so the rest of the comparison
        // can proceed. This mismatch is reported as a divergence.
        let combined_raw = format!("{}\n{}", music_asm, voice_asm);
        let label = sanitize_label(&song.metadata.name);
        let combined_asm = combined_raw.replace(
            &format!("Snd_{label}_PSG4,"),
            &format!("Snd_{label}_PSG_Noise4,"),
        );
        let has_psg_noise_label_bug = combined_raw != combined_asm;
        let reparsed = smps_parser::parse_smps(&combined_asm).unwrap();

        // ----------------------------------------------------------------
        // Diagnostic accumulators
        // ----------------------------------------------------------------
        let mut total_divergences: usize = 0;
        let mut channels_matched: usize = 0;
        let mut channels_diverged: usize = 0;
        let mut divergence_details: Vec<String> = Vec::new();

        // ----------------------------------------------------------------
        // 3. Header values comparison
        // ----------------------------------------------------------------
        eprintln!("\n========================================");
        eprintln!("  AIZ1 ROUNDTRIP DEEP COMPARISON");
        eprintln!("========================================\n");

        eprintln!("--- Header ---");
        eprintln!(
            "  tempo_divider : orig=${:02X}  rt=${:02X}",
            orig.tempo_divider, reparsed.tempo_divider
        );
        eprintln!(
            "  tempo_modifier: orig=${:02X}  rt=${:02X}",
            orig.tempo_modifier, reparsed.tempo_modifier
        );
        if orig.tempo_divider != reparsed.tempo_divider {
            divergence_details.push(format!(
                "HEADER: tempo_divider orig=${:02X} vs rt=${:02X}",
                orig.tempo_divider, reparsed.tempo_divider
            ));
            total_divergences += 1;
        }
        if orig.tempo_modifier != reparsed.tempo_modifier {
            divergence_details.push(format!(
                "HEADER: tempo_modifier orig=${:02X} vs rt=${:02X}",
                orig.tempo_modifier, reparsed.tempo_modifier
            ));
            total_divergences += 1;
        }

        if has_psg_noise_label_bug {
            eprintln!("  PSG NOISE LABEL BUG: export header says _PSG4 but data says _PSG_Noise4 (patched for re-parse)");
            divergence_details.push(
                "EXPORT BUG: PSG noise header label _PSG4 vs data label _PSG_Noise4".to_string(),
            );
            total_divergences += 1;
        }

        // ----------------------------------------------------------------
        // 4. Channel-by-channel comparison
        // ----------------------------------------------------------------
        let orig_fm: Vec<_> = orig
            .channels
            .iter()
            .filter(|c| c.kind == SmpsChannelKind::Fm)
            .collect();
        let orig_psg: Vec<_> = orig
            .channels
            .iter()
            .filter(|c| c.kind == SmpsChannelKind::Psg)
            .collect();
        let orig_dac: Vec<_> = orig
            .channels
            .iter()
            .filter(|c| c.kind == SmpsChannelKind::Dac)
            .collect();
        let rt_fm: Vec<_> = reparsed
            .channels
            .iter()
            .filter(|c| c.kind == SmpsChannelKind::Fm)
            .collect();
        let rt_psg: Vec<_> = reparsed
            .channels
            .iter()
            .filter(|c| c.kind == SmpsChannelKind::Psg)
            .collect();
        let rt_dac: Vec<_> = reparsed
            .channels
            .iter()
            .filter(|c| c.kind == SmpsChannelKind::Dac)
            .collect();

        eprintln!(
            "\n--- Channel counts: orig FM={} PSG={} DAC={}  rt FM={} PSG={} DAC={} ---",
            orig_fm.len(),
            orig_psg.len(),
            orig_dac.len(),
            rt_fm.len(),
            rt_psg.len(),
            rt_dac.len()
        );

        // ----------------------------------------------------------------
        // 4a. Compare FM channels
        // ----------------------------------------------------------------
        let fm_pairs = orig_fm.len().min(rt_fm.len());
        for i in 0..fm_pairs {
            let o = orig_fm[i];
            let r = rt_fm[i];
            let label = format!("FM{}", i + 1);
            let mut ch_divergences: Vec<String> = Vec::new();

            // -- Header: transpose, volume --
            eprintln!("\n--- {} ---", label);
            eprintln!(
                "  header pitch : orig=${:02X} ({:+})  rt=${:02X} ({:+})",
                o.initial_pitch as u8,
                o.initial_pitch,
                r.initial_pitch as u8,
                r.initial_pitch
            );
            eprintln!(
                "  header volume: orig=${:02X}  rt=${:02X}",
                o.initial_volume, r.initial_volume
            );

            if o.initial_volume != r.initial_volume {
                ch_divergences.push(format!(
                    "header volume: orig=${:02X} vs rt=${:02X}",
                    o.initial_volume, r.initial_volume
                ));
            }

            // -- Total SMPS tick duration --
            let orig_dur: u64 = o
                .events
                .iter()
                .map(|e| match e {
                    ParsedEvent::Note { duration, .. } => *duration as u64,
                    ParsedEvent::Rest { duration } => *duration as u64,
                    _ => 0,
                })
                .sum();
            let rt_dur: u64 = r
                .events
                .iter()
                .map(|e| match e {
                    ParsedEvent::Note { duration, .. } => *duration as u64,
                    ParsedEvent::Rest { duration } => *duration as u64,
                    _ => 0,
                })
                .sum();
            eprintln!(
                "  total SMPS ticks: orig={}  rt={}  delta={}",
                orig_dur,
                rt_dur,
                rt_dur as i64 - orig_dur as i64
            );
            if orig_dur != rt_dur {
                ch_divergences.push(format!(
                    "TICK DURATION: orig={} vs rt={} (delta={})",
                    orig_dur,
                    rt_dur,
                    rt_dur as i64 - orig_dur as i64
                ));
            }

            // -- Note-by-note Z80 frequency comparison --
            // Build orig notes with running transpose
            let orig_notes_full: Vec<(u8, u8, i16)> = {
                let mut transpose_acc: i16 = o.initial_pitch as i16;
                let mut notes = Vec::new();
                for ev in &o.events {
                    match ev {
                        ParsedEvent::Note { pitch, duration } => {
                            notes.push((*pitch, *duration, transpose_acc));
                        }
                        ParsedEvent::Transpose(delta) => {
                            transpose_acc += *delta as i16;
                        }
                        _ => {}
                    }
                }
                notes
            };

            // Reparsed notes (transpose is always 0 for exported FM)
            let rt_notes: Vec<(u8, u8)> = r
                .events
                .iter()
                .filter_map(|e| match e {
                    ParsedEvent::Note { pitch, duration } => Some((*pitch, *duration)),
                    _ => None,
                })
                .collect();

            eprintln!(
                "  note count: orig={}  rt={}",
                orig_notes_full.len(),
                rt_notes.len()
            );

            let note_count = orig_notes_full.len().min(rt_notes.len());
            let mut freq_mismatches = 0usize;
            let mut dur_mismatches = 0usize;

            for j in 0..note_count {
                let (o_byte, o_dur, o_trans) = orig_notes_full[j];
                let (r_byte, r_dur) = rt_notes[j];
                let r_trans: i16 = r.initial_pitch as i16;

                let (o_block, o_fnum) = z80_freq(o_byte, o_trans);
                let (r_block, r_fnum) = z80_freq(r_byte, r_trans);

                if o_block != r_block || o_fnum != r_fnum {
                    freq_mismatches += 1;
                    if freq_mismatches <= 5 {
                        ch_divergences.push(format!(
                            "note[{}] FREQ: orig byte=${:02X} trans={:+} -> blk={} fn=${:03X}  |  rt byte=${:02X} trans={:+} -> blk={} fn=${:03X}",
                            j, o_byte, o_trans, o_block, o_fnum, r_byte, r_trans, r_block, r_fnum
                        ));
                    }
                }

                if o_dur != r_dur {
                    dur_mismatches += 1;
                    if dur_mismatches <= 5 {
                        ch_divergences.push(format!(
                            "note[{}] DURATION: orig=${:02X} ({})  rt=${:02X} ({})",
                            j, o_dur, o_dur, r_dur, r_dur
                        ));
                    }
                }
            }

            if orig_notes_full.len() != rt_notes.len() {
                ch_divergences.push(format!(
                    "NOTE COUNT: orig={} vs rt={}",
                    orig_notes_full.len(),
                    rt_notes.len()
                ));
            }

            if freq_mismatches > 5 {
                ch_divergences.push(format!(
                    "... and {} more frequency mismatches",
                    freq_mismatches - 5
                ));
            }
            if dur_mismatches > 5 {
                ch_divergences.push(format!(
                    "... and {} more duration mismatches",
                    dur_mismatches - 5
                ));
            }

            eprintln!(
                "  freq mismatches: {}  dur mismatches: {}",
                freq_mismatches, dur_mismatches
            );

            // -- Coordination flags --
            let orig_voices: Vec<u8> = o
                .events
                .iter()
                .filter_map(|e| match e {
                    ParsedEvent::SetVoice(v) => Some(*v),
                    _ => None,
                })
                .collect();
            let rt_voices: Vec<u8> = r
                .events
                .iter()
                .filter_map(|e| match e {
                    ParsedEvent::SetVoice(v) => Some(*v),
                    _ => None,
                })
                .collect();
            eprintln!(
                "  voice changes: orig={} rt={}",
                orig_voices.len(),
                rt_voices.len()
            );
            if orig_voices.len() != rt_voices.len() {
                ch_divergences.push(format!(
                    "VOICE CHANGE COUNT: orig={} vs rt={}",
                    orig_voices.len(),
                    rt_voices.len()
                ));
            }

            let orig_pans: Vec<u8> = o
                .events
                .iter()
                .filter_map(|e| match e {
                    ParsedEvent::SetPan(p) => Some(*p),
                    _ => None,
                })
                .collect();
            let rt_pans: Vec<u8> = r
                .events
                .iter()
                .filter_map(|e| match e {
                    ParsedEvent::SetPan(p) => Some(*p),
                    _ => None,
                })
                .collect();
            eprintln!(
                "  pan events: orig={:?}  rt={:?}",
                orig_pans, rt_pans
            );
            // Compare first pan value if both have at least one
            if !orig_pans.is_empty() && !rt_pans.is_empty() {
                if orig_pans[0] != rt_pans[0] {
                    ch_divergences.push(format!(
                        "PAN: orig=${:02X} vs rt=${:02X}",
                        orig_pans[0], rt_pans[0]
                    ));
                }
            }

            let orig_mods: Vec<_> = o
                .events
                .iter()
                .filter(|e| matches!(e, ParsedEvent::SetModulation { .. }))
                .collect();
            let rt_mods: Vec<_> = r
                .events
                .iter()
                .filter(|e| matches!(e, ParsedEvent::SetModulation { .. }))
                .collect();
            if !orig_mods.is_empty() || !rt_mods.is_empty() {
                eprintln!(
                    "  modulation events: orig={}  rt={}",
                    orig_mods.len(),
                    rt_mods.len()
                );
                if orig_mods.len() != rt_mods.len() {
                    ch_divergences.push(format!(
                        "MODULATION COUNT: orig={} vs rt={} (export strips modulation)",
                        orig_mods.len(),
                        rt_mods.len()
                    ));
                }
            }

            let orig_vol_changes: Vec<i8> = o
                .events
                .iter()
                .filter_map(|e| match e {
                    ParsedEvent::VolumeChange(v) => Some(*v),
                    _ => None,
                })
                .collect();
            let rt_vol_changes: Vec<i8> = r
                .events
                .iter()
                .filter_map(|e| match e {
                    ParsedEvent::VolumeChange(v) => Some(*v),
                    _ => None,
                })
                .collect();
            if !orig_vol_changes.is_empty() || !rt_vol_changes.is_empty() {
                eprintln!(
                    "  volume changes: orig={}  rt={}",
                    orig_vol_changes.len(),
                    rt_vol_changes.len()
                );
                if orig_vol_changes != rt_vol_changes {
                    ch_divergences.push(format!(
                        "VOLUME CHANGES: orig={:?} vs rt={:?}",
                        &orig_vol_changes[..orig_vol_changes.len().min(5)],
                        &rt_vol_changes[..rt_vol_changes.len().min(5)]
                    ));
                }
            }

            let orig_transposes: Vec<i8> = o
                .events
                .iter()
                .filter_map(|e| match e {
                    ParsedEvent::Transpose(v) => Some(*v),
                    _ => None,
                })
                .collect();
            let rt_transposes: Vec<i8> = r
                .events
                .iter()
                .filter_map(|e| match e {
                    ParsedEvent::Transpose(v) => Some(*v),
                    _ => None,
                })
                .collect();
            if !orig_transposes.is_empty() || !rt_transposes.is_empty() {
                eprintln!(
                    "  transpose events: orig={}  rt={}",
                    orig_transposes.len(),
                    rt_transposes.len()
                );
                if orig_transposes != rt_transposes {
                    ch_divergences.push(format!(
                        "TRANSPOSE: orig count={} vs rt count={} (export bakes into notes)",
                        orig_transposes.len(),
                        rt_transposes.len()
                    ));
                }
            }

            let orig_detunes: Vec<i8> = o
                .events
                .iter()
                .filter_map(|e| match e {
                    ParsedEvent::Detune(v) => Some(*v),
                    _ => None,
                })
                .collect();
            let rt_detunes: Vec<i8> = r
                .events
                .iter()
                .filter_map(|e| match e {
                    ParsedEvent::Detune(v) => Some(*v),
                    _ => None,
                })
                .collect();
            if !orig_detunes.is_empty() || !rt_detunes.is_empty() {
                eprintln!(
                    "  detune events: orig={}  rt={}",
                    orig_detunes.len(),
                    rt_detunes.len()
                );
                if orig_detunes != rt_detunes {
                    ch_divergences.push(format!(
                        "DETUNE: orig count={} vs rt count={} (export strips detune)",
                        orig_detunes.len(),
                        rt_detunes.len()
                    ));
                }
            }

            // Tally channel result
            if ch_divergences.is_empty() {
                channels_matched += 1;
                eprintln!("  RESULT: MATCHED");
            } else {
                channels_diverged += 1;
                eprintln!("  RESULT: {} divergences", ch_divergences.len());
                for d in &ch_divergences {
                    eprintln!("    {}", d);
                    divergence_details.push(format!("[{}] {}", label, d));
                }
                total_divergences += ch_divergences.len();
            }
        }

        // ----------------------------------------------------------------
        // 4b. Compare PSG channels
        // ----------------------------------------------------------------
        let psg_pairs = orig_psg.len().min(rt_psg.len());
        for i in 0..psg_pairs {
            let o = orig_psg[i];
            let r = rt_psg[i];
            let label = format!("PSG{}", i + 1);
            let mut ch_divergences: Vec<String> = Vec::new();

            eprintln!("\n--- {} ---", label);

            let orig_dur: u64 = o
                .events
                .iter()
                .map(|e| match e {
                    ParsedEvent::Note { duration, .. } => *duration as u64,
                    ParsedEvent::Rest { duration } => *duration as u64,
                    _ => 0,
                })
                .sum();
            let rt_dur: u64 = r
                .events
                .iter()
                .map(|e| match e {
                    ParsedEvent::Note { duration, .. } => *duration as u64,
                    ParsedEvent::Rest { duration } => *duration as u64,
                    _ => 0,
                })
                .sum();
            eprintln!(
                "  total SMPS ticks: orig={}  rt={}  delta={}",
                orig_dur,
                rt_dur,
                rt_dur as i64 - orig_dur as i64
            );
            if orig_dur != rt_dur {
                ch_divergences.push(format!(
                    "TICK DURATION: orig={} vs rt={} (delta={})",
                    orig_dur,
                    rt_dur,
                    rt_dur as i64 - orig_dur as i64
                ));
            }

            let orig_note_count = o
                .events
                .iter()
                .filter(|e| matches!(e, ParsedEvent::Note { .. }))
                .count();
            let rt_note_count = r
                .events
                .iter()
                .filter(|e| matches!(e, ParsedEvent::Note { .. }))
                .count();
            eprintln!(
                "  note count: orig={}  rt={}",
                orig_note_count, rt_note_count
            );

            eprintln!(
                "  header pitch : orig=${:02X} ({:+})  rt=${:02X} ({:+})",
                o.initial_pitch as u8,
                o.initial_pitch,
                r.initial_pitch as u8,
                r.initial_pitch
            );
            eprintln!(
                "  header volume: orig=${:02X}  rt=${:02X}",
                o.initial_volume, r.initial_volume
            );
            if o.initial_volume != r.initial_volume {
                ch_divergences.push(format!(
                    "header volume: orig=${:02X} vs rt=${:02X}",
                    o.initial_volume, r.initial_volume
                ));
            }

            if ch_divergences.is_empty() {
                channels_matched += 1;
                eprintln!("  RESULT: MATCHED");
            } else {
                channels_diverged += 1;
                eprintln!("  RESULT: {} divergences", ch_divergences.len());
                for d in &ch_divergences {
                    eprintln!("    {}", d);
                    divergence_details.push(format!("[{}] {}", label, d));
                }
                total_divergences += ch_divergences.len();
            }
        }

        // ----------------------------------------------------------------
        // 4c. Compare DAC channels
        // ----------------------------------------------------------------
        let dac_pairs = orig_dac.len().min(rt_dac.len());
        for i in 0..dac_pairs {
            let o = orig_dac[i];
            let r = rt_dac[i];
            let label = "DAC".to_string();
            let mut ch_divergences: Vec<String> = Vec::new();

            eprintln!("\n--- {} ---", label);

            let orig_dur: u64 = o
                .events
                .iter()
                .map(|e| match e {
                    ParsedEvent::Note { duration, .. } => *duration as u64,
                    ParsedEvent::Rest { duration } => *duration as u64,
                    _ => 0,
                })
                .sum();
            let rt_dur: u64 = r
                .events
                .iter()
                .map(|e| match e {
                    ParsedEvent::Note { duration, .. } => *duration as u64,
                    ParsedEvent::Rest { duration } => *duration as u64,
                    _ => 0,
                })
                .sum();
            eprintln!(
                "  total SMPS ticks: orig={}  rt={}  delta={}",
                orig_dur,
                rt_dur,
                rt_dur as i64 - orig_dur as i64
            );
            if orig_dur != rt_dur {
                ch_divergences.push(format!(
                    "TICK DURATION: orig={} vs rt={} (delta={})",
                    orig_dur,
                    rt_dur,
                    rt_dur as i64 - orig_dur as i64
                ));
            }

            let orig_note_count = o
                .events
                .iter()
                .filter(|e| matches!(e, ParsedEvent::Note { .. }))
                .count();
            let rt_note_count = r
                .events
                .iter()
                .filter(|e| matches!(e, ParsedEvent::Note { .. }))
                .count();
            eprintln!(
                "  note count: orig={}  rt={}",
                orig_note_count, rt_note_count
            );
            if orig_note_count != rt_note_count {
                ch_divergences.push(format!(
                    "DAC NOTE COUNT: orig={} vs rt={}",
                    orig_note_count, rt_note_count
                ));
            }

            // Compare DAC note-by-note: sample byte + duration
            let orig_dac_notes: Vec<(u8, u8)> = o
                .events
                .iter()
                .filter_map(|e| match e {
                    ParsedEvent::Note { pitch, duration } => Some((*pitch, *duration)),
                    _ => None,
                })
                .collect();
            let rt_dac_notes: Vec<(u8, u8)> = r
                .events
                .iter()
                .filter_map(|e| match e {
                    ParsedEvent::Note { pitch, duration } => Some((*pitch, *duration)),
                    _ => None,
                })
                .collect();
            let dac_note_pairs = orig_dac_notes.len().min(rt_dac_notes.len());
            let mut dac_sample_mismatches = 0usize;
            let mut dac_dur_mismatches = 0usize;
            for j in 0..dac_note_pairs {
                let (o_sample, o_dur) = orig_dac_notes[j];
                let (r_sample, r_dur) = rt_dac_notes[j];
                if o_sample != r_sample {
                    dac_sample_mismatches += 1;
                    if dac_sample_mismatches <= 3 {
                        ch_divergences.push(format!(
                            "dac note[{}] SAMPLE: orig=${:02X} vs rt=${:02X}",
                            j, o_sample, r_sample
                        ));
                    }
                }
                if o_dur != r_dur {
                    dac_dur_mismatches += 1;
                    if dac_dur_mismatches <= 3 {
                        ch_divergences.push(format!(
                            "dac note[{}] DURATION: orig=${:02X} vs rt=${:02X}",
                            j, o_dur, r_dur
                        ));
                    }
                }
            }
            if dac_sample_mismatches > 3 {
                ch_divergences.push(format!(
                    "... and {} more DAC sample mismatches",
                    dac_sample_mismatches - 3
                ));
            }
            if dac_dur_mismatches > 3 {
                ch_divergences.push(format!(
                    "... and {} more DAC duration mismatches",
                    dac_dur_mismatches - 3
                ));
            }

            if ch_divergences.is_empty() {
                channels_matched += 1;
                eprintln!("  RESULT: MATCHED");
            } else {
                channels_diverged += 1;
                eprintln!("  RESULT: {} divergences", ch_divergences.len());
                for d in &ch_divergences {
                    eprintln!("    {}", d);
                    divergence_details.push(format!("[{}] {}", label, d));
                }
                total_divergences += ch_divergences.len();
            }
        }

        // ----------------------------------------------------------------
        // 5. Voice bank: byte-exact comparison
        // ----------------------------------------------------------------
        eprintln!("\n--- Voice Bank ---");

        // Resolve the original UVB voices for comparison
        let uvb_source = include_str!("../../test_data/UniBank.asm");
        let uvb_voices =
            smps_parser::parse_voice_bank(uvb_source).unwrap();

        // Collect the set of voice indices actually used in original channels
        let mut used_voice_indices: Vec<u8> = Vec::new();
        for ch in &orig.channels {
            if ch.kind != SmpsChannelKind::Fm {
                continue;
            }
            for ev in &ch.events {
                if let ParsedEvent::SetVoice(v) = ev {
                    if !used_voice_indices.contains(v) {
                        used_voice_indices.push(*v);
                    }
                }
            }
        }
        used_voice_indices.sort();

        eprintln!(
            "  original UVB indices used: {:?}",
            used_voice_indices
        );
        eprintln!(
            "  reparsed voice count: {}",
            reparsed.voices.len()
        );

        // Compare each exported voice against its UVB source.
        // voices[export_idx] is the FmInstrument reference from build_voice_index.
        // Its name is "Voice N" where N is the UVB index.
        let mut voice_divergences: Vec<String> = Vec::new();
        for (export_idx, export_bytes) in reparsed.voices.iter().enumerate() {
            let inst = voices.get(export_idx);
            if inst.is_none() {
                voice_divergences.push(format!(
                    "voice[{}]: no matching instrument in export list",
                    export_idx
                ));
                continue;
            }
            let inst = inst.unwrap();

            // Parse UVB index from instrument name "Voice N"
            let uvb_idx: Option<usize> = inst
                .name
                .strip_prefix("Voice ")
                .and_then(|s| s.parse().ok());
            if uvb_idx.is_none() {
                voice_divergences.push(format!(
                    "voice[{}] '{}': cannot determine UVB index",
                    export_idx, inst.name
                ));
                continue;
            }
            let uvb_idx = uvb_idx.unwrap();

            if uvb_idx >= uvb_voices.len() {
                voice_divergences.push(format!(
                    "voice[{}] '{}': UVB index {} out of range (bank has {})",
                    export_idx, inst.name, uvb_idx, uvb_voices.len()
                ));
                continue;
            }

            let orig_bytes = &uvb_voices[uvb_idx];
            let mut byte_mismatches: Vec<String> = Vec::new();

            for b in 0..25 {
                if orig_bytes[b] != export_bytes[b] {
                    let field = match b {
                        0..=3 => format!("DT|MUL op{}", b + 1),
                        4..=7 => format!("RS|AR op{}", b - 3),
                        8..=11 => format!("AM|D1R op{}", b - 7),
                        12..=15 => format!("D2R op{}", b - 11),
                        16..=19 => format!("SL|RR op{}", b - 15),
                        20..=23 => {
                            let op = b - 19;
                            let o_carrier = orig_bytes[b] & 0x80 != 0;
                            let r_carrier = export_bytes[b] & 0x80 != 0;
                            let o_tl = orig_bytes[b] & 0x7F;
                            let r_tl = export_bytes[b] & 0x7F;
                            format!(
                                "TL op{} (carrier: orig={} rt={}, level: orig={} rt={})",
                                op, o_carrier, r_carrier, o_tl, r_tl
                            )
                        }
                        24 => "FB|ALG".to_string(),
                        _ => format!("byte[{}]", b),
                    };
                    byte_mismatches.push(format!(
                        "byte[{:2}] ({}) orig=${:02X} vs rt=${:02X}",
                        b, field, orig_bytes[b], export_bytes[b]
                    ));
                }
            }

            if byte_mismatches.is_empty() {
                eprintln!(
                    "  voice[{}] '{}' (UVB {}): 25/25 bytes MATCH",
                    export_idx, inst.name, uvb_idx
                );
            } else {
                eprintln!(
                    "  voice[{}] '{}' (UVB {}): {} byte mismatches:",
                    export_idx,
                    inst.name,
                    uvb_idx,
                    byte_mismatches.len()
                );
                for m in &byte_mismatches {
                    eprintln!("    {}", m);
                    voice_divergences.push(format!(
                        "VOICE[{}] UVB{}: {}",
                        export_idx, uvb_idx, m
                    ));
                }
            }
        }
        total_divergences += voice_divergences.len();
        divergence_details.extend(voice_divergences);

        // ----------------------------------------------------------------
        // 6. Summary
        // ----------------------------------------------------------------
        eprintln!("\n========================================");
        eprintln!("  SUMMARY");
        eprintln!("========================================");
        eprintln!(
            "  Channels compared : {}",
            fm_pairs + psg_pairs + dac_pairs
        );
        eprintln!("  Channels matched  : {}", channels_matched);
        eprintln!("  Channels diverged : {}", channels_diverged);
        eprintln!("  Total divergences : {}", total_divergences);
        if !divergence_details.is_empty() {
            eprintln!("\n  ALL DIVERGENCES:");
            for (i, d) in divergence_details.iter().enumerate() {
                eprintln!("    {:3}. {}", i + 1, d);
            }
        }
        eprintln!("========================================\n");

        // Structural assertions: the pipeline completed and produced valid output.
        assert!(
            !reparsed.channels.is_empty(),
            "reparsed export must have channels"
        );
        assert!(
            !reparsed.voices.is_empty(),
            "reparsed export must have voices"
        );
        assert_eq!(
            orig_fm.len(),
            rt_fm.len(),
            "FM channel count must survive roundtrip"
        );
    }

    /// Where the Zyrinx test ROM sits RELATIVE TO the directory this repo is
    /// checked out in. A commercial ROM cannot live in the repo, so like the
    /// S3K disassembly it is a sibling, resolved against the same parent by
    /// the same `test_support::sibling_root` (audit F41). Overridable with
    /// `SERAPH_ZYRINX_ROM`.
    const ZYRINX_ROM_RELATIVE: &str =
        "The Adventures of Batman and Robin/Adventures of Batman & Robin, The (USA).md";

    /// Last-ditch root, used only when git cannot answer. Relative to the
    /// process's working directory, which for `cargo test` is `src-tauri/` in
    /// the MAIN checkout — right there and wrong from a linked worktree, which
    /// is exactly why `sibling_root` is tried first.
    const ZYRINX_ROOT_FALLBACK: &str = "../..";

    /// Resolves the Zyrinx test ROM, or **fails the test** explaining how to
    /// proceed (audit F32).
    ///
    /// These tests used to hardcode an absolute path under one user's home
    /// directory and `return` early when it was missing, which meant that on
    /// any other tree they REPORTED GREEN HAVING CHECKED NOTHING. A test that
    /// cannot run must say so; it must never pass by default.
    ///
    /// Returning `None` requires the opt-out to be set deliberately, so
    /// skipping is always a conscious act by someone who has read this.
    fn zyrinx_test_rom() -> Option<std::path::PathBuf> {
        let rom_path = match std::env::var("SERAPH_ZYRINX_ROM") {
            Ok(explicit) => std::path::PathBuf::from(explicit),
            Err(_) => crate::test_support::sibling_root()
                .unwrap_or_else(|| std::path::PathBuf::from(ZYRINX_ROOT_FALLBACK))
                .join(ZYRINX_ROM_RELATIVE),
        };
        if rom_path.exists() {
            return Some(rom_path);
        }
        if std::env::var("SERAPH_SKIP_ROM_TESTS").is_ok() {
            eprintln!(
                "SERAPH_SKIP_ROM_TESTS set; skipping Zyrinx ROM test (looked at {})",
                rom_path.display(),
            );
            return None;
        }
        panic!(
            "Zyrinx test ROM not found at {}.\n\
             This test verifies importing a real commercial ROM and cannot check \
             anything without it, so it FAILS rather than passing silently (F32).\n\
             Either set SERAPH_ZYRINX_ROM to the ROM's path, or set \
             SERAPH_SKIP_ROM_TESTS=1 to skip it deliberately.",
            rom_path.display(),
        );
    }

    #[test]
    fn test_import_zyrinx_batman_main_title() {
        let Some(rom_path) = zyrinx_test_rom() else { return };

        let tmp = tempfile::tempdir().unwrap();
        let result = super::import_zyrinx_rom(&rom_path, tmp.path(), 1, &Default::default()).unwrap();
        let project_dir = PathBuf::from(&result.project_dir);

        assert!(project_dir.join("project.json").exists());
        assert!(project_dir.join(".seraph").exists());
        assert!(project_dir.join("instruments/fm").exists());
        assert!(result.track_count >= 1);
        assert!(result.instrument_count > 0);

        eprintln!("=== Zyrinx Import: {} ===", result.metadata.name);
        eprintln!("  BPM: {:.1}", result.metadata.tempo);
        eprintln!("  Tracks: {}", result.track_count);
        eprintln!("  FM Instruments: {}", result.instrument_count);
        eprintln!("  Warnings: {}", result.warnings.len());
        for w in &result.warnings {
            eprintln!("    [{}] {}", w.channel, w.message);
        }

        let json = std::fs::read_to_string(project_dir.join("project.json")).unwrap();
        let pf: crate::model::song::ProjectFile = serde_json::from_str(&json).unwrap();
        assert_eq!(pf.metadata.driver_id, "zyrinx");

        let total_notes: usize = pf.tracks.iter()
            .flat_map(|t| t.regions.iter())
            .map(|r| r.notes.len())
            .sum();
        eprintln!("  Total notes: {}", total_notes);
        assert!(total_notes > 0, "imported song must have notes");
    }

    #[test]
    fn test_import_all_zyrinx_songs() {
        let Some(rom_path) = zyrinx_test_rom() else { return };

        let tmp = tempfile::tempdir().unwrap();
        // SECOND DEFECT, found while fixing F32's path handling and not named
        // in its booking: this loop used to print `ERROR` on a failed import
        // and assert NOTHING afterwards. Every one of the 19 songs could have
        // failed to import and the test still passed. A print harness in a
        // test's clothing is the same defect class as the silent skip above,
        // one level in: it cannot fail, so it cannot report.
        let mut failures: Vec<String> = Vec::new();
        let mut noteless: Vec<String> = Vec::new();
        let mut imported = 0usize;

        for game_id in 1u8..20 {
            let result = super::import_zyrinx_rom(&rom_path, tmp.path(), game_id, &Default::default());
            match result {
                Ok(r) => {
                    let notes: usize = {
                        let json = std::fs::read_to_string(
                            PathBuf::from(&r.project_dir).join("project.json")
                        ).unwrap();
                        let pf: crate::model::song::ProjectFile = serde_json::from_str(&json).unwrap();
                        pf.tracks.iter().flat_map(|t| t.regions.iter()).map(|r| r.notes.len()).sum()
                    };
                    eprintln!(
                        "[{:02}] {}: {} tracks, {} instruments, {} notes, {} warnings",
                        game_id, r.metadata.name, r.track_count, r.instrument_count, notes, r.warnings.len()
                    );
                    imported += 1;
                    if notes == 0 {
                        noteless.push(format!("[{game_id:02}] {}", r.metadata.name));
                    }
                }
                Err(e) => {
                    eprintln!("[{:02}] ERROR: {}", game_id, e);
                    failures.push(format!("[{game_id:02}] {e}"));
                }
            }
        }

        // SLOT 0 IS DELIBERATELY OUT OF RANGE, and this is a finding, not an
        // oversight. The driver reports `valid: 0-19`, so slot 0 exists, and
        // it does import -- as "Silence", with 6,868 notes, which is BYTE FOR
        // BYTE the note count of slot 1 "Main Title". A song called Silence
        // carrying another song's note count suggests slot 0 resolves to slot
        // 1's data. Extending this gate over it would ratify behaviour nobody
        // has verified, so the range stays 1..20 and the question is booked as
        // F35 instead.
        // MEASURED, not booked: all 19 song slots import cleanly from this ROM
        // with 0 errors and 0 warnings, note counts ranging 5,702..571,753.
        // The gate asserts the invariants that measurement established, not a
        // transcribed snapshot: every slot imports, and an imported song has
        // notes. A song that imports to silence is a failure wearing success.
        assert!(
            failures.is_empty(),
            "every Zyrinx song slot must import; {} of 19 failed: {:?}",
            failures.len(), failures,
        );
        assert!(
            noteless.is_empty(),
            "an imported song with zero notes is a silent import failure: {:?}",
            noteless,
        );
        assert_eq!(
            imported, 19,
            "expected all 19 Zyrinx song slots to import from this ROM, got {imported}",
        );
    }

    #[test]
    fn test_recognize_instruments_renames_matches_only() {
        use crate::library::entry::{content_hash, LibraryInstrument};
        use crate::model::instrument::{FmInstrument, FmOperator, InstrumentBank, InstrumentMetadata};
        use uuid::Uuid;

        let matching = FmInstrument {
            id: Uuid::new_v4(),
            name: "Voice 0".into(),
            algorithm: 4,
            feedback: 5,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata { category: "Imported".into(), ..Default::default() },
        };
        let mut other = matching.clone();
        other.id = Uuid::new_v4();
        other.name = "Voice 1".into();
        other.algorithm = 2; // different sound -> different hash

        let hash = content_hash(&LibraryInstrument::Fm(matching.clone()));
        let table: RecognitionTable = [(hash, "EHZ Lead".to_string())].into_iter().collect();

        let mut bank = InstrumentBank { fm: vec![matching, other], psg: vec![], dac: vec![] };
        recognize_instruments(&mut bank, &table);

        assert_eq!(bank.fm[0].name, "EHZ Lead", "hash hit takes the library name");
        assert_eq!(bank.fm[0].metadata.category, "Imported \u{b7} Voice 0",
            "origin marker kept, generic name preserved in category");
        assert_eq!(bank.fm[1].name, "Voice 1", "miss keeps its generic name");
        assert_eq!(bank.fm[1].metadata.category, "Imported", "miss category untouched");
    }
}
