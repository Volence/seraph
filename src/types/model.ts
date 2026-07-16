// The data types below are generated from Rust by tauri-specta (see
// `src/bindings.ts`). Rust is the single source of truth; these re-exports keep
// the historical `../types/model` import path working for the rest of the app.
export type {
  FmOperator,
  InstrumentMetadata,
  FmInstrument,
  PsgInstrument,
  NoiseMode,
  DacInstrument,
  InstrumentBank,
  SongMetadata,
  ChannelAssignment,
  Pan,
  Track,
  Region,
  NoteModulation,
  Note,
  Song,
  FmChannelInfo,
  PsgChannelInfo,
  DacChannelInfo,
  ChannelLayout,
  DriverInfo,
  DriverDetail,
  PlaybackState,
  OverlapWarning,
} from "../bindings";

// The DEFAULT_* constants below still need to satisfy the generated types.
import type { FmOperator, InstrumentMetadata } from "../bindings";

export type SelectedInstrument =
  | { type: "fm"; id: string }
  | { type: "psg"; id: string }
  | { type: "dac"; id: string };

export const DEFAULT_FM_MODULATOR: FmOperator = {
  detune: 0,
  multiple: 1,
  rateScale: 0,
  attackRate: 31,
  ampMod: false,
  d1r: 0,
  d2r: 0,
  sustainLevel: 0,
  releaseRate: 15,
  totalLevel: 127,
};

export const DEFAULT_FM_CARRIER: FmOperator = {
  detune: 0,
  multiple: 1,
  rateScale: 0,
  attackRate: 31,
  ampMod: false,
  d1r: 0,
  d2r: 0,
  sustainLevel: 0,
  releaseRate: 7,
  totalLevel: 20,
};

export const DEFAULT_METADATA: InstrumentMetadata = {
  category: "",
  author: "",
  tags: [],
};

export const CARRIER_MASKS = [
  0b1000, 0b1000, 0b1000, 0b1000,
  0b1010, 0b1110, 0b1110, 0b1111,
];

export function isCarrier(algorithm: number, opIndex: number): boolean {
  return (CARRIER_MASKS[algorithm] & (1 << opIndex)) !== 0;
}

export interface SelectedRegion {
  trackId: string;
  trackName: string;
  regionId: string;
  channelType: "fm" | "psg" | "dac";
  startTick: number;
  durationTicks: number;
}
