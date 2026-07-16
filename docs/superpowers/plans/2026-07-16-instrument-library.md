# Instrument Library Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A cross-project instrument library: batch extraction from games (Sonic SMPS, AoBR, GYB packs, PSG table) into a repo-committed default pack, plus a full in-app browser (search/tags/favorites/audition) with import-to-library and save-from-project.

**Architecture:** Folder-of-files library (one JSON per instrument, wrapping the EXISTING `FmInstrument`/`PsgInstrument` serde types) + content-hash identity (sha256 over canonical patch bytes) + an in-memory index in a new `library` Rust module served over typed IPC. Extraction is a `cargo` bin reusing the existing `import::*` parsers. Spec: `../specs/2026-07-16-instrument-library-design.md` (APPROVED + research corrections committed).

**Tech Stack:** Rust (serde, sha2 new dep, tempfile for tests), tauri-specta (landed 2026-07-15 — every new command gets `#[specta::specta]` and a `collect_commands!` entry; `cargo test` regenerates `src/bindings.ts`), React 19 + CSS Modules, vitest + @testing-library/react (NEW — Task 1 adds the runner; none exists today).

**Repo:** `/home/volence/sonic_hacks/seraph`, branch `feat/instrument-library` off `main`.

**Verified facts the plan relies on (do not re-derive):**
- `FmInstrument` fields: `id: Uuid, name: String, algorithm: u8, feedback: u8, operators: [FmOperator; 4], metadata: InstrumentMetadata` (`src-tauri/src/model/instrument.rs:11-20`). `FmOperator` fields: `detune, multiple, rate_scale, attack_rate, amp_mod: bool, d1r, d2r, sustain_level, release_rate, total_level, ssg_eg` (all `u8` except `amp_mod`). `InstrumentMetadata { category: String, author: String, tags: Vec<String> }`.
- `PsgInstrument` fields: `id, name, volume_sequence: Vec<u8>, loop_point: Option<usize>, silence_on_end: bool, noise_mode: Option<NoiseMode>, smps_envelope_index: Option<u8>, metadata` (`instrument.rs:76-89`).
- Existing previews are stateless: `preview_fm_instrument` (`ipc/commands.rs:321-366`) fetches by project UUID then does register writes + `FmKeyOn` on channel 0; `preview_psg_instrument` (`:418-447`) sends `PsgEnvelopePreview`. Only the lookup is project-scoped — library audition refactors the body into a helper taking `&FmInstrument`/`&PsgInstrument`.
- SMPS: `parse_smps(source: &str) -> Result<SmpsFile, String>` (`import/smps_parser.rs:782`), `SmpsFile.voices: Vec<[u8; 25]>`; 25-byte voice → `FmInstrument` via the driver's `fm_from_bytes` (`driver/flamedriver.rs:149`); acquire the driver the same way `smps_mapper.rs:52-67` does (READ that file first).
- Zyrinx importer IS the AoBR parser: `parse_zyrinx_song(rom: &[u8], game_id: u8)` where `game_id` = song index 0..19 (`import/zyrinx_parser.rs:293-306`); `ZyrinxVoice → FmInstrument` mapping lives in `zyrinx_mapper.rs:40-85`. AoBR ROM on disk: `/home/volence/sonic_hacks/The Adventures of Batman and Robin/Adventures of Batman & Robin, The (USA).md`.
- GYB/TFI/VGI/Y12: `import_fm_file(data: &[u8], filename: &str) -> Result<FmFileImportResult, String>` (`import/fm_formats.rs:18`), `FmFileImportResult { instruments: Vec<FmInstrument>, format: String }`.
- PSG presets source: `FLAMEDRIVER_PSG_ENVELOPES` (52 entries) + `get_envelope(index)` in `import/psg_envelopes.rs`; attenuation→volume conversion mirrors `smps_mapper.rs` `resolve_psg_env` (`:319-382`) — READ it before writing the psg-table extractor.
- `lib.rs` modules are PRIVATE (`mod import;` etc., `lib.rs:1-11`) — the bin target needs `pub mod` for `import`, `model`, `driver`, and the new `library`. Lib name is `seraph_lib` (`Cargo.toml:10-15`).
- IPC registration: add command fn in `ipc/commands.rs` with `#[tauri::command] #[specta::specta]`, re-export in `ipc/mod.rs`, add to `collect_commands![...]` in `build_specta()` (`lib.rs:59-132`), run `cargo test` to regenerate `src/bindings.ts`.
- Frontend: no state library — props + `useState` + `src/api/ipc.ts` wrappers; CSS Modules with tokens (`--bg-panel`, `--border`, `--accent-fm`, `--accent-psg`, `--text-primary`...). Dialog pattern: `NewProjectDialog.tsx` (overlay z-300). `Sidebar.tsx`/`InstrumentBrowser.tsx` are DEAD CODE (nothing imports Sidebar) — do not build on them. Mount point: inside `<div className={styles.body}>` in `App.tsx` (~line 179), beside `<MainArea>`. Instrument editors render in `BottomPanel.tsx` (FmEditor/PsgEditor, `:80-83`); editors audition via `PianoKeys` (`FmEditor.tsx:97-102`).
- No virtualization anywhere; plain `.map()` is the codebase pattern. DEVIATION FROM SPEC (recorded): the list renders plain `.map()` capped at 400 rows + a "N more — refine your search" footer instead of a virtualized list. Matches codebase idiom; revisit only if the pack outgrows it.
- Commit discipline: exact paths only (never `-A`); NO Claude/Anthropic attribution or Co-Authored-By trailers.

---

### Task 1: Branch + frontend test infra (vitest + RTL)

**Files:**
- Modify: `package.json` (devDependencies + `test` script)
- Create: `vitest.config.ts`
- Create: `src/test/setup.ts`
- Create: `src/lib/formatTags.test.ts` + `src/lib/formatTags.ts` (smoke util proving the runner works; the util is used later by LibraryPanel)

- [ ] **Step 1: Create the branch**

```bash
cd /home/volence/sonic_hacks/seraph && git checkout main && git pull --ff-only && git checkout -b feat/instrument-library
```

- [ ] **Step 2: Install the test stack**

```bash
npm install -D vitest @testing-library/react @testing-library/jest-dom jsdom
```

- [ ] **Step 3: Create `vitest.config.ts`**

```ts
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
```

Create `src/test/setup.ts`:

```ts
import "@testing-library/jest-dom/vitest";
```

Add to `package.json` scripts: `"test": "vitest run"`.

- [ ] **Step 4: Write a failing smoke test**

`src/lib/formatTags.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { formatTags } from "./formatTags";

describe("formatTags", () => {
  it("joins and lowercases tags", () => {
    expect(formatTags(["Lead", "BRIGHT"])).toBe("lead, bright");
  });
  it("returns empty string for no tags", () => {
    expect(formatTags([])).toBe("");
  });
});
```

Run: `npm test` — Expected: FAIL (module `./formatTags` not found).

- [ ] **Step 5: Implement `src/lib/formatTags.ts`**

```ts
export function formatTags(tags: string[]): string {
  return tags.map((t) => t.toLowerCase()).join(", ");
}
```

Run: `npm test` — Expected: 2 passed.

- [ ] **Step 6: Verify the main build still passes and commit**

Run: `npm run build` — Expected: green.

```bash
git add package.json package-lock.json vitest.config.ts src/test/setup.ts src/lib/formatTags.ts src/lib/formatTags.test.ts
git commit -m "test: vitest + testing-library setup (first frontend test infra)"
```

---

### Task 2: Library entry format + content hashing (Rust)

**Files:**
- Modify: `src-tauri/Cargo.toml` (add `sha2 = "0.10"`)
- Create: `src-tauri/src/library/mod.rs`
- Create: `src-tauri/src/library/entry.rs`
- Modify: `src-tauri/src/lib.rs:1-11` (add `pub mod library;` and make `import`, `model`, `driver` pub — needed by Task 5's bin)

- [ ] **Step 1: Add the dependency**

In `src-tauri/Cargo.toml` under `[dependencies]`: `sha2 = "0.10"`.

- [ ] **Step 2: Write failing tests for the wrapper + hash**

Create `src-tauri/src/library/entry.rs` with the tests FIRST (module skeleton so it compiles enough to fail meaningfully — or write tests referencing not-yet-written fns and let the compile failure be the "red"):

```rust
//! Library entry file format: a thin metadata wrapper around the existing
//! instrument serde types, plus content-hash identity.
//! See docs/superpowers/specs/2026-07-16-instrument-library-design.md.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::instrument::{FmInstrument, NoiseMode, PsgInstrument};

pub const LIBRARY_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    pub game: String,
    /// Every song the (deduped) voice appears in.
    #[serde(default)]
    pub songs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u8>,
    /// Content hash — the entry's identity across roots and re-extractions.
    pub hash: String,
}

/// `{"type":"fm","instrument":{...}}` shape per the spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "instrument", rename_all = "lowercase")]
pub enum LibraryInstrument {
    Fm(FmInstrument),
    Psg(PsgInstrument),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntryFile {
    pub schema: u32,
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub provenance: Provenance,
    #[serde(flatten)]
    pub instrument: LibraryInstrument,
}

/// Canonical byte string for an FM patch: fields only, fixed order, no JSON,
/// no floats — identical sound == identical bytes == identical hash.
pub fn fm_canonical_bytes(inst: &FmInstrument) -> Vec<u8> {
    let mut b = vec![inst.algorithm, inst.feedback];
    for op in &inst.operators {
        b.extend_from_slice(&[
            op.detune, op.multiple, op.rate_scale, op.attack_rate,
            op.amp_mod as u8, op.d1r, op.d2r, op.sustain_level,
            op.release_rate, op.total_level, op.ssg_eg,
        ]);
    }
    b
}

/// Canonical bytes for a PSG preset. `smps_envelope_index` is EXCLUDED
/// (provenance, not sound); `noise_mode` is included (it is sound).
pub fn psg_canonical_bytes(inst: &PsgInstrument) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&inst.volume_sequence);
    b.push(0xFE); // field separator (volumes are 0..15, never 0xFE)
    match inst.loop_point {
        Some(lp) => b.extend_from_slice(&(lp as u64).to_le_bytes()),
        None => b.extend_from_slice(&u64::MAX.to_le_bytes()),
    }
    b.push(inst.silence_on_end as u8);
    match &inst.noise_mode {
        None => b.push(0),
        Some(NoiseMode::Periodic(p)) => { b.push(1); b.extend_from_slice(&p.to_le_bytes()); }
        Some(NoiseMode::White(p)) => { b.push(2); b.extend_from_slice(&p.to_le_bytes()); }
    }
    b
}

pub fn content_hash(instrument: &LibraryInstrument) -> String {
    let bytes = match instrument {
        LibraryInstrument::Fm(i) => fm_canonical_bytes(i),
        LibraryInstrument::Psg(i) => psg_canonical_bytes(i),
    };
    format!("sha256:{:x}", Sha256::digest(&bytes))
}
```

Tests at the bottom of `entry.rs` (write these before running):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::instrument::{FmOperator, InstrumentMetadata};
    use uuid::Uuid;

    fn sample_fm() -> FmInstrument {
        FmInstrument {
            id: Uuid::nil(),
            name: "Test".into(),
            algorithm: 4,
            feedback: 5,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        }
    }

    #[test]
    fn wrapper_json_shape_matches_spec() {
        let entry = LibraryEntryFile {
            schema: LIBRARY_SCHEMA,
            name: "EHZ Lead".into(),
            tags: vec!["lead".into()],
            provenance: Provenance {
                game: "Sonic 2".into(),
                songs: vec!["EHZ".into()],
                slot: Some(3),
                hash: "sha256:abc".into(),
            },
            instrument: LibraryInstrument::Fm(sample_fm()),
        };
        let v: serde_json::Value = serde_json::to_value(&entry).unwrap();
        assert_eq!(v["type"], "fm");
        assert!(v["instrument"]["algorithm"].is_number());
        assert_eq!(v["schema"], 1);
        assert_eq!(v["provenance"]["game"], "Sonic 2");
        // round-trip
        let back: LibraryEntryFile = serde_json::from_value(v).unwrap();
        assert_eq!(back.name, "EHZ Lead");
    }

    #[test]
    fn fm_hash_ignores_name_and_id_but_not_patch() {
        let a = sample_fm();
        let mut b = sample_fm();
        b.name = "Different".into();
        b.id = Uuid::new_v4();
        assert_eq!(
            content_hash(&LibraryInstrument::Fm(a.clone())),
            content_hash(&LibraryInstrument::Fm(b.clone()))
        );
        b.algorithm = 7;
        assert_ne!(
            content_hash(&LibraryInstrument::Fm(a)),
            content_hash(&LibraryInstrument::Fm(b))
        );
    }

    #[test]
    fn psg_hash_excludes_envelope_index() {
        let mk = |idx: Option<u8>| PsgInstrument {
            id: Uuid::nil(),
            name: "e".into(),
            volume_sequence: vec![15, 12, 8, 4, 0],
            loop_point: Some(2),
            silence_on_end: true,
            noise_mode: None,
            smps_envelope_index: idx,
            metadata: InstrumentMetadata::default(),
        };
        assert_eq!(
            content_hash(&LibraryInstrument::Psg(mk(Some(3)))),
            content_hash(&LibraryInstrument::Psg(mk(None)))
        );
    }
}
```

Create `src-tauri/src/library/mod.rs`:

```rust
pub mod entry;
```

In `src-tauri/src/lib.rs`, change the module block (lines 1-11) so these are public (Task 5's bin target imports them through `seraph_lib::`):

```rust
mod audio;
mod dac;
pub mod driver;
mod export;
pub mod import;
mod ipc;
pub mod library;
pub mod model;
mod project;
mod sequencer;
mod sn76489;
mod ym2612;
```

(If `cargo check` then errors on now-public items exposing private types, make the minimum additional items `pub` — do NOT restructure.)

- [ ] **Step 3: Run the tests**

Run: `cd src-tauri && cargo test library::entry -q`
Expected: 3 passed. Also run the FULL suite once (`cargo test -q`) — the `pub mod` changes must not break anything (180+ tests stay green; bindings test regenerates with zero diff since no commands changed).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/library/mod.rs src-tauri/src/library/entry.rs src-tauri/src/lib.rs
git commit -m "feat(library): entry file format + sha256 content-hash identity"
```

---

### Task 3: Root scanning, merged index, overrides, filtering

**Files:**
- Create: `src-tauri/src/library/store.rs`
- Modify: `src-tauri/src/library/mod.rs` (`pub mod store;`)

- [ ] **Step 1: Write the store with tests-first mindset**

`src-tauri/src/library/store.rs` — complete implementation:

```rust
//! Library index: scans roots (folders of wrapper JSON), merges by content
//! hash with root precedence, applies per-user tag/favorite overrides,
//! serves filtered queries. Pure functions of paths — no Tauri types here
//! (IPC layer resolves paths), so everything unit-tests with tempdirs.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::entry::{content_hash, LibraryEntryFile, LibraryInstrument};

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
                    n.starts_with('_') || n == "index-meta.json"
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
/// the extractor). Skips the write when an identical file exists (idempotent
/// re-runs → zero diff). Returns the path written (or existing).
pub fn write_entry(dir: &Path, file: &LibraryEntryFile) -> Result<PathBuf, String> {
    let sub = match file.instrument {
        LibraryInstrument::Fm(_) => "fm",
        LibraryInstrument::Psg(_) => "psg",
    };
    let d = dir.join(sub);
    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let base = kebab(&file.name);
    let mut path = d.join(format!("{base}.json"));
    let body = serde_json::to_string_pretty(file).map_err(|e| e.to_string())? + "\n";
    let mut n = 1;
    loop {
        match fs::read_to_string(&path) {
            Ok(existing) if existing == body => return Ok(path), // identical: no-op
            Ok(existing) => {
                // Name collision with different content — but if it's the SAME
                // hash (renamed re-extract), overwrite; else suffix.
                let same_hash = serde_json::from_str::<LibraryEntryFile>(&existing)
                    .map(|e| e.provenance.hash == file.provenance.hash)
                    .unwrap_or(false);
                if same_hash { break; }
                n += 1;
                path = d.join(format!("{base}-{n}.json"));
            }
            Err(_) => break,
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
```

Append `#[cfg(test)] mod tests` in the same file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::entry::{Provenance, LIBRARY_SCHEMA};
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
    fn kebab_names() {
        assert_eq!(kebab("EHZ Lead #3!"), "ehz-lead-3");
    }
}
```

Add `pub mod store;` to `src-tauri/src/library/mod.rs`.

- [ ] **Step 2: Run**

Run: `cd src-tauri && cargo test library::store -q` — Expected: 5 passed.
Note: `is_none_or` needs Rust ≥1.82 (installed: 1.96) — fine.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/library/store.rs src-tauri/src/library/mod.rs
git commit -m "feat(library): root scanning, hash-merged index, overrides, filtering"
```

---

### Task 4: IPC commands, managed state, bindings

**Files:**
- Create: `src-tauri/src/library/state.rs` (roots resolution + LibraryState)
- Modify: `src-tauri/src/ipc/commands.rs` (new commands + preview-helper refactor)
- Modify: `src-tauri/src/ipc/mod.rs` (re-exports)
- Modify: `src-tauri/src/lib.rs` (`collect_commands!` + `.manage(...)` + startup scan)
- Modify: `src-tauri/tauri.conf.json` (bundle resources)
- Generated: `src/bindings.ts` (via `cargo test`)

- [ ] **Step 1: Refactor the existing previews into instrument-taking helpers (behavior-preserving)**

In `src-tauri/src/ipc/commands.rs`: READ `preview_fm_instrument` (:321-366) and `preview_psg_instrument` (:418-447) first. Extract their bodies into private helpers in the same file:

```rust
fn do_preview_fm(audio_state: &AudioState, inst: &FmInstrument, midi_note: u8) -> Result<(), String> {
    // moved body of preview_fm_instrument AFTER the project lookup —
    // register writes + midi_to_fm_freq + FmKeyOn { channel: 0, operators: 0xF0 }
}

fn do_preview_psg(audio_state: &AudioState, inst: &PsgInstrument, midi_note: u8) -> Result<(), String> {
    // moved body of preview_psg_instrument after the lookup —
    // midi_to_psg_period + PsgEnvelopePreview on channel 0
}
```

`preview_fm_instrument` / `preview_psg_instrument` become: project lookup → `do_preview_*`. Run `cargo test -q` — all green (no behavior change).

- [ ] **Step 2: Roots resolution + state**

`src-tauri/src/library/state.rs`:

```rust
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
    let overrides = overrides_path(app).map(|p| store::load_overrides(&p)).unwrap_or_default();
    *state.index.lock().unwrap() = merged;
    *state.overrides.lock().unwrap() = overrides;
    *state.roots.lock().unwrap() = roots;
    *state.warnings.lock().unwrap() = warns;
}
```

Add `pub mod state;` to `library/mod.rs`.

- [ ] **Step 3: The commands**

Append to `src-tauri/src/ipc/commands.rs` (match the file's existing imports/patterns; `LibraryState` etc. imported from `crate::library`):

```rust
use crate::library::entry::{content_hash, LibraryEntryFile, LibraryInstrument, Provenance, LIBRARY_SCHEMA};
use crate::library::state::{self, LibraryState, RootInfo};
use crate::library::store::{self, LibraryFilter, LibraryListEntry};

#[tauri::command]
#[specta::specta]
pub fn library_list(
    lib: State<'_, LibraryState>,
    filter: LibraryFilter,
) -> Result<Vec<LibraryListEntry>, String> {
    let idx = lib.index.lock().unwrap();
    let ov = lib.overrides.lock().unwrap();
    let all: Vec<LibraryListEntry> = idx.iter().map(|e| store::to_list_entry(e, &ov)).collect();
    Ok(store::apply_filter(&all, &filter))
}

#[tauri::command]
#[specta::specta]
pub fn library_games(lib: State<'_, LibraryState>) -> Result<Vec<String>, String> {
    let idx = lib.index.lock().unwrap();
    let mut games: Vec<String> = idx.iter().map(|e| e.file.provenance.game.clone()).collect();
    games.sort();
    games.dedup();
    Ok(games)
}

#[tauri::command]
#[specta::specta]
pub fn library_rescan(app: tauri::AppHandle, lib: State<'_, LibraryState>) -> Result<u32, String> {
    state::rescan(&app, &lib);
    Ok(lib.index.lock().unwrap().len() as u32)
}

#[tauri::command]
#[specta::specta]
pub fn library_audition(
    audio_state: State<'_, AudioState>,
    lib: State<'_, LibraryState>,
    hash: String,
    midi_note: u8,
) -> Result<(), String> {
    let idx = lib.index.lock().unwrap();
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
    state_proj: State<'_, ProjectState>,
    lib: State<'_, LibraryState>,
    hash: String,
) -> Result<String, String> {
    let inst = {
        let idx = lib.index.lock().unwrap();
        idx.iter().find(|e| e.file.provenance.hash == hash)
            .map(|e| e.file.instrument.clone())
            .ok_or("library entry not found")?
    };
    // Reuse the existing add paths (they assign fresh UUIDs + mark dirty) —
    // mirror how add_fm_instrument / add_psg_instrument acquire the manager.
    match inst {
        LibraryInstrument::Fm(i) => { /* mgr.add_fm_instrument(i) -> Ok(id) */ }
        LibraryInstrument::Psg(i) => { /* mgr.add_psg_instrument(i) -> Ok(id) */ }
    }
}

#[tauri::command]
#[specta::specta]
pub fn library_save_from_project(
    app: tauri::AppHandle,
    state_proj: State<'_, ProjectState>,
    lib: State<'_, LibraryState>,
    kind: String,
    id: String,
    name: Option<String>,
    tags: Vec<String>,
) -> Result<String, String> {
    // 1. fetch the instrument from ProjectManager by kind+id (mirror
    //    update_fm_instrument's lookup)
    // 2. set inst.id = Uuid::nil() (library files carry nil ids — determinism,
    //    and hash-dedup in write_entry needs identical bytes), then wrap:
    //    LibraryEntryFile { schema: LIBRARY_SCHEMA, name: name.unwrap_or(inst.name),
    //    tags, provenance: Provenance { game: "User".into(), songs: vec![],
    //    slot: None, hash: content_hash(&instrument) }, instrument }
    // 3. store::write_entry(&state::user_root(&app)?, &file)?
    // 4. state::rescan(&app, &lib);
    // return the hash
}

#[tauri::command]
#[specta::specta]
pub fn library_import_files(
    app: tauri::AppHandle,
    lib: State<'_, LibraryState>,
    paths: Vec<String>,
) -> Result<u32, String> {
    let root = state::user_root(&app)?;
    let mut written = 0u32;
    for p in &paths {
        let data = std::fs::read(p).map_err(|e| format!("{p}: {e}"))?;
        let fname = std::path::Path::new(p).file_name()
            .map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let res = crate::import::fm_formats::import_fm_file(&data, &fname)?;
        let game = format!("Imported: {}", res.format);
        for mut inst in res.instruments {
            inst.id = uuid::Uuid::nil(); // library files carry nil ids (determinism)
            let name = inst.name.clone();
            let li = LibraryInstrument::Fm(inst);
            let hash = content_hash(&li);
            let file = LibraryEntryFile {
                schema: LIBRARY_SCHEMA, name, tags: vec![],
                provenance: Provenance { game: game.clone(), songs: vec![], slot: None, hash },
                instrument: li,
            };
            store::write_entry(&root, &file)?;
            written += 1;
        }
    }
    state::rescan(&app, &lib);
    Ok(written)
}

#[tauri::command]
#[specta::specta]
pub fn library_set_tags(
    app: tauri::AppHandle, lib: State<'_, LibraryState>,
    hash: String, tags: Vec<String>,
) -> Result<(), String> {
    let mut ov = lib.overrides.lock().unwrap();
    ov.entry(hash).or_default().tags = Some(tags);
    store::save_overrides(&state::overrides_path(&app)?, &ov)
}

#[tauri::command]
#[specta::specta]
pub fn library_set_favorite(
    app: tauri::AppHandle, lib: State<'_, LibraryState>,
    hash: String, favorite: bool,
) -> Result<(), String> {
    let mut ov = lib.overrides.lock().unwrap();
    ov.entry(hash).or_default().favorite = favorite;
    store::save_overrides(&state::overrides_path(&app)?, &ov)
}

#[tauri::command]
#[specta::specta]
pub fn library_roots_get(lib: State<'_, LibraryState>) -> Result<Vec<RootInfo>, String> {
    Ok(lib.roots.lock().unwrap().clone())
}

#[tauri::command]
#[specta::specta]
pub fn library_root_add(
    app: tauri::AppHandle, lib: State<'_, LibraryState>, path: String,
) -> Result<(), String> {
    if !std::path::Path::new(&path).is_dir() { return Err("not a directory".into()); }
    {
        let mut roots = lib.roots.lock().unwrap();
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
        let mut roots = lib.roots.lock().unwrap();
        roots.retain(|r| !(r.kind == "custom" && r.path == path));
        state::save_custom_roots(&app, &roots)?;
    }
    state::rescan(&app, &lib);
    Ok(())
}
```

The two commented bodies (`library_add_to_project`, `library_save_from_project`) are NOT placeholders to skip — implement them by mirroring the adjacent `add_fm_instrument`/`update_fm_instrument` manager-acquisition code, which the executor MUST read first (`ipc/commands.rs:276-312, 380-416`). `import_fm_file` (the module fn, not the same-named IPC command) may need a `pub` bump on `crate::import::fm_formats` — do it.

- [ ] **Step 4: Register + manage + resource bundle**

- `ipc/mod.rs`: re-export the 12 new commands alongside the existing ones.
- `lib.rs` `build_specta()`: add all 12 to `collect_commands![]`.
- `lib.rs` `run()`: `.manage(crate::library::state::LibraryState::default())` next to the existing `.manage(...)` calls, and in the `.setup(...)` hook (add one if absent — read how `run()` builds the app): `crate::library::state::rescan(&app.handle(), &app.state::<LibraryState>());`
- `tauri.conf.json` `bundle` section: add `"resources": { "../library/": "library/" }` (Tauri v2 map form). The repo `library/` dir doesn't exist until Task 6 — create a placeholder now so builds don't break: `library/index-meta.json` = `{ "schema": 1, "name": "Seraph Pack" }`.

- [ ] **Step 5: Regenerate bindings + full gates**

```bash
cd src-tauri && cargo test -q     # regenerates ../src/bindings.ts; all tests green
cd .. && npm run build            # bindings compile; frontend green
git diff --stat src/bindings.ts   # sanity: new library* commands present
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/library/state.rs src-tauri/src/library/mod.rs src-tauri/src/ipc/commands.rs src-tauri/src/ipc/mod.rs src-tauri/src/lib.rs src-tauri/tauri.conf.json library/index-meta.json src/bindings.ts
git commit -m "feat(library): IPC surface — list/filter/audition/add/save/import/tags/favorites/roots"
```

---

### Task 5: Extractor CLI

**Files:**
- Create: `src-tauri/src/library/extract.rs` (all logic, unit-tested)
- Create: `src-tauri/src/bin/extract_library.rs` (thin arg-parsing shell)
- Modify: `src-tauri/src/library/mod.rs` (`pub mod extract;`)
- Possibly modify: `src-tauri/src/import/zyrinx_mapper.rs` (make the ZyrinxVoice→FmInstrument conversion a `pub fn` if it's inline today — read `zyrinx_mapper.rs:40-85` first)

- [ ] **Step 1: Write `extract.rs` with a fixture-driven test**

```rust
//! Batch extraction: game data -> library entry files.
//! Reuses the import parsers verbatim; dedups by content hash; deterministic
//! output (idempotent re-runs produce zero diff).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::import::{fm_formats, psg_envelopes, smps_parser};
use crate::library::entry::{content_hash, LibraryEntryFile, LibraryInstrument, Provenance, LIBRARY_SCHEMA};
use crate::library::store::write_entry;
use crate::model::instrument::{FmInstrument, InstrumentMetadata, PsgInstrument};

pub struct ExtractStats { pub songs: u32, pub voices_seen: u32, pub unique_written: u32 }

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
            let mut inst = fm_voice_to_instrument(voice)?; // see note below
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

/// 25-byte SMPS voice -> FmInstrument.
/// IMPLEMENTATION NOTE (executor): acquire the conversion the same way
/// smps_mapper.rs:52-67 does (driver's fm_from_bytes, flamedriver.rs:149).
/// READ smps_mapper.rs first and mirror its driver acquisition exactly.
fn fm_voice_to_instrument(voice: &[u8; 25]) -> Result<FmInstrument, String> {
    // e.g. let driver = <however smps_mapper builds it>; driver.fm_from_bytes(voice)
    unimplemented!("transcribe from smps_mapper.rs:52-67")
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
    // For game_id in 0..20: parse_zyrinx_song(&rom, game_id); convert each
    // ZyrinxVoice -> FmInstrument via the zyrinx_mapper conversion
    // (zyrinx_mapper.rs:40-85 — expose as pub fn if currently inline).
    // Dedup + provenance.songs from GAME_SONG_NAMES, same shape as
    // extract_smps_dir. Voice naming: "{song-name} voice {index:02}".
    unimplemented!("mirror extract_smps_dir using zyrinx_parser + zyrinx_mapper")
}

/// The 52 bundled Flamedriver PSG envelopes -> presets (generated once).
pub fn extract_psg_table(out_dir: &Path) -> Result<ExtractStats, String> {
    write_game_meta(out_dir, "SMPS PSG")?;
    let mut stats = ExtractStats { songs: 0, voices_seen: 0, unique_written: 0 };
    for idx in 0..psg_envelopes::FLAMEDRIVER_PSG_ENVELOPES.len() {
        // Convert with the SAME attenuation->volume rule as
        // smps_mapper::resolve_psg_env (READ smps_mapper.rs:319-382; volume =
        // 15 - atten, clamped). Build PsgInstrument { volume_sequence, loop_point,
        // silence_on_end, noise_mode: None, smps_envelope_index: Some(idx as u8 + 1), .. }
        // name: format!("smps env {:02X}", idx + 1)
        // NOTE (review correction): the song-facing envelope index is 1-BASED —
        // verified against the importer (resolve_psg_env does `env_idx.saturating_sub(1)`
        // for the table lookup, mirroring the Z80 `dec a`, and stores Some(env_idx)
        // un-decremented) and the exporter (export/smps.rs emits the field verbatim
        // as `sTone_{:02X}`). Table position idx = song index idx + 1.
        unimplemented!("transcribe conversion from resolve_psg_env")
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
```

The `unimplemented!` markers are transcription tasks with exact source locations, not open design: fill each by reading the named function and reusing/exposing it. Do NOT copy-paste logic if a `pub` bump lets you call it directly — prefer calling the existing code.

Test (same file, `#[cfg(test)]`): build a fixture SMPS asm with TWO songs sharing one voice — write the fixture inline as a `&str` using the exact macro dialect from the plan-verified OOZ.asm excerpt (`smpsHeaderVoice`, `smpsVcAlgorithm $01`, `smpsVcFeedback $07`, ..., `smpsVcTotalLevel $80, $97, $2C, $23` — copy a full real voice block from `/home/volence/sonic_hacks/s2disasm/sound/music/84 - OOZ.asm:287-304` and a minimal valid song around it; get the minimal song scaffold by reading what `parse_smps` requires + how `test_smps_import`-style tests in `import/mod.rs` build fixtures). Assertions:

```rust
#[test]
fn smps_extraction_dedups_and_unions_provenance() {
    let t = tempfile::tempdir().unwrap();
    let in_dir = t.path().join("in");
    std::fs::create_dir_all(&in_dir).unwrap();
    std::fs::write(in_dir.join("song-a.asm"), FIXTURE_SONG_A).unwrap();
    std::fs::write(in_dir.join("song-b.asm"), FIXTURE_SONG_B).unwrap(); // same voice
    let out = t.path().join("out");
    let stats = extract_smps_dir(&in_dir, "TestGame", &out).unwrap();
    assert_eq!(stats.songs, 2);
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
}
```

(`dir_snapshot`: helper mapping relative path → file contents.)

- [ ] **Step 2: The bin**

`src-tauri/src/bin/extract_library.rs`:

```rust
//! Batch library extraction CLI. Run from src-tauri/:
//!   cargo run --bin extract_library -- smps   --in <dir>  --game "Sonic 2" --out <dir>
//!   cargo run --bin extract_library -- gyb    --in <file> --game "<pack>"  --out <dir>
//!   cargo run --bin extract_library -- zyrinx --rom <file> --game "Batman & Robin" --out <dir>
//!   cargo run --bin extract_library -- psg-table --out <dir>

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::exit;

use seraph_lib::library::extract;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else { usage() };
    let opts: HashMap<String, String> = args[1..]
        .chunks(2)
        .filter_map(|c| match c {
            [k, v] if k.starts_with("--") => Some((k[2..].to_string(), v.clone())),
            _ => None,
        })
        .collect();
    let get = |k: &str| -> String {
        opts.get(k).cloned().unwrap_or_else(|| { eprintln!("missing --{k}"); exit(2) })
    };
    let out = PathBuf::from(get("out"));
    let res = match cmd.as_str() {
        "smps" => extract::extract_smps_dir(&PathBuf::from(get("in")), &get("game"), &out),
        "gyb" => extract::extract_gyb(&PathBuf::from(get("in")), &get("game"), &out),
        "zyrinx" => extract::extract_zyrinx(&PathBuf::from(get("rom")), &get("game"), &out),
        "psg-table" => extract::extract_psg_table(&out),
        _ => usage(),
    };
    match res {
        Ok(s) => println!("songs={} voices_seen={} unique_written={}", s.songs, s.voices_seen, s.unique_written),
        Err(e) => { eprintln!("error: {e}"); exit(1) }
    }
}

fn usage() -> ! {
    eprintln!("usage: extract_library <smps|gyb|zyrinx|psg-table> [--in PATH] [--rom PATH] [--game NAME] --out DIR");
    exit(2)
}
```

- [ ] **Step 3: Run tests + build the bin**

```bash
cd src-tauri && cargo test library::extract -q && cargo build --bin extract_library
```

Expected: extraction test passes; bin compiles.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/library/extract.rs src-tauri/src/library/mod.rs src-tauri/src/bin/extract_library.rs
git add src-tauri/src/import/zyrinx_mapper.rs   # only if the pub bump was needed
git commit -m "feat(library): extraction CLI — smps/gyb/zyrinx/psg-table, hash-deduped, idempotent"
```

---

### Task 6: Build the default pack (content run)

**Files:**
- Create (generated, committed): `library/**` (the default pack)

- [ ] **Step 1: Locate the skdisasm music dir**

Run: `grep -rl smpsVcAlgorithm /home/volence/sonic_hacks/skdisasm --include='*.asm' -l | head -3` and note the directory (expected something like `.../Sound/Music/`). s2disasm's is `/home/volence/sonic_hacks/s2disasm/sound/music/`. If skdisasm uses a different macro dialect and `parse_smps` rejects its files wholesale (0 songs parsed), record it, skip skdisasm, and log the deferral in the queue-doc step — do NOT hack the parser in this task.

- [ ] **Step 2: Run the extractions**

```bash
cd /home/volence/sonic_hacks/seraph/src-tauri
cargo run --bin extract_library -- smps --in "/home/volence/sonic_hacks/s2disasm/sound/music" --game "Sonic 2" --out ../library/sonic2
cargo run --bin extract_library -- smps --in "<skdisasm music dir>" --game "Sonic 3 & Knuckles" --out ../library/sonic3k
cargo run --bin extract_library -- zyrinx --rom "/home/volence/sonic_hacks/The Adventures of Batman and Robin/Adventures of Batman & Robin, The (USA).md" --game "Batman & Robin" --out ../library/batman-robin
cargo run --bin extract_library -- psg-table --out ../library/smps-psg
```

Expected: each prints stats with `unique_written > 0` (Sonic 2 should yield dozens of unique voices). Idempotency spot check: re-run the sonic2 line; `git status ../library` shows no changes.

- [ ] **Step 3: Spot-verify quality in the app (manual)**

`npm run tauri dev` — the LibraryPanel doesn't exist yet, so verify via a temporary check instead: `library_list` isn't reachable without UI; acceptable to defer audition-by-ear to Task 7 Step 5. At minimum verify one JSON by eye: an entry from `library/sonic2/fm/` has plausible operator values (attack_rate 0-31, total_level 0-127) and non-empty `provenance.songs`.

- [ ] **Step 4: Commit the pack**

```bash
cd /home/volence/sonic_hacks/seraph
git add library/
git commit -m "feat(library): default pack — Sonic 2, Sonic 3&K, Batman & Robin, SMPS PSG presets"
```

---

### Task 7: LibraryPanel UI + audition

**Files:**
- Create: `src/components/LibraryPanel.tsx`, `src/components/LibraryPanel.module.css`
- Create: `src/components/LibraryRootsDialog.tsx`, `src/components/LibraryRootsDialog.module.css`
- Create: `src/api/library.ts` (wrappers over the generated bindings, matching `src/api/ipc.ts` style)
- Create: `src/components/LibraryPanel.test.tsx`
- Modify: `src/App.tsx` (~line 179: render inside `styles.body` before `<MainArea>`)
- Modify: `src/App.module.css` (body already flex; panel supplies its own width)

- [ ] **Step 1: API wrappers**

`src/api/library.ts` — mirror the unwrap pattern in `src/api/ipc.ts` (read it first):

```ts
import { commands, type LibraryFilter, type LibraryListEntry, type RootInfo } from "../bindings";

// use the same `unwrap` helper style as src/api/ipc.ts (import or duplicate its 5 lines)

export async function libraryList(filter: LibraryFilter): Promise<LibraryListEntry[]> { /* unwrap(await commands.libraryList(filter)) */ }
export async function libraryGames(): Promise<string[]> { /* ... */ }
export async function libraryRescan(): Promise<number> { /* ... */ }
export async function libraryAudition(hash: string, midiNote: number): Promise<void> { /* ... */ }
export async function libraryStopAudition(): Promise<void> { /* see note below */ }
// STOP-AUDITION NOTE: verify what stops each preview type — read how
// PsgEditor's PianoKeys onNoteOff stops PSG preview vs FmEditor's
// stopFmPreview (AudioCommand::StopPreview may cover both). Wire this wrapper
// to whatever combination actually silences BOTH fm and psg previews; if PSG
// needs a separate command, call both.
export async function libraryAddToProject(hash: string): Promise<string> { /* ... */ }
export async function librarySetTags(hash: string, tags: string[]): Promise<void> { /* ... */ }
export async function librarySetFavorite(hash: string, favorite: boolean): Promise<void> { /* ... */ }
export async function libraryImportFiles(paths: string[]): Promise<number> { /* ... */ }
export async function libraryRootsGet(): Promise<RootInfo[]> { /* ... */ }
export async function libraryRootAdd(path: string): Promise<void> { /* ... */ }
export async function libraryRootRemove(path: string): Promise<void> { /* ... */ }
```

(Fill every body with the standard one-line unwrap; the generated camelCase names come from `src/bindings.ts` after Task 4.)

- [ ] **Step 2: The panel**

`src/components/LibraryPanel.tsx` — self-contained collapsible side panel (no TopBar changes): collapsed = a 28px vertical strip with a "Lib" label button; expanded = 280px panel. Complete component:

```tsx
import { useCallback, useEffect, useState } from "react";
import type { LibraryListEntry } from "../bindings";
import * as lib from "../api/library";
import { formatTags } from "../lib/formatTags";
import { LibraryRootsDialog } from "./LibraryRootsDialog";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
// ^ tauri-plugin-dialog is registered on the Rust side (Cargo.toml). If
// @tauri-apps/plugin-dialog is missing from package.json (check how the
// existing ImportDialog picks files — reuse ITS mechanism if different),
// `npm install @tauri-apps/plugin-dialog` and commit the lockfile.
import styles from "./LibraryPanel.module.css";

const RENDER_CAP = 400; // plain .map per codebase idiom; search narrows results

interface LibraryPanelProps {
  /** Bump to refresh (e.g. after save-from-project). */
  refreshToken?: number;
  onInstrumentAdded: () => void;
}

export function LibraryPanel({ refreshToken, onInstrumentAdded }: LibraryPanelProps) {
  const [open, setOpen] = useState(true);
  const [entries, setEntries] = useState<LibraryListEntry[]>([]);
  const [games, setGames] = useState<string[]>([]);
  const [text, setText] = useState("");
  const [kind, setKind] = useState<"all" | "fm" | "psg">("all");
  const [game, setGame] = useState<string>("all");
  const [favOnly, setFavOnly] = useState(false);
  const [editingTags, setEditingTags] = useState<string | null>(null); // hash
  const [tagDraft, setTagDraft] = useState("");
  const [rootsOpen, setRootsOpen] = useState(false);

  const refresh = useCallback(async () => {
    const filter = {
      text: text || null,
      kind: kind === "all" ? null : kind,
      game: game === "all" ? null : game,
      tag: null,
      favoritesOnly: favOnly,
    };
    setEntries(await lib.libraryList(filter));
    setGames(await lib.libraryGames());
  }, [text, kind, game, favOnly]);

  useEffect(() => { refresh(); }, [refresh, refreshToken]);

  async function handleImport() {
    const picked = await openFileDialog({
      multiple: true,
      filters: [{ name: "FM instruments", extensions: ["tfi", "vgi", "y12", "gyb"] }],
    });
    if (!picked) return;
    const paths = Array.isArray(picked) ? picked : [picked];
    await lib.libraryImportFiles(paths as string[]);
    await refresh();
  }

  async function saveTags(hash: string) {
    await lib.librarySetTags(hash, tagDraft.split(",").map((t) => t.trim()).filter(Boolean));
    setEditingTags(null);
    await refresh();
  }

  if (!open) {
    return (
      <button className={styles.rail} onClick={() => setOpen(true)} title="Open library">
        Lib
      </button>
    );
  }

  const shown = entries.slice(0, RENDER_CAP);
  return (
    <div className={styles.panel}>
      <div className={styles.header}>
        <span className={styles.title}>Library</span>
        <button onClick={() => setRootsOpen(true)} title="Library folders">⚙</button>
        <button onClick={handleImport} title="Import instrument files">Import</button>
        <button onClick={() => setOpen(false)} title="Collapse">«</button>
      </div>
      <input
        className={styles.search}
        placeholder="Search name, game, tag…"
        value={text}
        onChange={(e) => setText(e.target.value)}
      />
      <div className={styles.filters}>
        {(["all", "fm", "psg"] as const).map((k) => (
          <button key={k} className={kind === k ? styles.chipActive : styles.chip} onClick={() => setKind(k)}>
            {k.toUpperCase()}
          </button>
        ))}
        <button className={favOnly ? styles.chipActive : styles.chip} onClick={() => setFavOnly(!favOnly)}>★</button>
        <select className={styles.gameSelect} value={game} onChange={(e) => setGame(e.target.value)}>
          <option value="all">All games</option>
          {games.map((g) => <option key={g} value={g}>{g}</option>)}
        </select>
      </div>
      <div className={styles.list}>
        {shown.map((e) => (
          <div key={e.hash} className={styles.item}>
            <span
              className={`${styles.dot} ${e.kind === "fm" ? styles.fmDot : styles.psgDot}`}
            />
            <span
              className={styles.itemName}
              title={`${e.game} — audition (hold)`}
              onMouseDown={() => lib.libraryAudition(e.hash, 60)}
              onMouseUp={() => lib.libraryStopAudition()}
              onMouseLeave={() => lib.libraryStopAudition()}
            >
              {e.name}
            </span>
            <span className={styles.itemTags} onDoubleClick={() => { setEditingTags(e.hash); setTagDraft(e.tags.join(", ")); }}>
              {formatTags(e.tags)}
            </span>
            <button
              className={e.favorite ? styles.starOn : styles.star}
              onClick={async () => { await lib.librarySetFavorite(e.hash, !e.favorite); refresh(); }}
            >★</button>
            <button
              className={styles.addBtn}
              title="Add to project"
              onClick={async () => { await lib.libraryAddToProject(e.hash); onInstrumentAdded(); }}
            >+</button>
            {editingTags === e.hash && (
              <input
                className={styles.tagInput}
                autoFocus
                value={tagDraft}
                onChange={(ev) => setTagDraft(ev.target.value)}
                onKeyDown={(ev) => { if (ev.key === "Enter") saveTags(e.hash); if (ev.key === "Escape") setEditingTags(null); }}
                onBlur={() => saveTags(e.hash)}
              />
            )}
          </div>
        ))}
        {entries.length > RENDER_CAP && (
          <div className={styles.moreNote}>{entries.length - RENDER_CAP} more — refine your search</div>
        )}
      </div>
      {rootsOpen && <LibraryRootsDialog onClose={async () => { setRootsOpen(false); await refresh(); }} />}
    </div>
  );
}
```

`LibraryPanel.module.css` — tokens only (`--bg-panel`, `--border`, `--text-primary`, `--text-secondary`, `--accent`, `--accent-fm`, `--accent-psg`, `--bg-input`); panel `width: 280px; border-right: 1px solid var(--border); display: flex; flex-direction: column; background: var(--bg-panel);`, `.list { flex: 1; overflow-y: auto; }`, `.rail { width: 28px; writing-mode: vertical-rl; ... }`, item rows `display:flex; gap:6px; align-items:center; padding: 3px 8px; font-size: var(--fs-sm);`. Match `InstrumentBrowser.module.css`'s dot/name row styling (read it for reference even though the component is unused).

- [ ] **Step 3: Roots dialog**

`src/components/LibraryRootsDialog.tsx` — copy the `NewProjectDialog.tsx` overlay/dialog skeleton (read it first): title "Library folders", list of roots from `libraryRootsGet()` showing label + kind badge, a Remove button on `kind === "custom"` rows, an "Add folder…" button calling `openFileDialog({ directory: true })` then `libraryRootAdd(path)`, and a Close button. Same overlay z-index/CSS pattern as NewProjectDialog.

- [ ] **Step 4: Mount + component test**

In `src/App.tsx`, inside `<div className={styles.body}>` (line ~179), BEFORE `<MainArea ...>`, when a project is open (match how `BottomPanel` gates on `projectOpen`):

```tsx
{projectOpen && (
  <LibraryPanel refreshToken={libraryRefresh} onInstrumentAdded={handleInstrumentsChanged} />
)}
```

`handleInstrumentsChanged` = whatever callback App already uses to refresh instrument state after changes (find the one NewProjectDialog/import flows call; if none exists, a no-op is acceptable for v1 — the BottomPanel lists refresh on selection).

`src/components/LibraryPanel.test.tsx` — mock the API module, assert filter round-trip and render cap:

```tsx
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { LibraryPanel } from "./LibraryPanel";
import * as lib from "../api/library";

vi.mock("../api/library");
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

const entry = (name: string, kind: string) => ({
  hash: name, name, kind, game: "Sonic 2", tags: ["lead"], favorite: false, rootLabel: "Seraph Pack",
});

describe("LibraryPanel", () => {
  beforeEach(() => {
    vi.mocked(lib.libraryGames).mockResolvedValue(["Sonic 2"]);
    vi.mocked(lib.libraryList).mockResolvedValue([entry("EHZ Lead", "fm"), entry("Env 3", "psg")] as never);
  });

  it("renders entries and re-queries with kind filter", async () => {
    render(<LibraryPanel onInstrumentAdded={() => {}} />);
    await screen.findByText("EHZ Lead");
    fireEvent.click(screen.getByText("PSG"));
    await waitFor(() =>
      expect(vi.mocked(lib.libraryList)).toHaveBeenLastCalledWith(
        expect.objectContaining({ kind: "psg" })
      )
    );
  });

  it("passes search text to the backend filter", async () => {
    render(<LibraryPanel onInstrumentAdded={() => {}} />);
    await screen.findByText("EHZ Lead");
    fireEvent.change(screen.getByPlaceholderText(/Search/), { target: { value: "bass" } });
    await waitFor(() =>
      expect(vi.mocked(lib.libraryList)).toHaveBeenLastCalledWith(
        expect.objectContaining({ text: "bass" })
      )
    );
  });
});
```

Run: `npm test` — Expected: all pass. Then `npm run build` — green.

- [ ] **Step 5: Manual audition smoke (user-facing check)**

`npm run tauri dev`: open/create a project → panel shows the pack → hold-click a Sonic 2 FM entry (sound plays, stops on release) → click a PSG preset (envelope plays) → star one (persists across restart) → double-click tags, edit, Enter (persists) → "+" adds to project (appears in the project's instrument list) → search narrows. Fix what fails.

- [ ] **Step 6: Commit**

```bash
git add src/components/LibraryPanel.tsx src/components/LibraryPanel.module.css src/components/LibraryRootsDialog.tsx src/components/LibraryRootsDialog.module.css src/api/library.ts src/components/LibraryPanel.test.tsx src/App.tsx
git commit -m "feat(library): browser panel — search/filter/tags/favorites/audition/add + roots dialog"
```

---

### Task 8: Save-from-project buttons

**Files:**
- Modify: `src/components/FmEditor.tsx` (button near the PianoKeys preview section, `:97-102`)
- Modify: `src/components/PsgEditor.tsx` (same placement pattern)
- Modify: `src/api/library.ts` (add `librarySaveFromProject` wrapper)
- Modify: `src/App.tsx` (thread a `bumpLibraryRefresh` callback down to BottomPanel → editors, so the panel refreshes after save — mirror how `selectedInstrument` flows down)

- [ ] **Step 1: Wrapper + buttons**

`src/api/library.ts`:

```ts
export async function librarySaveFromProject(
  kind: "fm" | "psg", id: string, name: string | null, tags: string[],
): Promise<string> { /* unwrap(await commands.librarySaveFromProject(kind, id, name, tags)) */ }
```

In `FmEditor.tsx`, next to the preview section:

```tsx
<button
  className={styles.saveToLibrary}
  title="Save this instrument to My Library"
  onClick={async () => {
    await librarySaveFromProject("fm", instrumentId, null, []);
    onSavedToLibrary?.();
  }}
>
  Save to library
</button>
```

(Add `onSavedToLibrary?: () => void` to the editor props; PsgEditor identical with `"psg"`. Style the button with existing token classes — match neighboring buttons in the editor's module.css.)

- [ ] **Step 2: Wire the refresh + test the loop manually**

App holds `const [libraryRefresh, setLibraryRefresh] = useState(0)`; passes `refreshToken={libraryRefresh}` to LibraryPanel (already in Task 7's mount) and `onSavedToLibrary={() => setLibraryRefresh((n) => n + 1)}` down through BottomPanel to both editors.

Manual check (`npm run tauri dev`): import a GYB or edit an FM patch → "Save to library" → entry appears in the panel under game "User" without restart. Saving the SAME patch twice → still one entry (hash dedup via `write_entry`).

- [ ] **Step 3: Gates + commit**

```bash
npm test && npm run build && (cd src-tauri && cargo test -q)
git add src/components/FmEditor.tsx src/components/PsgEditor.tsx src/api/library.ts src/App.tsx src/components/BottomPanel.tsx
git commit -m "feat(library): save-from-project round-trip (editors -> user library)"
```

---

### Task 9: Closeout

- [ ] **Step 1: Full gates on the branch**

```bash
cd /home/volence/sonic_hacks/seraph
npm test && npm run build
cd src-tauri && cargo test -q && cd ..
```

Expected: everything green; `src/bindings.ts` regeneration produces zero diff.

- [ ] **Step 2: Merge to main**

```bash
git checkout main && git merge --no-ff feat/instrument-library -m "merge: instrument library (default pack + browser + extraction CLI)"
git branch -d feat/instrument-library
```

Re-run the three gates on merged main before pushing. Push: `git push origin main`.

- [ ] **Step 3: Record in the queue doc**

Append a Log line to `docs/superpowers/2026-07-03-seraph-banking-queue.md`: date, "instrument library shipped (independent of the parked S-queue)", pack contents (games + counts from Task 6 stats), and any deferrals (e.g. skdisasm dialect skip if it happened; Sub-Terrania/Red Zone follow-up stands). Commit:

```bash
git add docs/superpowers/2026-07-03-seraph-banking-queue.md
git commit -m "docs: queue — instrument library shipped (log + deferrals)"
git push origin main
```

---

## Self-review notes (spec coverage)

- Storage/format/portability (spec §Storage) → Tasks 2/3 (wrapper, hash, roots-merge) + Task 4 (bundled/user/custom roots, Tauri resource).
- Default pack (spec §Decisions) → Tasks 5/6; idempotency tested (T5) and spot-checked (T6).
- Full browser: search/tags/favorites/audition (spec §UI/§App core) → Tasks 4 (IPC) + 7 (panel). Virtualization deviation recorded in header + spec-conform cap note.
- Audition (spec, research-resolved) → Task 4 Step 1 helper refactor + `library_audition`.
- Save/load round-trip (spec §Decisions, user amendment) → Task 8 (+ `library_add_to_project` in Task 4).
- Import-to-library (spec §App core) → Task 4 `library_import_files` + Task 7 Import button. NOTE: v1 in-app import covers FM FILES (TFI/VGI/Y12/GYB) only; SMPS/VGM/Zyrinx *songs* enter through the existing project-import flow and reach the library via save-from-project — matches the spec's flows; full song→library import is not separately specified.
- AoBR in v1 (corrected spec) → Tasks 5/6 zyrinx path.
- PSG-from-table (corrected spec) → Task 5 `extract_psg_table` + Task 6.
- Testing (spec §Testing) → per-task TDD; vitest infra Task 1; fixture goldens Task 5; manual audition passes Tasks 6/7/8.
- Out of scope guarded: no DAC, no SQLite, no cloud, no auto-tagging anywhere in tasks.
