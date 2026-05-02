import styles from "./TimelineRuler.module.css";

interface TimelineRulerProps {
  ticksPerPixel: number;
  scrollLeft: number;
  ticksPerBeat: number;
  beatsPerBar: number;
  onSeek: (tick: number) => void;
}

export function TimelineRuler(_props: TimelineRulerProps) {
  return <canvas className={styles.ruler} />;
}
