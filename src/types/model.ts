export interface FmOperator {
  detune: number;
  multiple: number;
  rateScale: number;
  attackRate: number;
  ampMod: boolean;
  d1r: number;
  d2r: number;
  sustainLevel: number;
  releaseRate: number;
  totalLevel: number;
}

export interface InstrumentMetadata {
  category: string;
  author: string;
  tags: string[];
}

export interface FmInstrument {
  id: string;
  name: string;
  algorithm: number;
  feedback: number;
  operators: [FmOperator, FmOperator, FmOperator, FmOperator];
  metadata: InstrumentMetadata;
}

export interface PsgInstrument {
  id: string;
  name: string;
  volumeSequence: number[];
  loopPoint: number | null;
  noiseMode: NoiseMode | null;
  metadata: InstrumentMetadata;
}

export type NoiseMode = { Periodic: number } | { White: number };

export interface DacInstrument {
  id: string;
  name: string;
  targetSampleRate: number;
  loopStart: number | null;
  loopLength: number | null;
  originalFile: string;
  pcmFile: string;
  sourceIsRaw: boolean;
  metadata: InstrumentMetadata;
}

export interface InstrumentBank {
  fm: FmInstrument[];
  psg: PsgInstrument[];
  dac: DacInstrument[];
}

export interface SongMetadata {
  name: string;
  tempo: number;
  timeSignature: [number, number];
  ticksPerBeat: number;
  driverId: string;
}

export type ChannelAssignment =
  | { Fm: number }
  | { Psg: number }
  | "PsgNoise"
  | { Dac: number };

export type Pan = "Left" | "Right" | "Center";

export interface Track {
  id: string;
  name: string;
  channel: ChannelAssignment;
  instrumentId: string | null;
  regions: Region[];
  muted: boolean;
  solo: boolean;
  volume: number;
  pan: Pan;
}

export interface Region {
  id: string;
  startTick: number;
  durationTicks: number;
  notes: Note[];
}

export interface Note {
  tick: number;
  pitch: number;
  velocity: number;
  durationTicks: number;
}

export interface Song {
  metadata: SongMetadata;
  tracks: Track[];
  instruments: InstrumentBank;
}

export interface FmChannelInfo {
  index: number;
  name: string;
  supportsSpecialMode: boolean;
}

export interface PsgChannelInfo {
  index: number;
  name: string;
  isNoise: boolean;
}

export interface DacChannelInfo {
  index: number;
  name: string;
}

export interface ChannelLayout {
  fmChannels: FmChannelInfo[];
  psgChannels: PsgChannelInfo[];
  dacChannels: DacChannelInfo[];
}

export interface DriverInfo {
  id: string;
  name: string;
}

export interface DriverDetail {
  id: string;
  name: string;
  layout: ChannelLayout;
  features: string[];
}

export type SelectedInstrument =
  | { type: "fm"; id: string }
  | { type: "psg"; id: string }
  | { type: "dac"; id: string };

export const DEFAULT_FM_OPERATOR: FmOperator = {
  detune: 0,
  multiple: 0,
  rateScale: 0,
  attackRate: 0,
  ampMod: false,
  d1r: 0,
  d2r: 0,
  sustainLevel: 0,
  releaseRate: 0,
  totalLevel: 127,
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

export interface PlaybackState {
  playing: boolean;
  tick: number;
  loopStart: number | null;
  loopEnd: number | null;
}

export interface OverlapWarning {
  channelName: string;
  tickStart: number;
  tickEnd: number;
  trackIds: string[];
}

export interface SelectedRegion {
  trackId: string;
  trackName: string;
  regionId: string;
  channelType: "fm" | "psg" | "dac";
  startTick: number;
  durationTicks: number;
}
