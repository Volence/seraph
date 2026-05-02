import { invoke } from "@tauri-apps/api/core";
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
} from "../types/model";

export async function playFmTestTone(): Promise<string> {
  return invoke<string>("play_fm_test_tone");
}

export async function playPsgTestTone(): Promise<string> {
  return invoke<string>("play_psg_test_tone");
}

export async function stopAllSound(): Promise<string> {
  return invoke<string>("stop_all_sound");
}

export async function createProject(
  path: string,
  name: string,
  driverId: string,
  tempo: number,
  timeSigNum: number,
  timeSigDen: number,
): Promise<void> {
  return invoke("create_project", { path, name, driverId, tempo, timeSigNum, timeSigDen });
}

export async function openProject(path: string): Promise<Song> {
  return invoke<Song>("open_project", { path });
}

export async function saveProject(): Promise<void> {
  return invoke("save_project");
}

export async function closeProject(): Promise<void> {
  return invoke("close_project");
}

export async function getProjectInfo(): Promise<SongMetadata | null> {
  return invoke<SongMetadata | null>("get_project_info");
}

export async function listDrivers(): Promise<DriverInfo[]> {
  return invoke<DriverInfo[]>("list_drivers");
}

export async function getDriverInfo(driverId: string): Promise<DriverDetail> {
  return invoke<DriverDetail>("get_driver_info", { driverId });
}

export async function addFmInstrument(instrument: FmInstrument): Promise<string> {
  return invoke<string>("add_fm_instrument", { instrument });
}

export async function updateFmInstrument(id: string, instrument: FmInstrument): Promise<void> {
  return invoke("update_fm_instrument", { id, instrument });
}

export async function deleteFmInstrument(id: string): Promise<void> {
  return invoke("delete_fm_instrument", { id });
}

export async function listFmInstruments(): Promise<FmInstrument[]> {
  return invoke<FmInstrument[]>("list_fm_instruments");
}

export async function previewFmInstrument(id: string, midiNote: number): Promise<void> {
  return invoke("preview_fm_instrument", { id, midiNote });
}

export async function addPsgInstrument(instrument: PsgInstrument): Promise<string> {
  return invoke<string>("add_psg_instrument", { instrument });
}

export async function updatePsgInstrument(id: string, instrument: PsgInstrument): Promise<void> {
  return invoke("update_psg_instrument", { id, instrument });
}

export async function deletePsgInstrument(id: string): Promise<void> {
  return invoke("delete_psg_instrument", { id });
}

export async function listPsgInstruments(): Promise<PsgInstrument[]> {
  return invoke<PsgInstrument[]>("list_psg_instruments");
}

export async function previewPsgInstrument(id: string, midiNote: number): Promise<void> {
  return invoke("preview_psg_instrument", { id, midiNote });
}

export async function importDacWav(wavPath: string, targetRate: number): Promise<string> {
  return invoke<string>("import_dac_wav", { wavPath, targetRate });
}

export async function importDacRaw(pcmPath: string, sampleRate: number): Promise<string> {
  return invoke<string>("import_dac_raw", { pcmPath, sampleRate });
}

export async function updateDacInstrument(id: string, instrument: DacInstrument): Promise<void> {
  return invoke("update_dac_instrument", { id, instrument });
}

export async function reconvertDac(id: string, newRate: number): Promise<void> {
  return invoke("reconvert_dac", { id, newRate });
}

export async function deleteDacInstrument(id: string): Promise<void> {
  return invoke("delete_dac_instrument", { id });
}

export async function listDacInstruments(): Promise<DacInstrument[]> {
  return invoke<DacInstrument[]>("list_dac_instruments");
}

export async function previewDac(id: string): Promise<void> {
  return invoke("preview_dac", { id });
}

export async function getDacPcmData(id: string): Promise<number[]> {
  return invoke<number[]>("get_dac_pcm_data", { id });
}

// --- Track CRUD ---

export async function addTrack(
  name: string,
  channel: ChannelAssignment,
  instrumentId: string | null,
): Promise<string> {
  return invoke<string>("add_track", { name, channel, instrumentId });
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
): Promise<void> {
  return invoke("update_track", { id, name, channel, instrumentId, muted, solo, volume, pan });
}

export async function deleteTrack(id: string): Promise<void> {
  return invoke("delete_track", { id });
}

export async function listTracks(): Promise<Track[]> {
  return invoke<Track[]>("list_tracks");
}

// --- Region CRUD ---

export async function addRegion(
  trackId: string,
  startTick: number,
  durationTicks: number,
): Promise<string> {
  return invoke<string>("add_region", { trackId, startTick, durationTicks });
}

export async function updateRegion(
  trackId: string,
  regionId: string,
  startTick: number,
  durationTicks: number,
): Promise<void> {
  return invoke("update_region", { trackId, regionId, startTick, durationTicks });
}

export async function deleteRegion(trackId: string, regionId: string): Promise<void> {
  return invoke("delete_region", { trackId, regionId });
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
  return invoke<number>("add_note", { trackId, regionId, tick, pitch, velocity, durationTicks });
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
  return invoke("update_note", { trackId, regionId, noteIndex, tick, pitch, velocity, durationTicks });
}

export async function deleteNote(
  trackId: string,
  regionId: string,
  noteIndex: number,
): Promise<void> {
  return invoke("delete_note", { trackId, regionId, noteIndex });
}

// --- Transport ---

export async function transportPlay(): Promise<void> {
  return invoke("transport_play");
}

export async function transportStop(): Promise<void> {
  return invoke("transport_stop");
}

export async function transportSeek(tick: number): Promise<void> {
  return invoke("transport_seek", { tick });
}

export async function transportSetLoop(startTick: number, endTick: number): Promise<void> {
  return invoke("transport_set_loop", { startTick, endTick });
}

export async function transportClearLoop(): Promise<void> {
  return invoke("transport_clear_loop");
}

export async function getPlaybackState(): Promise<PlaybackState> {
  return invoke<PlaybackState>("get_playback_state");
}

// --- Validation ---

export async function getChannelOverlaps(): Promise<OverlapWarning[]> {
  return invoke<OverlapWarning[]>("get_channel_overlaps");
}
