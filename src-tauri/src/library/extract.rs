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

#[derive(Default)]
pub struct ExtractStats {
    pub songs: u32,
    pub voices_seen: u32,
    pub unique_written: u32,
    /// Inputs that failed to parse and were skipped (never aborts the batch).
    pub failed: u32,
}

/// Extract every FM voice from every .asm song in `in_dir` (sorted for
/// determinism), dedup by hash, write to `out_dir`.
pub fn extract_smps_dir(in_dir: &Path, game: &str, out_dir: &Path) -> Result<ExtractStats, String> {
    let mut files: Vec<_> = fs::read_dir(in_dir).map_err(|e| e.to_string())?
        .flatten().map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "asm"))
        .collect();
    files.sort();
    let mut stats = ExtractStats::default();
    // hash -> entry (accumulate provenance across songs before writing)
    let mut entries: HashMap<String, LibraryEntryFile> = HashMap::new();
    let mut order: Vec<String> = Vec::new(); // stable insertion order
    for f in files {
        let src = fs::read_to_string(&f).map_err(|e| e.to_string())?;
        let smps = match smps_parser::parse_smps(&src) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("skip {}: {e}", f.display());
                stats.failed += 1;
                continue;
            }
        };
        stats.songs += 1;
        // Provenance keeps the RAW file stem ("84 - OOZ"); display names get
        // the track-number prefix stripped ("OOZ voice 00").
        let song = song_stem(&f);
        let display = display_song_name(&song).to_string();
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
                    inst.name = format!("{display} voice {i:02}");
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
    write_game_meta(out_dir, game, &in_dir.display().to_string())?;
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
    write_game_meta(out_dir, game, &file.display().to_string())?;
    let mut stats = ExtractStats::default();
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
    let mut stats = ExtractStats::default();
    let mut entries: HashMap<String, LibraryEntryFile> = HashMap::new();
    let mut order: Vec<String> = Vec::new(); // stable insertion order
    for game_id in 0..zyrinx_parser::GAME_SONG_NAMES.len() as u8 {
        let zy = match zyrinx_parser::parse_zyrinx_song(&rom, game_id) {
            Ok(z) => z,
            // Some songs fail to parse (bank quirks) — skip, don't abort.
            Err(e) => {
                eprintln!("skip zyrinx song {game_id}: {e}");
                stats.failed += 1;
                continue;
            }
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
    write_game_meta(out_dir, game, &rom_path.display().to_string())?;
    for h in order {
        write_entry(out_dir, &entries[&h])?;
        stats.unique_written += 1;
    }
    Ok(stats)
}

/// The bundled Flamedriver PSG envelopes -> presets (generated once).
/// Attenuation -> volume conversion mirrors `smps_mapper::resolve_psg_env`:
/// volume = 15 - attenuation (clamped); loop point / silence flags carried.
///
/// Song-facing envelope indices are 1-BASED — the Z80 driver does `dec a`
/// before the table lookup (see `resolve_psg_env`: `env_idx.saturating_sub(1)`
/// for the table read, `smps_envelope_index: Some(env_idx)` stored
/// un-decremented; the exporter emits the field verbatim as `sTone_{:02X}`).
/// So table position `idx` is song index `idx + 1`, and index 0 means
/// "no envelope" — never a table entry.
///
/// Several table entries are byte-identical (e.g. sTone_02/sTone_0F); they
/// share a content hash, so the library index merge would hide the later
/// ones. Duplicates are folded into ONE entry whose name lists every song
/// index ("smps env 02/0F") and whose tags alias the folded indices
/// ("env-0f") so text search finds any of them.
pub fn extract_psg_table(out_dir: &Path) -> Result<ExtractStats, String> {
    write_game_meta(out_dir, "SMPS PSG", "flamedriver-psg-table")?;
    let mut stats = ExtractStats::default();
    // hash -> (instrument from first occurrence, all 1-based song indices)
    let mut entries: HashMap<String, (PsgInstrument, Vec<u8>)> = HashMap::new();
    let mut order: Vec<String> = Vec::new(); // stable insertion order
    for (idx, env) in psg_envelopes::FLAMEDRIVER_PSG_ENVELOPES.iter().enumerate() {
        stats.voices_seen += 1;
        let env_num = idx as u8 + 1; // 1-based song-facing index
        let volumes: Vec<u8> = env.volumes.iter().map(|&v| {
            let atten = (v as u8).min(15);
            15 - atten
        }).collect();
        let inst = PsgInstrument {
            id: uuid::Uuid::nil(), // determinism — see extract_smps_dir note
            name: String::new(), // finalized below once folded indices are known
            volume_sequence: volumes,
            loop_point: env.loop_point,
            silence_on_end: env.silence_on_end,
            noise_mode: None,
            smps_envelope_index: Some(env_num),
            metadata: InstrumentMetadata {
                category: "SMPS PSG".into(), author: String::new(), tags: vec![],
            },
        };
        let hash = content_hash(&LibraryInstrument::Psg(inst.clone()));
        match entries.get_mut(&hash) {
            Some((_, nums)) => nums.push(env_num),
            None => {
                entries.insert(hash.clone(), (inst, vec![env_num]));
                order.push(hash);
            }
        }
    }
    for h in order {
        let (mut inst, nums) = entries.remove(&h).expect("hash in order came from entries");
        let joined = nums.iter().map(|n| format!("{n:02X}")).collect::<Vec<_>>().join("/");
        inst.name = format!("smps env {joined}");
        let name = inst.name.clone();
        let tags: Vec<String> = nums[1..].iter().map(|n| format!("env-{n:02x}")).collect();
        // slot = the primary (first) 1-based song index, same numbering as
        // the name and smps_envelope_index.
        let slot = nums[0];
        write_entry(out_dir, &LibraryEntryFile {
            schema: LIBRARY_SCHEMA, name, tags,
            provenance: Provenance {
                game: "SMPS PSG".into(), songs: vec![], slot: Some(slot), hash: h,
            },
            instrument: LibraryInstrument::Psg(inst),
        })?;
        stats.unique_written += 1;
    }
    Ok(stats)
}

fn write_game_meta(out_dir: &Path, game: &str, source: &str) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let p = out_dir.join("_game.json");
    // No timestamps here — the file must be byte-identical across re-runs.
    let body = serde_json::json!({ "name": game, "source": source }).to_string() + "\n";
    if fs::read_to_string(&p).ok().as_deref() != Some(&body) {
        fs::write(p, body).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn song_stem(p: &Path) -> String {
    p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
}

/// Strip a leading track-number stem ("84 - OOZ" -> "OOZ") for display
/// names; provenance keeps the raw stem. Equivalent to `^\d+\s*-\s*` without
/// pulling in a regex dependency. Falls back to the raw stem when nothing
/// meaningful would remain.
fn display_song_name(stem: &str) -> &str {
    let after_digits = stem.trim_start_matches(|c: char| c.is_ascii_digit());
    if after_digits.len() == stem.len() {
        return stem; // no leading digits
    }
    match after_digits.trim_start().strip_prefix('-') {
        Some(rest) => {
            let rest = rest.trim_start();
            if rest.is_empty() { stem } else { rest }
        }
        None => stem, // digits not followed by " - " (e.g. "2fast")
    }
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
        // Track-numbered stems, s2disasm-style, to prove the display-name strip.
        std::fs::write(in_dir.join("10 - Song A.asm"), fixture_song("Song_A")).unwrap();
        std::fs::write(in_dir.join("11 - Song B.asm"), fixture_song("Song_B")).unwrap(); // same voice
        // Unparseable song: counted in `failed`, never aborts the batch.
        std::fs::write(
            in_dir.join("99 - Broken.asm"),
            "Broken_Header:\n\tsmpsHeaderStartSong 2\n\tsmpsHeaderFM Missing_Label, $00, $00\n",
        ).unwrap();
        let out = t.path().join("out");
        let stats = extract_smps_dir(&in_dir, "TestGame", &out).unwrap();
        assert_eq!(stats.songs, 2);
        assert_eq!(stats.voices_seen, 2);
        assert_eq!(stats.unique_written, 1); // deduped
        assert_eq!(stats.failed, 1);
        // idempotency: second run — no file content changes
        let before = dir_snapshot(&out);
        extract_smps_dir(&in_dir, "TestGame", &out).unwrap();
        assert_eq!(before, dir_snapshot(&out));
        // provenance union keeps the RAW stems
        let entry_path = std::fs::read_dir(out.join("fm")).unwrap().next().unwrap().unwrap().path();
        let e: crate::library::entry::LibraryEntryFile =
            serde_json::from_str(&std::fs::read_to_string(&entry_path).unwrap()).unwrap();
        assert_eq!(e.provenance.songs, vec!["10 - Song A", "11 - Song B"]);
        // display name has the track-number prefix stripped
        assert_eq!(e.name, "Song A voice 00");
        assert!(entry_path.ends_with("song-a-voice-00.json"));
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
    fn display_song_name_strips_track_number_prefix_only() {
        assert_eq!(display_song_name("84 - OOZ"), "OOZ");
        assert_eq!(display_song_name("10-Song A"), "Song A");
        assert_eq!(display_song_name("OOZ"), "OOZ"); // no prefix
        assert_eq!(display_song_name("2fast"), "2fast"); // digits, no dash
        assert_eq!(display_song_name("84 - "), "84 - "); // nothing left: raw stem
    }

    #[test]
    fn psg_table_extraction_folds_duplicates_idempotently() {
        let t = tempfile::tempdir().unwrap();
        let out = t.path().join("out");
        let stats = extract_psg_table(&out).unwrap();
        assert_eq!(
            stats.voices_seen as usize,
            psg_envelopes::FLAMEDRIVER_PSG_ENVELOPES.len()
        );
        assert_eq!(stats.voices_seen, 52);
        // duplicate envelope data folds into one entry per unique hash
        assert!(stats.unique_written < 52);
        assert_eq!(
            fs::read_dir(out.join("psg")).unwrap().count(),
            stats.unique_written as usize
        );
        // idempotency: second run — no file content changes
        let before = dir_snapshot(&out);
        extract_psg_table(&out).unwrap();
        assert_eq!(before, dir_snapshot(&out));
        // spot-check table entry 0 = song-facing env 01 (1-based: the Z80
        // driver does `dec a`), folded with its duplicate env 0E; volume
        // conversion: [2] attenuation -> [13].
        let e: crate::library::entry::LibraryEntryFile = serde_json::from_str(
            &fs::read_to_string(out.join("psg/smps-env-01-0e.json")).unwrap(),
        ).unwrap();
        assert_eq!(e.name, "smps env 01/0E");
        assert_eq!(e.tags, vec!["env-0e"]);
        assert_eq!(e.provenance.slot, Some(1));
        match e.instrument {
            LibraryInstrument::Psg(inst) => {
                assert_eq!(inst.id, uuid::Uuid::nil());
                assert_eq!(inst.volume_sequence, vec![13]);
                assert!(inst.silence_on_end);
                assert_eq!(inst.smps_envelope_index, Some(1));
            }
            _ => panic!("expected psg entry"),
        }
        // the coordinator-flagged three-way duplicate ($0B/$18/$1A table
        // positions = song envs 0C/19/1B) folds into one searchable entry
        let e3: crate::library::entry::LibraryEntryFile = serde_json::from_str(
            &fs::read_to_string(out.join("psg/smps-env-0c-19-1b.json")).unwrap(),
        ).unwrap();
        assert_eq!(e3.name, "smps env 0C/19/1B");
        assert_eq!(e3.tags, vec!["env-19", "env-1b"]);
    }
}
