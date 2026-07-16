//! Library index: scans roots (folders of wrapper JSON), merges by content
//! hash with root precedence, applies per-user tag/favorite overrides,
//! serves filtered queries. Pure functions of paths — no Tauri types here
//! (IPC layer resolves paths), so everything unit-tests with tempdirs.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::entry::{LibraryEntryFile, LibraryInstrument};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryListEntry {
    pub hash: String,
    pub name: String,
    /// "fm" | "psg"
    pub kind: String,
    pub game: String,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub root_label: String,
}

#[derive(Debug, Clone)]
pub struct IndexedEntry {
    pub file: LibraryEntryFile,
    pub path: PathBuf,
    pub root_label: String,
    pub root_precedence: usize,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct OverrideEntry {
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub favorite: bool,
}

pub type Overrides = HashMap<String, OverrideEntry>;

#[derive(Debug, Clone, Serialize, Deserialize, Default, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryFilter {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub game: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub favorites_only: bool,
}

/// Scan one root. Tolerant: unreadable/invalid files are skipped with a
/// warning list (a bad file must never take the whole library down).
pub fn scan_root(root: &Path, label: &str, precedence: usize)
    -> (Vec<IndexedEntry>, Vec<String>)
{
    let mut out = Vec::new();
    let mut warnings = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "json")
                && !p.file_name().is_some_and(|n| {
                    let n = n.to_string_lossy();
                    // index-meta.json is only special at the root itself — an
                    // instrument legitimately named "Index Meta" inside fm/
                    // must not be invisible.
                    n.starts_with('_')
                        || (n == "index-meta.json" && p.parent() == Some(root))
                })
            {
                match fs::read_to_string(&p)
                    .map_err(|e| e.to_string())
                    .and_then(|s| serde_json::from_str::<LibraryEntryFile>(&s)
                        .map_err(|e| e.to_string()))
                {
                    Ok(file) => out.push(IndexedEntry {
                        file,
                        path: p,
                        root_label: label.to_string(),
                        root_precedence: precedence,
                    }),
                    Err(err) => warnings.push(format!("{}: {}", p.display(), err)),
                }
            }
        }
    }
    (out, warnings)
}

/// Merge scanned roots: same hash collapses to the LOWEST precedence
/// (bundled=0 wins display); provenance songs are unioned.
/// Only `songs` are unioned; the losing entry's name/tags/slot are dropped —
/// per-user customization must go through overrides, which key on hash and
/// survive merge.
pub fn merge(mut entries: Vec<IndexedEntry>) -> Vec<IndexedEntry> {
    entries.sort_by(|a, b| {
        a.root_precedence.cmp(&b.root_precedence)
            .then_with(|| a.path.cmp(&b.path))
    });
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut out: Vec<IndexedEntry> = Vec::new();
    for e in entries {
        let h = e.file.provenance.hash.clone();
        match seen.get(&h) {
            Some(&i) => {
                let songs = &mut out[i].file.provenance.songs;
                for s in &e.file.provenance.songs {
                    if !songs.contains(s) { songs.push(s.clone()); }
                }
            }
            None => { seen.insert(h, out.len()); out.push(e); }
        }
    }
    out
}

pub fn load_overrides(path: &Path) -> Overrides {
    fs::read_to_string(path).ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_overrides(path: &Path, o: &Overrides) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, serde_json::to_string_pretty(o).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

pub fn to_list_entry(e: &IndexedEntry, overrides: &Overrides) -> LibraryListEntry {
    let ov = overrides.get(&e.file.provenance.hash);
    LibraryListEntry {
        hash: e.file.provenance.hash.clone(),
        name: e.file.name.clone(),
        kind: match e.file.instrument {
            LibraryInstrument::Fm(_) => "fm".into(),
            LibraryInstrument::Psg(_) => "psg".into(),
        },
        game: e.file.provenance.game.clone(),
        tags: ov.and_then(|o| o.tags.clone()).unwrap_or_else(|| e.file.tags.clone()),
        favorite: ov.map(|o| o.favorite).unwrap_or(false),
        root_label: e.root_label.clone(),
    }
}

pub fn apply_filter(list: &[LibraryListEntry], f: &LibraryFilter) -> Vec<LibraryListEntry> {
    let text = f.text.as_deref().unwrap_or("").to_lowercase();
    list.iter()
        .filter(|e| f.kind.as_deref().is_none_or(|k| e.kind == k))
        .filter(|e| f.game.as_deref().is_none_or(|g| e.game == g))
        .filter(|e| f.tag.as_deref().is_none_or(|t| e.tags.iter().any(|x| x == t)))
        .filter(|e| !f.favorites_only || e.favorite)
        .filter(|e| {
            text.is_empty()
                || e.name.to_lowercase().contains(&text)
                || e.game.to_lowercase().contains(&text)
                || e.tags.iter().any(|t| t.to_lowercase().contains(&text))
        })
        .cloned()
        .collect()
}

/// Write one entry file (used by import-to-library, save-from-project, and
/// the extractor). Skips the write when an identical file exists, so re-runs
/// are idempotent UNDER STABLE NAMES — if the naming scheme ever changes, a
/// stale same-hash stray is left at the old name (merge hides it from the
/// index; cleaning it up is extractor territory). Returns the path written
/// (or existing).
pub fn write_entry(dir: &Path, file: &LibraryEntryFile) -> Result<PathBuf, String> {
    let sub = match file.instrument {
        LibraryInstrument::Fm(_) => "fm",
        LibraryInstrument::Psg(_) => "psg",
    };
    let d = dir.join(sub);
    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    // A name with no ASCII alphanumerics kebabs to "" — fall back to a
    // hash-derived base ("untitled-<8 hex>") so the file never becomes an
    // extensionless ".json" that scan_root skips forever. Hash-derived keeps
    // re-runs idempotent.
    let base = match kebab(&file.name) {
        b if b.is_empty() => format!("untitled-{}", &file.provenance.hash[7..15]),
        b => b,
    };
    let mut path = d.join(format!("{base}.json"));
    let body = serde_json::to_string_pretty(file).map_err(|e| e.to_string())? + "\n";
    let mut n = 1;
    loop {
        match fs::read_to_string(&path) {
            Ok(existing) if existing == body => return Ok(path), // identical: no-op
            Ok(existing) => {
                // Name collision with different content. Different hash →
                // suffix. Same hash (renamed re-extract) → overwrite in
                // place. Unparseable (truncated/corrupt after a crash
                // mid-write) → self-heal by overwriting in place rather than
                // forking a suffixed corpse.
                match serde_json::from_str::<LibraryEntryFile>(&existing) {
                    Ok(e) if e.provenance.hash != file.provenance.hash => {
                        n += 1;
                        path = d.join(format!("{base}-{n}.json"));
                    }
                    _ => break, // same hash or corrupt: overwrite in place
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
            Err(e) => return Err(format!("{}: {}", path.display(), e)),
        }
    }
    fs::write(&path, body).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn kebab(s: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::entry::{content_hash, Provenance, LIBRARY_SCHEMA};
    use crate::model::instrument::{FmInstrument, FmOperator, InstrumentMetadata};
    use uuid::Uuid;

    fn entry_named(name: &str, alg: u8, game: &str) -> LibraryEntryFile {
        let inst = FmInstrument {
            id: Uuid::nil(), name: name.into(), algorithm: alg, feedback: 0,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        };
        let li = LibraryInstrument::Fm(inst);
        let hash = content_hash(&li);
        LibraryEntryFile {
            schema: LIBRARY_SCHEMA, name: name.into(), tags: vec!["lead".into()],
            provenance: Provenance { game: game.into(), songs: vec!["S".into()], slot: None, hash },
            instrument: li,
        }
    }

    #[test]
    fn scan_merge_precedence_and_song_union() {
        let t = tempfile::tempdir().unwrap();
        let bundled = t.path().join("bundled");
        let user = t.path().join("user");
        // same patch (alg 1) in both roots, different song lists
        let mut a = entry_named("Same", 1, "G");
        a.provenance.songs = vec!["song-a".into()];
        let mut b = entry_named("Same", 1, "G");
        b.provenance.songs = vec!["song-b".into()];
        write_entry(&bundled, &a).unwrap();
        write_entry(&user, &b).unwrap();
        write_entry(&user, &entry_named("Only User", 2, "G")).unwrap();

        let (mut e1, w1) = scan_root(&bundled, "bundled", 0);
        let (e2, w2) = scan_root(&user, "user", 1);
        assert!(w1.is_empty() && w2.is_empty());
        e1.extend(e2);
        let merged = merge(e1);
        assert_eq!(merged.len(), 2);
        let same = merged.iter().find(|e| e.file.name == "Same").unwrap();
        assert_eq!(same.root_label, "bundled"); // precedence 0 wins
        assert!(same.file.provenance.songs.contains(&"song-a".to_string()));
        assert!(same.file.provenance.songs.contains(&"song-b".to_string()));
    }

    #[test]
    fn overrides_replace_tags_and_set_favorite() {
        let e = entry_named("X", 3, "G");
        let idx = IndexedEntry {
            path: PathBuf::from("x.json"), root_label: "r".into(),
            root_precedence: 0, file: e,
        };
        let mut ov: Overrides = HashMap::new();
        ov.insert(idx.file.provenance.hash.clone(), OverrideEntry {
            tags: Some(vec!["bass".into()]), favorite: true,
        });
        let le = to_list_entry(&idx, &ov);
        assert_eq!(le.tags, vec!["bass"]);
        assert!(le.favorite);
        let le2 = to_list_entry(&idx, &HashMap::new());
        assert_eq!(le2.tags, vec!["lead"]); // baseline from file
        assert!(!le2.favorite);
    }

    #[test]
    fn filter_by_text_kind_game_tag_favorite() {
        let mk = |name: &str, kind: &str, game: &str, tags: &[&str], fav: bool| LibraryListEntry {
            hash: name.into(), name: name.into(), kind: kind.into(), game: game.into(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            favorite: fav, root_label: "r".into(),
        };
        let all = vec![
            mk("EHZ Lead", "fm", "Sonic 2", &["lead"], true),
            mk("Bass 1", "fm", "Sonic 3K", &["bass"], false),
            mk("Env 3", "psg", "SMPS", &["staccato"], false),
        ];
        let f = |flt: LibraryFilter| apply_filter(&all, &flt).len();
        assert_eq!(f(LibraryFilter { text: Some("lead".into()), ..Default::default() }), 1);
        assert_eq!(f(LibraryFilter { kind: Some("psg".into()), ..Default::default() }), 1);
        assert_eq!(f(LibraryFilter { game: Some("Sonic 2".into()), ..Default::default() }), 1);
        assert_eq!(f(LibraryFilter { tag: Some("bass".into()), ..Default::default() }), 1);
        assert_eq!(f(LibraryFilter { favorites_only: true, ..Default::default() }), 1);
        assert_eq!(f(LibraryFilter::default()), 3);
    }

    #[test]
    fn write_entry_idempotent_and_collision_suffixed() {
        let t = tempfile::tempdir().unwrap();
        let e = entry_named("A Name", 1, "G");
        let p1 = write_entry(t.path(), &e).unwrap();
        let p2 = write_entry(t.path(), &e).unwrap();
        assert_eq!(p1, p2); // identical → no new file
        let other = entry_named("A Name", 2, "G"); // same name, different patch
        let p3 = write_entry(t.path(), &other).unwrap();
        assert_ne!(p1, p3);
        assert!(p3.to_string_lossy().contains("a-name-2"));
    }

    #[test]
    fn write_entry_untitled_fallback_for_empty_kebab() {
        let t = tempfile::tempdir().unwrap();
        let e = entry_named("!!!", 1, "G"); // kebabs to ""
        let p = write_entry(t.path(), &e).unwrap();
        assert!(p.to_string_lossy().contains("untitled-"));
        // idempotent under the fallback name too
        assert_eq!(write_entry(t.path(), &e).unwrap(), p);
        // and a rescan actually finds it (no invisible fm/.json)
        let (found, warns) = scan_root(t.path(), "r", 0);
        assert!(warns.is_empty());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].file.name, "!!!");
    }

    #[test]
    fn write_entry_same_hash_overwrites_in_place() {
        let t = tempfile::tempdir().unwrap();
        let mut e = entry_named("Voice", 1, "G");
        e.provenance.songs = vec!["song-a".into()];
        let p1 = write_entry(t.path(), &e).unwrap();
        // same patch (same hash), updated provenance → overwrite, no fork
        e.provenance.songs.push("song-b".into());
        let p2 = write_entry(t.path(), &e).unwrap();
        assert_eq!(p1, p2);
        let on_disk: LibraryEntryFile =
            serde_json::from_str(&fs::read_to_string(&p2).unwrap()).unwrap();
        assert_eq!(on_disk.provenance.songs, vec!["song-a", "song-b"]);
    }

    #[test]
    fn kebab_names() {
        assert_eq!(kebab("EHZ Lead #3!"), "ehz-lead-3");
    }
}
