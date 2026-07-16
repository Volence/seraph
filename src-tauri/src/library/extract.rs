//! Batch extraction: game data -> library entry files.
//! Reuses the import parsers verbatim; dedups by content hash; deterministic
//! output (idempotent re-runs produce zero diff).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::driver::FlamedriverProfile;
use crate::import::{fm_formats, psg_envelopes, smps_parser, zyrinx_mapper, zyrinx_parser};
use crate::library::entry::{
    content_hash, LibraryEntryFile, LibraryInstrument, Provenance, LIBRARY_SCHEMA,
};
use crate::library::store::write_entry;
use crate::model::driver::DriverProfile;
use crate::model::instrument::{FmInstrument, InstrumentMetadata, PsgInstrument};

pub struct ExtractStats {
    pub songs: u32,
    pub voices_seen: u32,
    pub unique_written: u32,
}

/// Extract every FM voice from every .asm song in `in_dir` (sorted for
/// determinism), dedup by hash, write to `out_dir`.
pub fn extract_smps_dir(in_dir: &Path, game: &str, out_dir: &Path) -> Result<ExtractStats, String> {
    let mut files: Vec<_> = fs::read_dir(in_dir).map_err(|e| e.to_string())?
        .flatten().map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "asm"))
        .collect();
    files.sort();
    let mut stats = ExtractStats { songs: 0, voices_seen: 0, unique_written: 0 };
    // hash -> entry (accumulate provenance across songs before writing)
    let mut entries: HashMap<String, LibraryEntryFile> = HashMap::new();
    let mut order: Vec<String> = Vec::new(); // stable insertion order
    for f in files {
        let src = fs::read_to_string(&f).map_err(|e| e.to_string())?;
        let smps = match smps_parser::parse_smps(&src) {
            Ok(s) => s,
            Err(e) => { eprintln!("skip {}: {e}", f.display()); continue; }
        };
        stats.songs += 1;
        let song = song_stem(&f);
        for (i, voice) in smps.voices.iter().enumerate() {
            stats.voices_seen += 1;
            let mut inst = fm_voice_to_instrument(voice)?;
            // CRITICAL for idempotency: importers assign fresh random UUIDs.
            // Library files must be deterministic — nil the id (a real id is
            // re-assigned by add_fm_instrument when added to a project).
            inst.id = uuid::Uuid::nil();
            let li_probe = LibraryInstrument::Fm(inst.clone());
            let hash = content_hash(&li_probe);
            match entries.get_mut(&hash) {
                Some(e) => {
                    if !e.provenance.songs.contains(&song) { e.provenance.songs.push(song.clone()); }
                }
                None => {
                    inst.name = format!("{song} voice {i:02}");
                    inst.metadata = InstrumentMetadata {
                        category: game.to_string(), author: String::new(), tags: vec![],
                    };
                    let name = inst.name.clone();
                    entries.insert(hash.clone(), LibraryEntryFile {
                        schema: LIBRARY_SCHEMA, name, tags: vec![],
                        provenance: Provenance {
                            game: game.to_string(), songs: vec![song.clone()],
                            slot: Some(i as u8), hash: hash.clone(),
                        },
                        instrument: LibraryInstrument::Fm(inst),
                    });
                    order.push(hash);
                }
            }
        }
    }
    write_game_meta(out_dir, game)?;
    for h in order {
        write_entry(out_dir, &entries[&h])?;
        stats.unique_written += 1;
    }
    Ok(stats)
}

/// 25-byte SMPS voice -> FmInstrument, via the same driver conversion the
/// song importer uses (`smps_mapper::map_smps_to_song_with_dac` calls
/// `driver.fm_from_bytes`; the registered driver is `FlamedriverProfile`).
fn fm_voice_to_instrument(voice: &[u8; 25]) -> Result<FmInstrument, String> {
    FlamedriverProfile.fm_from_bytes(voice)
}

pub fn extract_gyb(file: &Path, game: &str, out_dir: &Path) -> Result<ExtractStats, String> {
    let data = fs::read(file).map_err(|e| e.to_string())?;
    let fname = file.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let res = fm_formats::import_fm_file(&data, &fname)?;
    write_game_meta(out_dir, game)?;
    let mut stats = ExtractStats { songs: 0, voices_seen: 0, unique_written: 0 };
    let mut seen = std::collections::HashSet::new();
    for mut inst in res.instruments {
        stats.voices_seen += 1;
        inst.id = uuid::Uuid::nil(); // determinism — see extract_smps_dir note
        let li = LibraryInstrument::Fm(inst);
        let hash = content_hash(&li);
        if !seen.insert(hash.clone()) { continue; }
        let name = match &li { LibraryInstrument::Fm(i) => i.name.clone(), _ => unreachable!() };
        write_entry(out_dir, &LibraryEntryFile {
            schema: LIBRARY_SCHEMA, name, tags: vec![],
            provenance: Provenance { game: game.into(), songs: vec![], slot: None, hash },
            instrument: li,
        })?;
        stats.unique_written += 1;
    }
    Ok(stats)
}

/// AoBR: iterate all 20 songs, collect + dedup voices.
pub fn extract_zyrinx(rom_path: &Path, game: &str, out_dir: &Path) -> Result<ExtractStats, String> {
    let rom = fs::read(rom_path).map_err(|e| e.to_string())?;
    let mut stats = ExtractStats { songs: 0, voices_seen: 0, unique_written: 0 };
    let mut entries: HashMap<String, LibraryEntryFile> = HashMap::new();
    let mut order: Vec<String> = Vec::new(); // stable insertion order
    for game_id in 0..zyrinx_parser::GAME_SONG_NAMES.len() as u8 {
        let zy = match zyrinx_parser::parse_zyrinx_song(&rom, game_id) {
            Ok(z) => z,
            // Some songs fail to parse (bank quirks) — skip, don't abort.
            Err(e) => { eprintln!("skip zyrinx song {game_id}: {e}"); continue; }
        };
        stats.songs += 1;
        let song = zyrinx_parser::GAME_SONG_NAMES[game_id as usize].to_string();
        for voice in &zy.voices {
            stats.voices_seen += 1;
            let mut inst = zyrinx_mapper::zyrinx_voice_to_fm(voice);
            inst.id = uuid::Uuid::nil(); // determinism — see extract_smps_dir note
            let li_probe = LibraryInstrument::Fm(inst.clone());
            let hash = content_hash(&li_probe);
            match entries.get_mut(&hash) {
                Some(e) => {
                    if !e.provenance.songs.contains(&song) { e.provenance.songs.push(song.clone()); }
                }
                None => {
                    inst.name = format!("{song} voice {:02}", voice.index);
                    inst.metadata = InstrumentMetadata {
                        category: game.to_string(), author: String::new(), tags: vec![],
                    };
                    let name = inst.name.clone();
                    entries.insert(hash.clone(), LibraryEntryFile {
                        schema: LIBRARY_SCHEMA, name, tags: vec![],
                        provenance: Provenance {
                            game: game.to_string(), songs: vec![song.clone()],
                            slot: Some(voice.index), hash: hash.clone(),
                        },
                        instrument: LibraryInstrument::Fm(inst),
                    });
                    order.push(hash);
                }
            }
        }
    }
    write_game_meta(out_dir, game)?;
    for h in order {
        write_entry(out_dir, &entries[&h])?;
        stats.unique_written += 1;
    }
    Ok(stats)
}

/// The bundled Flamedriver PSG envelopes -> presets (generated once).
/// Attenuation -> volume conversion mirrors `smps_mapper::resolve_psg_env`:
/// volume = 15 - attenuation (clamped); loop point / silence flags carried.
pub fn extract_psg_table(out_dir: &Path) -> Result<ExtractStats, String> {
    write_game_meta(out_dir, "SMPS PSG")?;
    let mut stats = ExtractStats { songs: 0, voices_seen: 0, unique_written: 0 };
    for (idx, env) in psg_envelopes::FLAMEDRIVER_PSG_ENVELOPES.iter().enumerate() {
        stats.voices_seen += 1;
        let volumes: Vec<u8> = env.volumes.iter().map(|&v| {
            let atten = (v as u8).min(15);
            15 - atten
        }).collect();
        let inst = PsgInstrument {
            id: uuid::Uuid::nil(), // determinism — see extract_smps_dir note
            name: format!("smps env {idx:02X}"),
            volume_sequence: volumes,
            loop_point: env.loop_point,
            silence_on_end: env.silence_on_end,
            noise_mode: None,
            smps_envelope_index: Some(idx as u8),
            metadata: InstrumentMetadata {
                category: "SMPS PSG".into(), author: String::new(), tags: vec![],
            },
        };
        let name = inst.name.clone();
        let li = LibraryInstrument::Psg(inst);
        let hash = content_hash(&li);
        write_entry(out_dir, &LibraryEntryFile {
            schema: LIBRARY_SCHEMA, name, tags: vec![],
            provenance: Provenance {
                game: "SMPS PSG".into(), songs: vec![], slot: Some(idx as u8), hash,
            },
            instrument: li,
        })?;
        stats.unique_written += 1;
    }
    Ok(stats)
}

fn write_game_meta(out_dir: &Path, game: &str) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let p = out_dir.join("_game.json");
    let body = serde_json::json!({ "name": game }).to_string() + "\n";
    if fs::read_to_string(&p).ok().as_deref() != Some(&body) {
        fs::write(p, body).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn song_stem(p: &Path) -> String {
    p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// Minimal valid SMPS song (macro dialect per s2disasm) whose single
    /// voice is copied verbatim from s2disasm OOZ.asm voice $00.
    fn fixture_song(p: &str) -> String {
        format!(
            r"{p}_Header:
	smpsHeaderStartSong 2
	smpsHeaderVoice     {p}_Voices
	smpsHeaderChan      $01, $00
	smpsHeaderTempo     $01, $00

	smpsHeaderFM        {p}_FM1, $00, $00

{p}_FM1:
	smpsSetvoice        $00
	smpsStop

{p}_Voices:
;	Voice $00 — copied from s2disasm 84 - OOZ.asm
	smpsVcAlgorithm     $01
	smpsVcFeedback      $07
	smpsVcUnusedBits    $00
	smpsVcDetune        $00, $03, $06, $00
	smpsVcCoarseFreq    $01, $00, $00, $06
	smpsVcRateScale     $01, $01, $00, $00
	smpsVcAttackRate    $1F, $1F, $3F, $3F
	smpsVcAmpMod        $00, $00, $00, $00
	smpsVcDecayRate1    $09, $13, $0F, $11
	smpsVcDecayRate2    $03, $04, $04, $05
	smpsVcDecayLevel    $02, $02, $02, $02
	smpsVcReleaseRate   $0F, $0F, $0F, $0F
	smpsVcTotalLevel    $80, $97, $2C, $23
"
        )
    }

    /// Relative path -> file contents, for byte-identical idempotency checks.
    fn dir_snapshot(dir: &Path) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    let rel = p.strip_prefix(dir).unwrap().to_string_lossy().to_string();
                    m.insert(rel, fs::read_to_string(&p).unwrap());
                }
            }
        }
        m
    }

    #[test]
    fn smps_extraction_dedups_and_unions_provenance() {
        let t = tempfile::tempdir().unwrap();
        let in_dir = t.path().join("in");
        std::fs::create_dir_all(&in_dir).unwrap();
        std::fs::write(in_dir.join("song-a.asm"), fixture_song("Song_A")).unwrap();
        std::fs::write(in_dir.join("song-b.asm"), fixture_song("Song_B")).unwrap(); // same voice
        let out = t.path().join("out");
        let stats = extract_smps_dir(&in_dir, "TestGame", &out).unwrap();
        assert_eq!(stats.songs, 2);
        assert_eq!(stats.voices_seen, 2);
        assert_eq!(stats.unique_written, 1); // deduped
        // idempotency: second run — no file content changes
        let before = dir_snapshot(&out);
        extract_smps_dir(&in_dir, "TestGame", &out).unwrap();
        assert_eq!(before, dir_snapshot(&out));
        // provenance union
        let entry_path = std::fs::read_dir(out.join("fm")).unwrap().next().unwrap().unwrap().path();
        let e: crate::library::entry::LibraryEntryFile =
            serde_json::from_str(&std::fs::read_to_string(entry_path).unwrap()).unwrap();
        assert_eq!(e.provenance.songs, vec!["song-a", "song-b"]);
        assert_eq!(e.name, "song-a voice 00");
        // deterministic identity: nil id, real patch values from the fixture
        match e.instrument {
            LibraryInstrument::Fm(inst) => {
                assert_eq!(inst.id, uuid::Uuid::nil());
                assert_eq!(inst.algorithm, 1);
                assert_eq!(inst.feedback, 7);
            }
            _ => panic!("expected fm entry"),
        }
    }

    #[test]
    fn psg_table_extraction_writes_all_envelopes_idempotently() {
        let t = tempfile::tempdir().unwrap();
        let out = t.path().join("out");
        let stats = extract_psg_table(&out).unwrap();
        assert_eq!(stats.unique_written, 52);
        assert_eq!(
            stats.unique_written as usize,
            psg_envelopes::FLAMEDRIVER_PSG_ENVELOPES.len()
        );
        assert_eq!(fs::read_dir(out.join("psg")).unwrap().count(), 52);
        // idempotency: second run — no file content changes
        let before = dir_snapshot(&out);
        extract_psg_table(&out).unwrap();
        assert_eq!(before, dir_snapshot(&out));
        // spot-check env $00 ([2] attenuation -> [13] volume, nil id)
        let e: crate::library::entry::LibraryEntryFile = serde_json::from_str(
            &fs::read_to_string(out.join("psg/smps-env-00.json")).unwrap(),
        ).unwrap();
        match e.instrument {
            LibraryInstrument::Psg(inst) => {
                assert_eq!(inst.id, uuid::Uuid::nil());
                assert_eq!(inst.volume_sequence, vec![13]);
                assert!(inst.silence_on_end);
                assert_eq!(inst.smps_envelope_index, Some(0));
            }
            _ => panic!("expected psg entry"),
        }
    }
}
