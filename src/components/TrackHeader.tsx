import { useState } from "react";
import type { Track, FmInstrument, PsgInstrument, DacInstrument } from "../types/model";
import * as ipc from "../api/ipc";
import styles from "./TrackHeader.module.css";

interface TrackHeaderProps {
  track: Track;
  fmInstruments: FmInstrument[];
  psgInstruments: PsgInstrument[];
  dacInstruments: DacInstrument[];
  onUpdate: () => void;
  onDelete: () => void;
}

function channelColor(track: Track): string {
  const ch = track.channel;
  if (ch === "PsgNoise") return "var(--accent-psg)";
  if (typeof ch === "object" && "Fm" in ch) return "var(--accent-fm)";
  if (typeof ch === "object" && "Psg" in ch) return "var(--accent-psg)";
  return "var(--accent-dac)";
}

function channelLabel(track: Track): string {
  const ch = track.channel;
  if (ch === "PsgNoise") return "Noise";
  if (typeof ch === "object" && "Fm" in ch) return `FM${ch.Fm + 1}`;
  if (typeof ch === "object" && "Psg" in ch) return `PSG${ch.Psg + 1}`;
  if (typeof ch === "object" && "Dac" in ch) return "DAC";
  return "?";
}

function channelType(track: Track): "fm" | "psg" | "dac" {
  const ch = track.channel;
  if (ch === "PsgNoise") return "psg";
  if (typeof ch === "object" && "Fm" in ch) return "fm";
  if (typeof ch === "object" && "Psg" in ch) return "psg";
  return "dac";
}

export function TrackHeader({
  track,
  fmInstruments,
  psgInstruments,
  dacInstruments,
  onUpdate,
  onDelete,
}: TrackHeaderProps) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(track.name);

  const ct = channelType(track);
  const instruments =
    ct === "fm" ? fmInstruments :
    ct === "psg" ? psgInstruments :
    dacInstruments;

  async function toggleMute() {
    await ipc.updateTrack(
      track.id, track.name, track.channel, track.instrumentId,
      !track.muted, track.solo, track.volume, track.pan,
    );
    onUpdate();
  }

  async function toggleSolo() {
    await ipc.updateTrack(
      track.id, track.name, track.channel, track.instrumentId,
      track.muted, !track.solo, track.volume, track.pan,
    );
    onUpdate();
  }

  async function commitRename() {
    setEditing(false);
    if (name.trim() && name !== track.name) {
      await ipc.updateTrack(
        track.id, name.trim(), track.channel, track.instrumentId,
        track.muted, track.solo, track.volume, track.pan,
      );
      onUpdate();
    }
  }

  async function changeInstrument(instId: string) {
    await ipc.updateTrack(
      track.id, track.name, track.channel, instId || null,
      track.muted, track.solo, track.volume, track.pan,
    );
    onUpdate();
  }

  return (
    <div className={styles.header} onContextMenu={(e) => { e.preventDefault(); onDelete(); }}>
      <div className={styles.top}>
        <span className={styles.badge} style={{ background: channelColor(track) }}>
          {channelLabel(track)}
        </span>
        {editing ? (
          <input
            className={styles.nameInput}
            value={name}
            onChange={(e) => setName(e.target.value)}
            onBlur={commitRename}
            onKeyDown={(e) => e.key === "Enter" && commitRename()}
            autoFocus
          />
        ) : (
          <span className={styles.name} onDoubleClick={() => setEditing(true)}>
            {track.name}
          </span>
        )}
      </div>
      <div className={styles.controls}>
        <button
          className={`${styles.muteBtn} ${track.muted ? styles.active : ""}`}
          onClick={toggleMute}
        >
          M
        </button>
        <button
          className={`${styles.soloBtn} ${track.solo ? styles.active : ""}`}
          onClick={toggleSolo}
        >
          S
        </button>
        <select
          className={styles.instSelect}
          value={track.instrumentId ?? ""}
          onChange={(e) => changeInstrument(e.target.value)}
        >
          <option value="">-- None --</option>
          {instruments.map((inst) => (
            <option key={inst.id} value={inst.id}>{inst.name}</option>
          ))}
        </select>
      </div>
    </div>
  );
}
