import type { SongMetadata } from "../types/model";
import { usePlaybackPosition } from "../hooks/usePlaybackPosition";
import * as ipc from "../api/ipc";
import styles from "./TransportControls.module.css";

interface TransportControlsProps {
  projectMeta: SongMetadata;
  playing: boolean;
  loopEnabled: boolean;
  onPlayingChange: (playing: boolean) => void;
  onLoopChange: (enabled: boolean) => void;
  /** Seek request; App owns the seek cursor + transport call (G29). */
  onSeek: (tick: number) => void;
}

function tickToBarBeatTick(
  tick: number,
  ticksPerBeat: number,
  beatsPerBar: number,
): string {
  const totalBeats = Math.floor(tick / ticksPerBeat);
  const bar = Math.floor(totalBeats / beatsPerBar) + 1;
  const beat = (totalBeats % beatsPerBar) + 1;
  const subTick = Math.floor(tick % ticksPerBeat);
  return `${bar}:${beat}:${String(subTick).padStart(3, "0")}`;
}

export function TransportControls({
  projectMeta,
  playing,
  loopEnabled,
  onPlayingChange,
  onLoopChange,
  onSeek,
}: TransportControlsProps) {
  const { currentTick } = usePlaybackPosition(
    playing,
    projectMeta.tempo,
    projectMeta.ticksPerBeat,
  );

  async function handlePlayStop() {
    if (playing) {
      await ipc.transportStop();
      onPlayingChange(false);
    } else {
      await ipc.transportPlay();
      onPlayingChange(true);
    }
  }

  async function handleLoop() {
    if (loopEnabled) {
      await ipc.transportClearLoop();
      onLoopChange(false);
    } else {
      const ticksPerBar = projectMeta.ticksPerBeat * projectMeta.timeSignature[0];
      await ipc.transportSetLoop(0, ticksPerBar * 4);
      onLoopChange(true);
    }
  }

  function handleHome() {
    // Routes through App so the seek cursor moves with the transport (G29).
    onSeek(0);
  }

  const position = tickToBarBeatTick(
    currentTick,
    projectMeta.ticksPerBeat,
    projectMeta.timeSignature[0],
  );

  return (
    <div className={styles.transport}>
      <button
        className={`${styles.btn} ${playing ? styles.active : ""}`}
        onClick={handlePlayStop}
        title={playing ? "Stop (Space)" : "Play (Space)"}
      >
        {playing ? "■" : "▶"}
      </button>
      <button
        className={`${styles.btn} ${loopEnabled ? styles.active : ""}`}
        onClick={handleLoop}
        title="Loop (L)"
      >
        {"↻"}
      </button>
      <button className={styles.btn} onClick={handleHome} title="Home">
        {"⏮"}
      </button>
      <span className={styles.position}>{position}</span>
    </div>
  );
}
