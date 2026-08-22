// Thin wrappers over the tauri-specta generated bindings (`src/bindings.ts`).
//
// The generated `commands.*` return a `Result<T, E>` discriminated union.
// These wrappers unwrap that into the historical "resolve with T / reject with
// the error payload" contract the rest of the app relies on, so call sites did
// not have to change. Types come straight from the generated bindings — Rust is
// the single source of truth.
import { commands } from "../bindings";
import type {
  FmInstrument,
  PsgInstrument,
  DacInstrument,
  Song,
  SongMetadata,
  DriverInfo,
  DriverDetail,
  Track,
  ChannelAssignment,
  Pan,
  PlaybackState,
  OverlapWarning,
  UndoState,
  Result,
  ExportResult,
  ImportResult,
  FmFileImportResponse,
} from "../bindings";

// Re-export the generated response types that call sites reference via `ipc.*`.
export type {
  ExportResult,
  ExportError,
  ExportFailure,
  ImportResult,
  ImportWarning,
  FmFileImportResponse,
  UndoState,
} from "../bindings";

/** Unwrap a generated `Result<T, E>`: return `T` or throw the error payload. */
function unwrap<T, E>(res: Result<T, E>): T {
  if (res.status === "ok") return res.data;
  throw res.error;
}

export async function playFmTestTone(): Promise<string> {
  return unwrap(await commands.playFmTestTone());
}

export async function playPsgTestTone(): Promise<string> {
  return unwrap(await commands.playPsgTestTone());
}

export async function stopAllSound(): Promise<string> {
  return unwrap(await commands.stopAllSound());
}

export async function createProject(
  path: string,
  name: string,
  driverId: string,
  tempo: number,
  timeSigNum: number,
  timeSigDen: number,
): Promise<void> {
  unwrap(await commands.createProject(path, name, driverId, tempo, timeSigNum, timeSigDen));
}

export async function openProject(path: string): Promise<Song> {
  return unwrap(await commands.openProject(path));
}

export async function saveProject(): Promise<void> {
  unwrap(await commands.saveProject());
}

export async function closeProject(): Promise<void> {
  unwrap(await commands.closeProject());
}

export async function getProjectInfo(): Promise<SongMetadata | null> {
  return unwrap(await commands.getProjectInfo());
}

/** Edit tempo / time signature after creation. Returns the updated
 *  metadata. Marks dirty; NOT undoable in v1 (metadata is outside the
 *  track undo snapshot). */
export async function updateProjectMetadata(
  tempo: number,
  timeSigNum: number,
  timeSigDen: number,
): Promise<SongMetadata> {
  return unwrap(await commands.updateProjectMetadata(tempo, timeSigNum, timeSigDen));
}

export async function listDrivers(): Promise<DriverInfo[]> {
  return unwrap(await commands.listDrivers());
}

export async function getDriverInfo(driverId: string): Promise<DriverDetail> {
  return unwrap(await commands.getDriverInfo(driverId));
}

export async function addFmInstrument(instrument: FmInstrument): Promise<string> {
  return unwrap(await commands.addFmInstrument(instrument));
}

export async function updateFmInstrument(id: string, instrument: FmInstrument): Promise<void> {
  unwrap(await commands.updateFmInstrument(id, instrument));
}

export async function deleteFmInstrument(id: string): Promise<void> {
  unwrap(await commands.deleteFmInstrument(id));
}

export async function listFmInstruments(): Promise<FmInstrument[]> {
  return unwrap(await commands.listFmInstruments());
}

export async function previewFmInstrument(id: string, midiNote: number): Promise<void> {
  unwrap(await commands.previewFmInstrument(id, midiNote));
}

export async function stopFmPreview(): Promise<void> {
  unwrap(await commands.stopFmPreview());
}

export async function addPsgInstrument(instrument: PsgInstrument): Promise<string> {
  return unwrap(await commands.addPsgInstrument(instrument));
}

export async function updatePsgInstrument(id: string, instrument: PsgInstrument): Promise<void> {
  unwrap(await commands.updatePsgInstrument(id, instrument));
}

export async function deletePsgInstrument(id: string): Promise<void> {
  unwrap(await commands.deletePsgInstrument(id));
}

export async function listPsgInstruments(): Promise<PsgInstrument[]> {
  return unwrap(await commands.listPsgInstruments());
}

export async function previewPsgInstrument(id: string, midiNote: number): Promise<void> {
  unwrap(await commands.previewPsgInstrument(id, midiNote));
}

export async function importDacWav(wavPath: string, targetRate: number): Promise<string> {
  return unwrap(await commands.importDacWav(wavPath, targetRate));
}

export async function importDacRaw(pcmPath: string, sampleRate: number): Promise<string> {
  return unwrap(await commands.importDacRaw(pcmPath, sampleRate));
}

export async function updateDacInstrument(id: string, instrument: DacInstrument): Promise<void> {
  unwrap(await commands.updateDacInstrument(id, instrument));
}

export async function reconvertDac(id: string, newRate: number): Promise<void> {
  unwrap(await commands.reconvertDac(id, newRate));
}

export async function deleteDacInstrument(id: string): Promise<void> {
  unwrap(await commands.deleteDacInstrument(id));
}

export async function listDacInstruments(): Promise<DacInstrument[]> {
  return unwrap(await commands.listDacInstruments());
}

export async function previewDac(id: string): Promise<void> {
  unwrap(await commands.previewDac(id));
}

export async function getDacPcmData(id: string): Promise<number[]> {
  return unwrap(await commands.getDacPcmData(id));
}

// --- Track CRUD ---

export async function addTrack(
  name: string,
  channel: ChannelAssignment,
  instrumentId: string | null,
): Promise<string> {
  return unwrap(await commands.addTrack(name, channel, instrumentId));
}

export async function updateTrack(
  id: string,
  name: string,
  channel: ChannelAssignment,
  instrumentId: string | null,
  muted: boolean,
  solo: boolean,
  volume: number,
  pan: Pan,
  pitchOffset?: number,
): Promise<void> {
  unwrap(await commands.updateTrack(id, name, channel, instrumentId, muted, solo, volume, pan, pitchOffset ?? 0));
}

export async function deleteTrack(id: string): Promise<void> {
  unwrap(await commands.deleteTrack(id));
}

export async function listTracks(): Promise<Track[]> {
  return unwrap(await commands.listTracks());
}

// --- Region CRUD ---

export async function addRegion(
  trackId: string,
  startTick: number,
  durationTicks: number,
): Promise<string> {
  return unwrap(await commands.addRegion(trackId, startTick, durationTicks));
}

export async function updateRegion(
  trackId: string,
  regionId: string,
  startTick: number,
  durationTicks: number,
): Promise<void> {
  unwrap(await commands.updateRegion(trackId, regionId, startTick, durationTicks));
}

export async function moveRegion(
  srcTrackId: string,
  regionId: string,
  dstTrackId: string,
  startTick: number,
): Promise<void> {
  unwrap(await commands.moveRegion(srcTrackId, regionId, dstTrackId, startTick));
}

/** Deep-clone a region on its own track at the given start tick; resolves
 *  with the new region's id. One undoable edit server-side. */
export async function duplicateRegion(
  trackId: string,
  regionId: string,
  atStartTick: number,
): Promise<string> {
  return unwrap(await commands.duplicateRegion(trackId, regionId, atStartTick));
}

export async function deleteRegion(trackId: string, regionId: string): Promise<void> {
  unwrap(await commands.deleteRegion(trackId, regionId));
}

// --- Note CRUD ---

export async function addNote(
  trackId: string,
  regionId: string,
  tick: number,
  pitch: number,
  velocity: number,
  durationTicks: number,
): Promise<number> {
  return unwrap(await commands.addNote(trackId, regionId, tick, pitch, velocity, durationTicks));
}

export async function updateNote(
  trackId: string,
  regionId: string,
  noteIndex: number,
  tick: number,
  pitch: number,
  velocity: number,
  durationTicks: number,
): Promise<void> {
  unwrap(await commands.updateNote(trackId, regionId, noteIndex, tick, pitch, velocity, durationTicks));
}

export async function deleteNote(
  trackId: string,
  regionId: string,
  noteIndex: number,
): Promise<void> {
  unwrap(await commands.deleteNote(trackId, regionId, noteIndex));
}

// --- Undo / Redo ---

/** Undo the last song edit; resolves with the restored tracks. */
export async function undo(): Promise<Track[]> {
  return unwrap(await commands.undo());
}

export async function redo(): Promise<Track[]> {
  return unwrap(await commands.redo());
}

/** Open a coalescing undo group (drag gesture / batch loop). */
export async function beginUndoGroup(): Promise<void> {
  unwrap(await commands.beginUndoGroup());
}

export async function endUndoGroup(): Promise<void> {
  unwrap(await commands.endUndoGroup());
}

export async function getUndoState(): Promise<UndoState> {
  return unwrap(await commands.getUndoState());
}

// --- Transport ---

export async function transportPlay(): Promise<void> {
  unwrap(await commands.transportPlay());
}

export async function transportStop(): Promise<void> {
  unwrap(await commands.transportStop());
}

export async function transportSeek(tick: number): Promise<void> {
  unwrap(await commands.transportSeek(tick));
}

export async function transportSetLoop(startTick: number, endTick: number): Promise<void> {
  unwrap(await commands.transportSetLoop(startTick, endTick));
}

export async function transportClearLoop(): Promise<void> {
  unwrap(await commands.transportClearLoop());
}

export async function reloadSequence(): Promise<void> {
  unwrap(await commands.reloadSequence());
}

export async function setMasterVolume(volume: number): Promise<void> {
  unwrap(await commands.setMasterVolume(volume));
}

export async function getPlaybackState(): Promise<PlaybackState> {
  return unwrap(await commands.getPlaybackState());
}

// --- Validation ---

export async function getChannelOverlaps(): Promise<OverlapWarning[]> {
  return unwrap(await commands.getChannelOverlaps());
}

// --- Export ---

export async function exportSong(outputDir: string): Promise<ExportResult> {
  return unwrap(await commands.exportSong(outputDir));
}

// --- Import ---

export async function importSong(sourcePath: string, parentDir: string, dacDir?: string): Promise<ImportResult> {
  return unwrap(await commands.importSong(sourcePath, parentDir, dacDir ?? null));
}

export async function importZyrinxSong(romPath: string, parentDir: string, gameId: number): Promise<ImportResult> {
  return unwrap(await commands.importZyrinxSong(romPath, parentDir, gameId));
}

export async function importVgm(vgmPath: string, parentDir: string): Promise<ImportResult> {
  return unwrap(await commands.importVgm(vgmPath, parentDir));
}

// --- WAV Export ---

export async function exportWav(outputPath: string, durationSeconds: number): Promise<string> {
  return unwrap(await commands.exportWav(outputPath, durationSeconds));
}

// --- VGM Export ---

export async function exportVgm(outputPath: string, durationSeconds: number): Promise<string> {
  return unwrap(await commands.exportVgm(outputPath, durationSeconds));
}

// --- FM File Import ---

export async function importFmFile(filePath: string): Promise<FmFileImportResponse> {
  return unwrap(await commands.importFmFile(filePath));
}
