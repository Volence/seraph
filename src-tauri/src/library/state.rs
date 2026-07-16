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

/// Parse the custom-roots file body (a JSON array of directory paths).
fn parse_custom_roots(s: &str) -> Result<Vec<String>, String> {
    serde_json::from_str::<Vec<String>>(s).map_err(|e| e.to_string())
}

/// Load the custom-roots list WITHOUT silent data loss (same hazard as the
/// overrides file: silently yielding zero roots on a corrupt file would let
/// the next root add/remove rewrite — permanently clobber — the user's list).
/// Missing file is a normal first run; exists-but-unparseable is warned AND
/// quarantined to `library-roots.json.bak-corrupt` for hand recovery.
fn load_custom_roots_guarded(path: &std::path::Path, warnings: &mut Vec<String>) -> Vec<String> {
    let s = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(e) => {
            warnings.push(format!("could not read custom-roots file {}: {e}", path.display()));
            return Vec::new();
        }
    };
    match parse_custom_roots(&s) {
        Ok(customs) => customs,
        Err(e) => {
            let bak = path.with_file_name("library-roots.json.bak-corrupt");
            match std::fs::rename(path, &bak) {
                Ok(()) => warnings.push(format!(
                    "custom-roots file was corrupt ({e}); moved to {} — custom roots are gone until it is restored",
                    bak.display()
                )),
                Err(re) => warnings.push(format!(
                    "custom-roots file {} is corrupt ({e}) and could not be quarantined ({re}); the roots list may be lost on next save",
                    path.display()
                )),
            }
            Vec::new()
        }
    }
}

pub fn resolve_roots(app: &AppHandle, warnings: &mut Vec<String>) -> Vec<RootInfo> {
    let mut roots = Vec::new();
    if let Some(b) = bundled_root(app) {
        roots.push(RootInfo { label: "Seraph Pack".into(), path: b.to_string_lossy().into(), kind: "bundled".into() });
    }
    if let Ok(u) = user_root(app) {
        let _ = std::fs::create_dir_all(&u);
        roots.push(RootInfo { label: "My Library".into(), path: u.to_string_lossy().into(), kind: "user".into() });
    }
    if let Ok(p) = custom_roots_path(app) {
        for c in load_custom_roots_guarded(&p, warnings) {
            roots.push(RootInfo { label: c.clone(), path: c, kind: "custom".into() });
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

/// Load overrides WITHOUT silent data loss: an empty result for a corrupt
/// file would let the next save permanently erase the user's favorites/tags.
/// Distinguish missing (normal first run) from corrupt: warn AND quarantine
/// the corrupt file to `library-overrides.json.bak-corrupt` so a later save
/// can't clobber it and the user can recover by hand.
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
    let mut warns = Vec::new();
    let roots = resolve_roots(app, &mut warns);
    let mut all = Vec::new();
    for (i, r) in roots.iter().enumerate() {
        let (entries, w) = store::scan_root(std::path::Path::new(&r.path), &r.label, i);
        all.extend(entries);
        warns.extend(w);
    }
    let merged = store::merge(all);
    let overrides = overrides_path(app)
        .map(|p| load_overrides_guarded(&p, &mut warns))
        .unwrap_or_default();
    // Self-healing: rescan replaces every value wholesale, so recovering a
    // poisoned guard is safe — un-poison instead of propagating a panic.
    *state.index.lock().unwrap_or_else(|p| p.into_inner()) = merged;
    *state.overrides.lock().unwrap_or_else(|p| p.into_inner()) = overrides;
    *state.roots.lock().unwrap_or_else(|p| p.into_inner()) = roots;
    *state.warnings.lock().unwrap_or_else(|p| p.into_inner()) = warns;
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

    #[test]
    fn parse_custom_roots_accepts_array_rejects_junk() {
        assert_eq!(
            parse_custom_roots(r#"["/a/b", "/c"]"#).unwrap(),
            vec!["/a/b".to_string(), "/c".to_string()]
        );
        assert!(parse_custom_roots("{not json").is_err());
        assert!(parse_custom_roots(r#"{"roots": []}"#).is_err()); // wrong shape
    }

    #[test]
    fn guarded_roots_missing_file_is_silent() {
        let t = tempfile::tempdir().unwrap();
        let mut warns = Vec::new();
        let roots = load_custom_roots_guarded(&t.path().join("library-roots.json"), &mut warns);
        assert!(roots.is_empty());
        assert!(warns.is_empty());
    }

    #[test]
    fn guarded_roots_corrupt_file_warns_and_quarantines() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("library-roots.json");
        std::fs::write(&p, "[not json").unwrap();
        let mut warns = Vec::new();
        let roots = load_custom_roots_guarded(&p, &mut warns);
        assert!(roots.is_empty());
        assert_eq!(warns.len(), 1);
        assert!(warns[0].contains("corrupt"));
        // original is gone; quarantined copy holds the bytes
        assert!(!p.exists());
        let bak = t.path().join("library-roots.json.bak-corrupt");
        assert_eq!(std::fs::read_to_string(bak).unwrap(), "[not json");
    }
}
