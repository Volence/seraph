import type { Track, SelectedRegion } from "../types/model";
import styles from "./TimelineCanvas.module.css";

interface TimelineCanvasProps {
  tracks: Track[];
  ticksPerPixel: number;
  scrollLeft: number;
  trackHeight: number;
  playbackTick: number;
  playing: boolean;
  selectedRegion: SelectedRegion | null;
  onRegionClick: (trackId: string, regionId: string) => void;
  onRegionDoubleClick: (trackId: string, regionId: string) => void;
  onEmptyDoubleClick: (trackId: string, startTick: number) => void;
}

export function TimelineCanvas(_props: TimelineCanvasProps) {
  return <canvas className={styles.canvas} />;
}
