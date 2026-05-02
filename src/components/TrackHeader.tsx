import type { Track } from "../types/model";
import * as ipc from "../api/ipc";
import styles from "./TrackHeader.module.css";

interface TrackHeaderProps {
  track: Track;
  onUpdate: () => void;
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

export function TrackHeader({ track, onUpdate }: TrackHeaderProps) {
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

  return (
    <div className={styles.header}>
      <div className={styles.top}>
        <span className={styles.badge} style={{ background: channelColor(track) }}>
          {channelLabel(track)}
        </span>
        <span className={styles.name}>{track.name}</span>
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
      </div>
    </div>
  );
}
