import type { Track } from "../types/model";
import * as ipc from "../api/ipc";
import styles from "./TrackHeader.module.css";

interface TrackHeaderProps {
  track: Track;
  selected: boolean;
  level: number;
  onUpdate: () => void;
  onClick: () => void;
  isGroupHead?: boolean;
  groupCollapsed?: boolean;
  groupSize?: number;
  onToggleCollapse?: () => void;
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

function levelColor(pct: number): string {
  if (pct >= 1.0) return "#f44";
  if (pct > 0.85) return "#fa0";
  if (pct > 0.7) return "#ff0";
  return "#4f4";
}

export function channelLevelIndex(track: Track): number {
  const ch = track.channel;
  if (typeof ch === "object" && "Fm" in ch) return ch.Fm;
  if (typeof ch === "object" && "Psg" in ch) return 6 + ch.Psg;
  if (ch === "PsgNoise") return 9;
  if (typeof ch === "object" && "Dac" in ch) return 10;
  return -1;
}

export function TrackHeader({ track, selected, level, onUpdate, onClick, isGroupHead, groupCollapsed, groupSize, onToggleCollapse }: TrackHeaderProps) {
  async function toggleMute(e: React.MouseEvent) {
    e.stopPropagation();
    await ipc.updateTrack(
      track.id, track.name, track.channel, track.instrumentId,
      !track.muted, track.solo, track.volume, track.pan, track.pitchOffset,
    );
    onUpdate();
    ipc.reloadSequence();
  }

  async function toggleSolo(e: React.MouseEvent) {
    e.stopPropagation();
    await ipc.updateTrack(
      track.id, track.name, track.channel, track.instrumentId,
      track.muted, !track.solo, track.volume, track.pan, track.pitchOffset,
    );
    onUpdate();
    ipc.reloadSequence();
  }

  async function handleVolume(e: React.ChangeEvent<HTMLInputElement>) {
    e.stopPropagation();
    const vol = parseInt(e.target.value);
    await ipc.updateTrack(
      track.id, track.name, track.channel, track.instrumentId,
      track.muted, track.solo, vol, track.pan, track.pitchOffset,
    );
    onUpdate();
    ipc.reloadSequence();
  }

  async function handlePitchOffset(delta: number, e: React.MouseEvent) {
    e.stopPropagation();
    const newOffset = Math.max(-48, Math.min(48, (track.pitchOffset ?? 0) + delta));
    await ipc.updateTrack(
      track.id, track.name, track.channel, track.instrumentId,
      track.muted, track.solo, track.volume, track.pan, newOffset,
    );
    onUpdate();
    ipc.reloadSequence();
  }

  const pct = Math.min(level / 127, 1);

  return (
    <div
      className={`${styles.header} ${selected ? styles.selected : ""}`}
      onClick={onClick}
    >
      <div className={styles.top}>
        {isGroupHead && (
          <button className={styles.collapseBtn} onClick={(e) => { e.stopPropagation(); onToggleCollapse?.(); }}>
            {groupCollapsed ? '▶' : '▼'}
          </button>
        )}
        <span className={styles.badge} style={{ background: channelColor(track) }}>
          {channelLabel(track)}
        </span>
        <span className={styles.name}>{track.name}</span>
        {isGroupHead && groupCollapsed && groupSize && groupSize > 1 && (
          <span className={styles.hiddenCount}>+{groupSize - 1}</span>
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
        <input
          type="range"
          className={styles.volumeSlider}
          min={0}
          max={127}
          value={track.volume}
          onChange={handleVolume}
          onClick={(e) => e.stopPropagation()}
          title={`Vol: ${track.volume}`}
        />
        <div className={styles.pitchGroup}>
          <button className={styles.pitchBtn} onClick={(e) => handlePitchOffset(-1, e)} title="Pitch down 1 semitone">-</button>
          <span className={styles.pitchLabel} title="Pitch offset (semitones)">{(track.pitchOffset ?? 0) >= 0 ? "+" : ""}{track.pitchOffset ?? 0}</span>
          <button className={styles.pitchBtn} onClick={(e) => handlePitchOffset(1, e)} title="Pitch up 1 semitone">+</button>
        </div>
        <div className={styles.meter}>
          <div
            className={styles.meterFill}
            style={{
              width: `${pct * 100}%`,
              background: levelColor(pct),
            }}
          />
        </div>
      </div>
    </div>
  );
}
