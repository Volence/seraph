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
 * playback recovers on ch0's next note-on. Do NOT swap in `stop_all_sound`
 * (Panic): it resets both chips without invalidating that cache, killing FM
 * output until the next stop/seek; Panic remains the global panic button.
 */
export async function libraryStopAudition(): Promise<void> {
  unwrap(await commands.libraryStopAudition());
}

export async function libraryAddToProject(hash: string): Promise<string> {
  return unwrap(await commands.libraryAddToProject(hash));
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
