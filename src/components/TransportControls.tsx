import type { SongMetadata } from "../types/model";
import { usePlaybackPosition } from "../hooks/usePlaybackPosition";
import * as ipc from "../api/ipc";
import { recordPlayStart, recordStop } from "../utils/transportMemory";
import styles from "./TransportControls.module.css";

interface TransportControlsProps {
  projectMeta: SongMetadata;
  playing: boolean;
  loopEnabled: boolean;
  onPlayingChange: (playing: boolean) => void;
  /** Toggle the preview loop using the last-set range; App owns the range
   *  state and the transport set/clear calls. */
  onToggleLoop: () => void;
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
  onToggleLoop,
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
      // Feed the Space double-tap window (G37) from the button too.
      recordStop();
      onPlayingChange(false);
    } else {
      // Feed the launch-point memory so a stop double-tap can return there
      // (G37); transportMemory decides whether this play records (a plain
      // resume does not). Best-effort, mirrors App.startPlayback.
      try {
        const s = await ipc.getPlaybackState();
        recordPlayStart(s.tick);
      } catch {
        // keep the previous play-start tick
      }
      await ipc.transportPlay();
      onPlayingChange(true);
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
        onClick={onToggleLoop}
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
