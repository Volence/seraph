//! Library runtime state: resolved roots, the merged index, overrides.
//! Path resolution (config dir, bundled resource) happens HERE — store.rs
//! stays pure.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

use super::store::{self, IndexedEntry, Overrides};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct RootInfo {
    pub label: String,
    pub path: String,
    /// "bundled" | "user" | "custom"
    pub kind: String,
}

#[derive(Default)]
pub struct LibraryState {
    pub index: Mutex<Vec<IndexedEntry>>,
    pub overrides: Mutex<Overrides>,
    pub roots: Mutex<Vec<RootInfo>>,
    pub warnings: Mutex<Vec<String>>,
}

pub fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map_err(|e| e.to_string())
}

pub fn overrides_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(config_dir(app)?.join("library-overrides.json"))
}

fn custom_roots_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(config_dir(app)?.join("library-roots.json"))
}

pub fn user_root(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_data_dir().map_err(|e| e.to_string())?.join("library"))
}

/// Dev builds read the repo's library/ directly; release builds read the
/// bundled Tauri resource.
pub fn bundled_root(app: &AppHandle) -> Option<PathBuf> {
    #[cfg(debug_assertions)]
    {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../library");
        if p.exists() {
            return p.canonicalize().ok();
        }
    }
    app.path()
        .resolve("library", BaseDirectory::Resource)
        .ok()
        .filter(|p| p.exists())
}

pub fn resolve_roots(app: &AppHandle) -> Vec<RootInfo> {
    let mut roots = Vec::new();
    if let Some(b) = bundled_root(app) {
        roots.push(RootInfo { label: "Seraph Pack".into(), path: b.to_string_lossy().into(), kind: "bundled".into() });
    }
    if let Ok(u) = user_root(app) {
        let _ = std::fs::create_dir_all(&u);
        roots.push(RootInfo { label: "My Library".into(), path: u.to_string_lossy().into(), kind: "user".into() });
    }
    if let Ok(p) = custom_roots_path(app) {
        if let Ok(s) = std::fs::read_to_string(p) {
            if let Ok(customs) = serde_json::from_str::<Vec<String>>(&s) {
                for c in customs {
                    roots.push(RootInfo { label: c.clone(), path: c, kind: "custom".into() });
                }
            }
        }
    }
    roots
}

pub fn save_custom_roots(app: &AppHandle, roots: &[RootInfo]) -> Result<(), String> {
    let customs: Vec<&String> = roots.iter().filter(|r| r.kind == "custom").map(|r| &r.path).collect();
    let p = custom_roots_path(app)?;
    if let Some(parent) = p.parent() { std::fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    std::fs::write(p, serde_json::to_string_pretty(&customs).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

/// Load overrides WITHOUT silent data loss. `store::load_overrides` returns
/// empty on a corrupt file, and the next save would then permanently erase the
/// user's favorites/tags. Distinguish missing (normal first run) from corrupt:
/// warn AND quarantine the corrupt file to `library-overrides.json.bak-corrupt`
/// so a later save can't clobber it and the user can recover by hand.
fn load_overrides_guarded(path: &std::path::Path, warnings: &mut Vec<String>) -> Overrides {
    let s = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Overrides::default(),
        Err(e) => {
            warnings.push(format!("could not read overrides file {}: {e}", path.display()));
            return Overrides::default();
        }
    };
    match serde_json::from_str(&s) {
        Ok(o) => o,
        Err(e) => {
            let bak = path.with_file_name("library-overrides.json.bak-corrupt");
            match std::fs::rename(path, &bak) {
                Ok(()) => warnings.push(format!(
                    "overrides file was corrupt ({e}); moved to {} — favorites/tags are reset until it is restored",
                    bak.display()
                )),
                Err(re) => warnings.push(format!(
                    "overrides file {} is corrupt ({e}) and could not be quarantined ({re}); favorites/tags may be lost on next save",
                    path.display()
                )),
            }
            Overrides::default()
        }
    }
}

pub fn rescan(app: &AppHandle, state: &LibraryState) {
    let roots = resolve_roots(app);
    let mut all = Vec::new();
    let mut warns = Vec::new();
    for (i, r) in roots.iter().enumerate() {
        let (entries, w) = store::scan_root(std::path::Path::new(&r.path), &r.label, i);
        all.extend(entries);
        warns.extend(w);
    }
    let merged = store::merge(all);
    let overrides = overrides_path(app)
        .map(|p| load_overrides_guarded(&p, &mut warns))
        .unwrap_or_default();
    *state.index.lock().unwrap() = merged;
    *state.overrides.lock().unwrap() = overrides;
    *state.roots.lock().unwrap() = roots;
    *state.warnings.lock().unwrap() = warns;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guarded_load_missing_file_is_silent() {
        let t = tempfile::tempdir().unwrap();
        let mut warns = Vec::new();
        let o = load_overrides_guarded(&t.path().join("library-overrides.json"), &mut warns);
        assert!(o.is_empty());
        assert!(warns.is_empty());
    }

    #[test]
    fn guarded_load_corrupt_file_warns_and_quarantines() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("library-overrides.json");
        std::fs::write(&p, "{not json").unwrap();
        let mut warns = Vec::new();
        let o = load_overrides_guarded(&p, &mut warns);
        assert!(o.is_empty());
        assert_eq!(warns.len(), 1);
        assert!(warns[0].contains("corrupt"));
        // original is gone; quarantined copy holds the bytes
        assert!(!p.exists());
        let bak = t.path().join("library-overrides.json.bak-corrupt");
        assert_eq!(std::fs::read_to_string(bak).unwrap(), "{not json");
    }
}
