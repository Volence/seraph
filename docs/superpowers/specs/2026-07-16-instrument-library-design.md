# Instrument Library — Design

**Date:** 2026-07-16
**Status:** APPROVED (user, 2026-07-16, this session)
**Context:** Independent of the banked S-queue (no Memra manifest dependency —
executable while the aeon sound format stabilizes). Builds a cross-project
instrument library: extraction from games + a full-featured browser in the app.

## Goal

Ship Seraph with a **default instrument pack** extracted from Genesis games
(Sonic 1/2/3&K first), browsable in-app with search/tags/favorites/audition,
growable by users (their own instruments, community packs, extra folders).
FM + PSG in v1; DAC deferred.

## Decisions (pinned during brainstorm)

- **Sources:** Sonic 1/2/3&K SMPS (s2disasm/skdisasm in workspace), other SMPS
  games (content sourced over time), Zyrinx games, community GYB/TFI packs.
  **The Adventures of Batman & Robin is OUT of v1** — Kyd's post-Zyrinx driver
  needs its own reverse-engineered parser; banked as a named follow-up.
- **Depth:** full browser — search, tag filters, favorites, audition. Backed by
  a folder-of-files + in-memory index, NOT a database (YAGNI; hundreds of
  instruments, not millions).
- **Types:** FM instruments + PSG envelope presets. DAC samples deferred to v2.
- **Feeding:** repo-committed batch extraction CLI (reproducible, reviewed
  output) + in-app import-to-library.
- **Default pack:** everything we extract is committed to the seraph repo and
  ships inside release builds as the bundled read-only pack. Users layer more
  on top.
- **Load/save round-trip:** any instrument in the open project — hand-made or
  created by a song import (SMPS/VGM/Zyrinx) — can be saved back into the
  library.

## Storage & format

Library root = plain directory tree; portable by construction (clone/zip/sync
a folder, point the app at it).

```
<library-root>/
  index-meta.json            # { schema: 1, name: "<pack name>" }
  sonic2/
    _game.json               # display name, source ("s2disasm"), extraction provenance
    fm/ehz-lead.json         # one FM instrument per file
    psg/env-03-staccato.json # one PSG envelope preset per file
  ...
```

Instrument file = thin metadata wrapper around the EXISTING serde types
(`FmInstrument` / `PsgInstrument` verbatim — no new instrument representation):

```json
{
  "schema": 1,
  "name": "EHZ Lead",
  "type": "fm",
  "tags": ["lead", "bright"],
  "provenance": { "game": "Sonic 2", "song": "EHZ", "slot": 3, "hash": "sha1:..." },
  "instrument": { }
}
```

`provenance.hash` = content hash of the raw patch bytes (canonical: the packed
register block for FM; the envelope byte string for PSG). It is the entry's
identity: dedup key for extraction, idempotency key for re-runs, and the key
user overrides attach to.

### Roots (portability model)

1. **Bundled pack** — `library/` committed at seraph repo root; shipped in
   release builds as a Tauri resource. Read-only at runtime.
2. **User library** — writable folder in the platform app-data dir; in-app
   imports and save-from-project land here.
3. **Custom roots** — arbitrary folders added/removed in settings (downloaded
   packs, synced drives).

All roots merge into one index, in fixed precedence order: bundled pack →
user library → custom roots in configured order. Same-hash entries across
roots collapse (earliest root wins for display; provenance lists all). **Personal tags + favorites live
in a per-user overrides file keyed by content hash** (survives file
moves/renames); baseline tags ship inside the instrument files. The bundled
pack is never mutated by user state.

## App core (`src-tauri/src/library/`, new module)

Startup + on-demand rescan: walk every root, parse wrappers, build the
in-memory index `hash → { meta, root, path, type, tags, favorite }`. Search =
case-insensitive substring over name/game/tags served from the index.

IPC (typed via the tauri-specta codegen landed 2026-07-15):

- `library_list(filter)` — text, type, tags, favorites-only, game → entries
- `library_add_to_project(id)` — copy instrument into the open project via the
  existing instrument-added flow
- `library_save_from_project(instrumentId, meta)` — write a project instrument
  into the user root (metadata prompt: name/tags; provenance auto-filled from
  the import source when known). Covers the "imported a song, keep its voices"
  case, since song imports create project instruments.
- `library_set_tags(id, tags)` / `library_set_favorite(id, bool)` — user
  overrides file
- `library_import_files(paths)` — run existing importers (TFI/VGI/Y12/GYB via
  `import_fm_file`; SMPS; Zyrinx), dedup by hash, write into the user root
- `library_audition(id, note, on|off)` — preview slot (below)
- `library_roots_get` / `library_roots_set` — manage custom roots

**Audition:** load the instrument into a reserved preview slot (FM or PSG
channel matching type) WITHOUT touching the project; key on mouse-down,
release on mouse-up. Prefer reusing the instrument editors' existing
live-preview path; if that path only plays project-owned instruments, add a
scratch-instrument holder on the engine side (small, bounded addition — the
plan's research step must settle which).

## UI (`src/components/LibraryPanel.tsx` + CSS module)

Dockable panel, existing component patterns: search box; filter chips (type,
game, tags, ★ favorites); virtualized result list (name/game/tags/★); click =
audition default note, double-click/button = add to project; inline tag editor
+ favorite toggle; Import button; roots management affordance. Selected entry
shows a compact detail card (FM: algorithm/feedback/op summary; PSG: envelope
sparkline). Project-side: a "Save to library" action on instruments.

## Extraction pipeline

Rust CLI in the existing crate — `src-tauri/src/bin/extract_library.rs` —
sharing `import::*` parsers as a library (zero duplicated parsing logic).
Manual runs, output committed:

```
extract_library smps   --in <path>/sound/music --game "Sonic 2" --out library/sonic2/
extract_library gyb    --in pack.gyb --game "<pack>" --out library/community-<pack>/
extract_library zyrinx --in <data> --game "<game>" --out library/<game>/
```

Per run: parse every song, collect FM voices + PSG envelope tables, dedup by
content hash (one file per unique voice; provenance lists every song using
it), auto-name `"<song>-voice-<slot>"` (curation = humans renaming/tagging the
JSON afterward — file-per-instrument exists for this). **Idempotent:** re-run
over unchanged input → zero git diff.

### Source matrix

| Source | Parser | Content availability | v1? |
|---|---|---|---|
| Sonic 1/2/3&K | SMPS-ASM (exists) | disassemblies in workspace | YES — seed of the pack |
| Other SMPS games | same | ROM/disasm sourcing is ongoing manual work | pack grows over time |
| Zyrinx (Sub-Terrania, Red Zone) | exists | user-provided data | YES |
| Community GYB/TFI | exists | community packs | YES (import + curate) |
| Adventures of Batman & Robin | DOES NOT EXIST | — | NO — named follow-up |

## Testing

- Wrapper serde round-trip; index merge across roots; overrides keyed by hash
  survive file moves; dedup + idempotency (extractor twice over fixture →
  identical output).
- Extraction fidelity goldens: fixture SMPS song with known voices → exact
  expected JSON.
- UI: vitest/RTL for filter logic + add-to-project; audition smoke manual.
- Gates: `cargo test` + `npm run build` green; commits with exact paths.

## Out of scope (v1)

DAC samples; AoBR driver parsing; SQLite/FTS; cloud sync/sharing service;
in-app pack downloading (users add folders manually); automatic tag inference.

## Follow-ups banked

1. **AoBR driver parser** (named follow-up — user explicitly wants these
   instruments eventually).
2. DAC drum-kit extraction (v2 of the library).
3. DAW polish/bugfix round — separate brainstorm; backlog to be captured from
   user experience + a smoke-test pass.
