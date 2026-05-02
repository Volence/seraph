use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

use crate::model::driver::DriverRegistry;
use crate::model::instrument::*;
use crate::model::song::*;

pub struct ProjectManager {
    project_path: Option<PathBuf>,
    metadata: Option<SongMetadata>,
    tracks: Vec<Track>,
    instruments: InstrumentBank,
    dirty_instruments: HashSet<Uuid>,
    dac_pcm_cache: HashMap<Uuid, Arc<Vec<u8>>>,
    driver_registry: DriverRegistry,
}

impl ProjectManager {
    pub fn new(driver_registry: DriverRegistry) -> Self {
        Self {
            project_path: None,
            metadata: None,
            tracks: Vec::new(),
            instruments: InstrumentBank::default(),
            dirty_instruments: HashSet::new(),
            dac_pcm_cache: HashMap::new(),
            driver_registry,
        }
    }

    pub fn create(
        &mut self,
        path: &Path,
        name: &str,
        driver_id: &str,
        tempo: f64,
        time_sig: (u8, u8),
    ) -> Result<(), String> {
        if self.driver_registry.get(driver_id).is_none() {
            return Err(format!("unknown driver: {driver_id}"));
        }

        fs::create_dir_all(path).map_err(|e| e.to_string())?;
        fs::create_dir_all(path.join("instruments/fm")).map_err(|e| e.to_string())?;
        fs::create_dir_all(path.join("instruments/psg")).map_err(|e| e.to_string())?;
        fs::create_dir_all(path.join("instruments/dac")).map_err(|e| e.to_string())?;
        fs::create_dir_all(path.join("exports")).map_err(|e| e.to_string())?;

        let version = serde_json::json!({ "version": "0.1.0" });
        fs::write(
            path.join(".megadaw"),
            serde_json::to_string_pretty(&version).unwrap(),
        )
        .map_err(|e| e.to_string())?;

        let metadata = SongMetadata {
            name: name.to_string(),
            tempo,
            time_signature: time_sig,
            ticks_per_beat: 480,
            driver_id: driver_id.to_string(),
        };

        let project_file = ProjectFile {
            metadata: metadata.clone(),
            tracks: Vec::new(),
        };
        let json = serde_json::to_string_pretty(&project_file).map_err(|e| e.to_string())?;
        fs::write(path.join("project.json"), json).map_err(|e| e.to_string())?;

        self.project_path = Some(path.to_path_buf());
        self.metadata = Some(metadata);
        self.tracks = Vec::new();
        self.instruments = InstrumentBank::default();
        self.dirty_instruments.clear();
        self.dac_pcm_cache.clear();

        Ok(())
    }

    pub fn open(&mut self, path: &Path) -> Result<Song, String> {
        if !path.join(".megadaw").exists() {
            return Err("not a MegaDAW project (no .megadaw file)".into());
        }

        let json = fs::read_to_string(path.join("project.json")).map_err(|e| e.to_string())?;
        let project_file: ProjectFile =
            serde_json::from_str(&json).map_err(|e| e.to_string())?;

        let mut instruments = InstrumentBank::default();

        let fm_dir = path.join("instruments/fm");
        if fm_dir.exists() {
            for entry in fs::read_dir(&fm_dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.path().extension().map_or(false, |ext| ext == "json") {
                    let data = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
                    let inst: FmInstrument =
                        serde_json::from_str(&data).map_err(|e| e.to_string())?;
                    instruments.fm.push(inst);
                }
            }
        }

        let psg_dir = path.join("instruments/psg");
        if psg_dir.exists() {
            for entry in fs::read_dir(&psg_dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.path().extension().map_or(false, |ext| ext == "json") {
                    let data = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
                    let inst: PsgInstrument =
                        serde_json::from_str(&data).map_err(|e| e.to_string())?;
                    instruments.psg.push(inst);
                }
            }
        }

        let dac_dir = path.join("instruments/dac");
        if dac_dir.exists() {
            for entry in fs::read_dir(&dac_dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.path().extension().map_or(false, |ext| ext == "json") {
                    let data = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
                    let inst: DacInstrument =
                        serde_json::from_str(&data).map_err(|e| e.to_string())?;
                    let pcm_path = path.join("instruments/dac").join(&inst.pcm_file);
                    if pcm_path.exists() {
                        let pcm_data = fs::read(&pcm_path).map_err(|e| e.to_string())?;
                        self.dac_pcm_cache.insert(inst.id, Arc::new(pcm_data));
                    }
                    instruments.dac.push(inst);
                }
            }
        }

        self.project_path = Some(path.to_path_buf());
        self.metadata = Some(project_file.metadata.clone());
        self.tracks = project_file.tracks.clone();
        self.instruments = instruments.clone();
        self.dirty_instruments.clear();

        Ok(Song {
            metadata: project_file.metadata,
            tracks: project_file.tracks,
            instruments,
        })
    }

    pub fn save(&mut self) -> Result<(), String> {
        let path = self.project_path.as_ref().ok_or("no project open")?;
        let metadata = self.metadata.as_ref().ok_or("no project open")?.clone();

        let project_file = ProjectFile {
            metadata,
            tracks: self.tracks.clone(),
        };
        let json = serde_json::to_string_pretty(&project_file).map_err(|e| e.to_string())?;
        fs::write(path.join("project.json"), json).map_err(|e| e.to_string())?;

        for inst in &self.instruments.fm {
            if self.dirty_instruments.contains(&inst.id) {
                let json = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
                fs::write(path.join(format!("instruments/fm/{}.json", inst.id)), json)
                    .map_err(|e| e.to_string())?;
            }
        }
        for inst in &self.instruments.psg {
            if self.dirty_instruments.contains(&inst.id) {
                let json = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
                fs::write(path.join(format!("instruments/psg/{}.json", inst.id)), json)
                    .map_err(|e| e.to_string())?;
            }
        }
        for inst in &self.instruments.dac {
            if self.dirty_instruments.contains(&inst.id) {
                let json = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
                fs::write(path.join(format!("instruments/dac/{}.json", inst.id)), json)
                    .map_err(|e| e.to_string())?;
                if let Some(pcm) = self.dac_pcm_cache.get(&inst.id) {
                    fs::write(path.join("instruments/dac").join(&inst.pcm_file), pcm.as_ref())
                        .map_err(|e| e.to_string())?;
                }
            }
        }

        self.dirty_instruments.clear();
        Ok(())
    }

    pub fn close(&mut self) {
        self.project_path = None;
        self.metadata = None;
        self.tracks.clear();
        self.instruments = InstrumentBank::default();
        self.dirty_instruments.clear();
        self.dac_pcm_cache.clear();
    }

    pub fn is_open(&self) -> bool {
        self.project_path.is_some()
    }

    pub fn metadata(&self) -> Option<&SongMetadata> {
        self.metadata.as_ref()
    }

    pub fn project_path(&self) -> Option<&Path> {
        self.project_path.as_deref()
    }

    pub fn driver_registry(&self) -> &DriverRegistry {
        &self.driver_registry
    }

    pub fn song(&self) -> Option<Song> {
        self.metadata.as_ref().map(|meta| Song {
            metadata: meta.clone(),
            tracks: self.tracks.clone(),
            instruments: self.instruments.clone(),
        })
    }

    // --- FM CRUD ---

    pub fn add_fm_instrument(&mut self, mut inst: FmInstrument) -> Uuid {
        let id = Uuid::new_v4();
        inst.id = id;
        self.instruments.fm.push(inst);
        self.dirty_instruments.insert(id);
        id
    }

    pub fn update_fm_instrument(&mut self, id: Uuid, mut inst: FmInstrument) -> Result<(), String> {
        let existing = self.instruments.fm.iter_mut().find(|i| i.id == id)
            .ok_or("FM instrument not found")?;
        inst.id = id;
        *existing = inst;
        self.dirty_instruments.insert(id);
        Ok(())
    }

    pub fn delete_fm_instrument(&mut self, id: Uuid) -> Result<(), String> {
        let pos = self.instruments.fm.iter().position(|i| i.id == id)
            .ok_or("FM instrument not found")?;
        self.instruments.fm.remove(pos);
        self.dirty_instruments.remove(&id);
        if let Some(path) = &self.project_path {
            let file = path.join(format!("instruments/fm/{id}.json"));
            if file.exists() { let _ = fs::remove_file(file); }
        }
        Ok(())
    }

    pub fn list_fm_instruments(&self) -> &[FmInstrument] {
        &self.instruments.fm
    }

    pub fn get_fm_instrument(&self, id: &Uuid) -> Option<&FmInstrument> {
        self.instruments.fm.iter().find(|i| &i.id == id)
    }

    // --- PSG CRUD ---

    pub fn add_psg_instrument(&mut self, mut inst: PsgInstrument) -> Uuid {
        let id = Uuid::new_v4();
        inst.id = id;
        self.instruments.psg.push(inst);
        self.dirty_instruments.insert(id);
        id
    }

    pub fn update_psg_instrument(&mut self, id: Uuid, mut inst: PsgInstrument) -> Result<(), String> {
        let existing = self.instruments.psg.iter_mut().find(|i| i.id == id)
            .ok_or("PSG instrument not found")?;
        inst.id = id;
        *existing = inst;
        self.dirty_instruments.insert(id);
        Ok(())
    }

    pub fn delete_psg_instrument(&mut self, id: Uuid) -> Result<(), String> {
        let pos = self.instruments.psg.iter().position(|i| i.id == id)
            .ok_or("PSG instrument not found")?;
        self.instruments.psg.remove(pos);
        self.dirty_instruments.remove(&id);
        if let Some(path) = &self.project_path {
            let file = path.join(format!("instruments/psg/{id}.json"));
            if file.exists() { let _ = fs::remove_file(file); }
        }
        Ok(())
    }

    pub fn list_psg_instruments(&self) -> &[PsgInstrument] {
        &self.instruments.psg
    }

    pub fn get_psg_instrument(&self, id: &Uuid) -> Option<&PsgInstrument> {
        self.instruments.psg.iter().find(|i| &i.id == id)
    }

    // --- DAC CRUD ---

    pub fn add_dac_instrument(&mut self, inst: DacInstrument, pcm_data: Vec<u8>) -> Uuid {
        let id = inst.id;
        self.dac_pcm_cache.insert(id, Arc::new(pcm_data));
        self.instruments.dac.push(inst);
        self.dirty_instruments.insert(id);
        id
    }

    pub fn update_dac_instrument(&mut self, id: Uuid, mut inst: DacInstrument) -> Result<(), String> {
        let existing = self.instruments.dac.iter_mut().find(|i| i.id == id)
            .ok_or("DAC instrument not found")?;
        inst.id = id;
        *existing = inst;
        self.dirty_instruments.insert(id);
        Ok(())
    }

    pub fn delete_dac_instrument(&mut self, id: Uuid) -> Result<(), String> {
        let pos = self.instruments.dac.iter().position(|i| i.id == id)
            .ok_or("DAC instrument not found")?;
        let inst = self.instruments.dac.remove(pos);
        self.dirty_instruments.remove(&id);
        self.dac_pcm_cache.remove(&id);
        if let Some(path) = &self.project_path {
            for name in [
                format!("instruments/dac/{id}.json"),
                format!("instruments/dac/{}", inst.pcm_file),
                format!("instruments/dac/{}", inst.original_file),
            ] {
                let p = path.join(&name);
                if p.exists() { let _ = fs::remove_file(p); }
            }
        }
        Ok(())
    }

    pub fn list_dac_instruments(&self) -> &[DacInstrument] {
        &self.instruments.dac
    }

    pub fn get_dac_instrument(&self, id: &Uuid) -> Option<&DacInstrument> {
        self.instruments.dac.iter().find(|i| &i.id == id)
    }

    pub fn get_dac_pcm(&self, id: &Uuid) -> Option<Arc<Vec<u8>>> {
        self.dac_pcm_cache.get(id).cloned()
    }

    pub fn update_dac_pcm(&mut self, id: Uuid, pcm_data: Vec<u8>) {
        self.dac_pcm_cache.insert(id, Arc::new(pcm_data));
        self.dirty_instruments.insert(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::FlamedriverProfile;
    use std::env;

    fn test_registry() -> DriverRegistry {
        let mut reg = DriverRegistry::new();
        reg.register(Box::new(FlamedriverProfile));
        reg
    }

    fn temp_project_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!("megadaw_test_{name}_{}", Uuid::new_v4()))
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn test_create_project_makes_folder_structure() {
        let path = temp_project_path("create");
        let mut mgr = ProjectManager::new(test_registry());

        mgr.create(&path, "Test Song", "flamedriver", 120.0, (4, 4)).unwrap();

        assert!(path.join(".megadaw").exists());
        assert!(path.join("project.json").exists());
        assert!(path.join("instruments/fm").is_dir());
        assert!(path.join("instruments/psg").is_dir());
        assert!(path.join("instruments/dac").is_dir());
        assert!(path.join("exports").is_dir());
        assert!(mgr.is_open());

        cleanup(&path);
    }

    #[test]
    fn test_create_rejects_unknown_driver() {
        let path = temp_project_path("bad_driver");
        let mut mgr = ProjectManager::new(test_registry());
        let result = mgr.create(&path, "X", "nonexistent", 120.0, (4, 4));
        assert!(result.is_err());
        cleanup(&path);
    }

    #[test]
    fn test_open_rejects_non_project() {
        let path = temp_project_path("not_project");
        fs::create_dir_all(&path).unwrap();
        let mut mgr = ProjectManager::new(test_registry());
        let result = mgr.open(&path);
        assert!(result.is_err());
        cleanup(&path);
    }

    #[test]
    fn test_create_save_open_round_trip() {
        let path = temp_project_path("round_trip");
        let mut mgr = ProjectManager::new(test_registry());

        mgr.create(&path, "Round Trip", "flamedriver", 150.0, (3, 4)).unwrap();

        let fm_inst = FmInstrument {
            id: Uuid::nil(),
            name: "Bass".into(),
            algorithm: 2,
            feedback: 3,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        };
        let fm_id = mgr.add_fm_instrument(fm_inst);

        let psg_inst = PsgInstrument {
            id: Uuid::nil(),
            name: "Pluck".into(),
            volume_sequence: vec![15, 12, 8, 4, 0],
            loop_point: None,
            noise_mode: None,
            metadata: InstrumentMetadata::default(),
        };
        let psg_id = mgr.add_psg_instrument(psg_inst);

        mgr.save().unwrap();
        mgr.close();
        assert!(!mgr.is_open());

        let song = mgr.open(&path).unwrap();
        assert_eq!(song.metadata.name, "Round Trip");
        assert_eq!(song.metadata.tempo, 150.0);
        assert_eq!(song.instruments.fm.len(), 1);
        assert_eq!(song.instruments.fm[0].id, fm_id);
        assert_eq!(song.instruments.fm[0].name, "Bass");
        assert_eq!(song.instruments.psg.len(), 1);
        assert_eq!(song.instruments.psg[0].id, psg_id);

        cleanup(&path);
    }

    #[test]
    fn test_delete_fm_removes_file() {
        let path = temp_project_path("delete_fm");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Del Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let inst = FmInstrument {
            id: Uuid::nil(),
            name: "ToDelete".into(),
            algorithm: 0,
            feedback: 0,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        };
        let id = mgr.add_fm_instrument(inst);
        mgr.save().unwrap();
        assert!(path.join(format!("instruments/fm/{id}.json")).exists());

        mgr.delete_fm_instrument(id).unwrap();
        assert!(!path.join(format!("instruments/fm/{id}.json")).exists());
        assert!(mgr.list_fm_instruments().is_empty());

        cleanup(&path);
    }

    #[test]
    fn test_close_clears_state() {
        let path = temp_project_path("close");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Test", "flamedriver", 120.0, (4, 4)).unwrap();
        mgr.close();
        assert!(!mgr.is_open());
        assert!(mgr.metadata().is_none());
        assert!(mgr.list_fm_instruments().is_empty());
        cleanup(&path);
    }

    #[test]
    fn test_get_dac_pcm_returns_cached_data() {
        let path = temp_project_path("dac_pcm");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "PCM Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let inst = DacInstrument {
            id: Uuid::new_v4(),
            name: "Test".into(),
            target_sample_rate: 16000,
            loop_start: None,
            loop_length: None,
            original_file: "test.raw".into(),
            pcm_file: "test.pcm".into(),
            source_is_raw: true,
            metadata: InstrumentMetadata::default(),
        };
        let pcm_data = vec![128u8, 130, 132, 134];
        let id = mgr.add_dac_instrument(inst, pcm_data.clone());

        let cached = mgr.get_dac_pcm(&id).unwrap();
        assert_eq!(cached.as_ref(), &pcm_data);

        assert!(mgr.get_dac_pcm(&Uuid::new_v4()).is_none());

        cleanup(&path);
    }
}
