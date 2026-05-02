pub mod psg_envelopes;
pub mod smps_mapper;
pub mod smps_parser;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub project_dir: String,
    pub metadata: crate::model::song::SongMetadata,
    pub track_count: usize,
    pub instrument_count: usize,
    pub warnings: Vec<ImportWarning>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportWarning {
    pub channel: String,
    pub message: String,
}

pub fn import_smps_file(
    source_path: &std::path::Path,
    parent_dir: &std::path::Path,
    driver: &dyn crate::model::driver::DriverProfile,
) -> Result<ImportResult, String> {
    let source = std::fs::read_to_string(source_path)
        .map_err(|e| format!("failed to read {}: {e}", source_path.display()))?;

    let smps = smps_parser::parse_smps(&source)?;
    let mapped = smps_mapper::map_smps_to_song(&smps, driver)?;
    let song = mapped.song;

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
        project_dir.join(".megadaw"),
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
        assert!(project_dir.join(".megadaw").exists());
        assert!(project_dir.join("instruments/fm").exists());
        assert_eq!(result.track_count, 9);
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
        let fm1 = fm_tracks.iter().find(|t| t.name == "FM1").unwrap();
        assert!(!fm1.regions.is_empty());
        assert!(!fm1.regions[0].notes.is_empty());
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
}
