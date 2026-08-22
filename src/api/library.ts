// Thin wrappers over the generated library bindings, matching `src/api/ipc.ts`:
// unwrap the generated `Result<T, E>` into "resolve with T / reject with the
// error payload". Types come straight from `src/bindings.ts` — Rust is the
// single source of truth.
import { commands } from "../bindings";
import type {
  LibraryEntryDetail,
  LibraryFilter,
  LibraryImportResult,
  LibraryListEntry,
  Result,
  RootInfo,
} from "../bindings";

export type {
  LibraryEntryDetail,
  LibraryFilter,
  LibraryImportResult,
  LibraryInstrument,
  LibraryListEntry,
  RootInfo,
} from "../bindings";

/**
 * DataTransfer MIME type for dragging a library entry (payload:
 * `JSON.stringify({ hash, kind })`). A kind-suffixed variant
 * (`${LIBRARY_DRAG_TYPE}-fm` / `-psg`) is also set so drop targets can show
 * a compatibility cue during dragover, when only `types` is readable.
 */
export const LIBRARY_DRAG_TYPE = "application/x-seraph-library-entry";

/** Unwrap a generated `Result<T, E>`: return `T` or throw the error payload. */
function unwrap<T, E>(res: Result<T, E>): T {
  if (res.status === "ok") return res.data;
  throw res.error;
}

export async function libraryList(filter: LibraryFilter): Promise<LibraryListEntry[]> {
  return unwrap(await commands.libraryList(filter));
}

export async function libraryGames(): Promise<string[]> {
  return unwrap(await commands.libraryGames());
}

export async function libraryGetEntry(hash: string): Promise<LibraryEntryDetail> {
  return unwrap(await commands.libraryGetEntry(hash));
}

export async function libraryRescan(): Promise<number> {
  return unwrap(await commands.libraryRescan());
}

export async function libraryWarnings(): Promise<string[]> {
  return unwrap(await commands.libraryWarnings());
}

export async function libraryAudition(hash: string, midiNote: number): Promise<void> {
  unwrap(await commands.libraryAudition(hash, midiNote));
}

/**
 * Stop a library audition, whatever the entry kind.
 *
 * Backed by the dedicated `library_stop_audition` command: it forces a fast
 * release on FM ch0 (imported patches can carry RR=0 and would ring on
 * KeyOff alone), keys off, and sends `StopPreview` — clearing a looping PSG
 * envelope preview and invalidating the sequencer's ch0 FM patch cache so
 * playback recovers on ch0's next note-on. Still do NOT swap in
 * `stop_all_sound` (Panic) — it is the global panic button and kills every
 * channel, not just the audition. (Panic no longer leaves FM dead: since
 * 2026-08-22 it invalidates the whole patch cache, which it must, because it
 * resets the chip. That was a real defect — the next note after a Panic
 * rendered silent; guarded by
 * `src-tauri/src/audio/live_edit_audibility.rs::a_note_after_panic_reprograms_its_patch`.)
 */
export async function libraryStopAudition(): Promise<void> {
  unwrap(await commands.libraryStopAudition());
}

export async function libraryAddToProject(hash: string): Promise<string> {
  return unwrap(await commands.libraryAddToProject(hash));
}

/**
 * Drag-to-track swap: bind a library voice to a track. Reuses a project
 * instrument with the same content hash, otherwise adds the voice first.
 * Returns the bound project instrument id. Kind-checked server-side.
 */
export async function libraryAssignToTrack(trackId: string, hash: string): Promise<string> {
  return unwrap(await commands.libraryAssignToTrack(trackId, hash));
}

/**
 * Resolve a library entry into the project's instrument bank WITHOUT touching
 * any track (reuse by content hash, else add). Returns the project instrument
 * id — the id `setNoteInstrument` wants when a library entry is dropped on
 * selected notes.
 */
export async function libraryEnsureProjectInstrument(hash: string): Promise<string> {
  return unwrap(await commands.libraryEnsureProjectInstrument(hash));
}

export async function librarySaveFromProject(
  kind: string,
  id: string,
  name: string | null,
  tags: string[],
): Promise<string> {
  return unwrap(await commands.librarySaveFromProject(kind, id, name, tags));
}

export async function librarySetTags(hash: string, tags: string[]): Promise<void> {
  unwrap(await commands.librarySetTags(hash, tags));
}

export async function librarySetFavorite(hash: string, favorite: boolean): Promise<void> {
  unwrap(await commands.librarySetFavorite(hash, favorite));
}

export async function libraryImportFiles(paths: string[]): Promise<LibraryImportResult> {
  return unwrap(await commands.libraryImportFiles(paths));
}

export async function libraryRootsGet(): Promise<RootInfo[]> {
  return unwrap(await commands.libraryRootsGet());
}

export async function libraryRootAdd(path: string): Promise<void> {
  unwrap(await commands.libraryRootAdd(path));
}

export async function libraryRootRemove(path: string): Promise<void> {
  unwrap(await commands.libraryRootRemove(path));
}
