import { invoke } from "@tauri-apps/api/core";
import type {
  FmInstrument,
  PsgInstrument,
  DacInstrument,
  Song,
  SongMetadata,
  DriverInfo,
  DriverDetail,
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
